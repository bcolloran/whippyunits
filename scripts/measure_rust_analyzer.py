#!/usr/bin/env python3
"""Measure direct rust-analyzer LSP requests against labeled dependency profiles.

Example (use immutable source snapshots for reproducible comparisons):
  python3 scripts/measure_rust_analyzer.py --server /path/to/rust-analyzer \
    --profile baseline /tmp/baseline std,units-dynamics \
    --profile coherent /tmp/candidate std,units-si-dynamics \
    --output /tmp/whippyunits-ra-results --runs 3

Each profile becomes a standalone Cargo consumer with default features disabled.
Use --project-profile for an existing locked probe; its lockfile and source
imports are preserved. --import-style rewrites imports only in generated probes;
for existing probes, select the style already present (autoimport adds validation).
Cargo dependencies are warmed outside the timing window; each sample starts a
fresh server, waits for indexing/cache priming, then measures first/repeated
hover and completion. Every timed completion must offer a usable `meters()`
edit. After timing, ten coherent constructor methods are validated by default,
including `newtons()`. Missing required completions fail the run.
--allow-missing-completion LABEL permits a known-broken baseline to retain its
indexing/hover measurements, but explicitly excludes its completion latencies.
This measures the server directly, without VS Code or the formatter extension.
Use --prepare-only to generate inputs without executing Cargo or rust-analyzer.
"""

import argparse
import hashlib
import json
import os
from pathlib import Path
import queue
import re
import signal
import statistics
import subprocess
import threading
import time


SOURCE = '''use whippyunits::quantity;
__IMPORT__
fn main() {
    let distance = quantity!(3.0, m);
    let duration = quantity!(2.0, s);
    let velocity = distance / duration; // benchmark-hover
    let force = quantity!(2.0, kg) * quantity!(3.0, m / s ^ 2);
    let energy = force * distance;
    let scalar: f64 = 1.0;
    let probe = whippyunits::default_declarators::MetricLength::meters(scalar); // benchmark-completion
    let _ = (velocity, energy, probe);
}
'''
IMPORTS = {
    "glob": "use whippyunits::default_declarators::*;",
    "explicit": "use whippyunits::default_declarators::MetricLength;",
    "autoimport": "// Deliberately leave the constructor trait out of scope.",
}


def write_json(path, data):
    temporary = path.with_suffix(path.suffix + ".tmp")
    temporary.write_text(json.dumps(data, indent=2) + "\n")
    temporary.replace(path)


def position(source, marker, token, end=False):
    matching = [(n, line) for n, line in enumerate(source.splitlines()) if marker in line]
    if len(matching) != 1:
        raise ValueError(f"expected one source marker: {marker}")
    number, line = matching[0]
    offset = line.index(token) + (len(token) if end else min(2, len(token) - 1))
    return {"line": number, "character": len(line[:offset].encode("utf-16-le")) // 2}


def offset(source, point):
    lines = source.splitlines(keepends=True)
    line = lines[point["line"]]
    prefix = line.encode("utf-16-le")[:point["character"] * 2].decode("utf-16-le")
    return sum(map(len, lines[:point["line"]])) + len(prefix)


def usable_completion(items, source, cursor, receiver="scalar", method="meters"):
    """Check semantic name AND the edit, instead of timing native-only results."""
    for item in items:
        name = item.get("filterText", item.get("label", "").split("(", 1)[0])
        if name != method or item.get("kind") != 2:
            continue
        edit = item.get("textEdit")
        if not edit:
            continue
        edit_range = edit.get("range", edit.get("replace"))
        if not edit_range:
            continue
        start = offset(source, edit_range["start"])
        stop = offset(source, edit_range["end"])
        if not start <= offset(source, cursor) <= stop:
            continue
        inserted = edit["newText"].replace("$0", "")
        inserted = re.sub(r"\$\{\d+:([^}]*)\}", r"\1", inserted)
        replaced = source[:start] + inserted + source[stop:]
        if receiver + "." + method + "()" in replaced:
            return item
    return None


class LspClient:
    def __init__(self, server, project, environment, log, settings):
        self.started = time.perf_counter()
        self.process = subprocess.Popen(
            [str(server)], cwd=project, env=environment, stdin=subprocess.PIPE,
            stdout=subprocess.PIPE, stderr=log, start_new_session=(os.name == "posix"),
        )
        self.settings = settings
        self.messages = queue.Queue()
        self.events = []
        self.responses = {}
        self.sequence = 0
        self.last_status = None
        self.saw_busy = False
        self.ready_seconds = None
        self.rss_peak_kib = None
        self.memory_stop = threading.Event()
        threading.Thread(target=self.read_messages, daemon=True).start()
        threading.Thread(target=self.sample_memory, daemon=True).start()

    def rss_kib(self):
        """Linux server RSS only; excludes Cargo and proc-macro children."""
        try:
            status = Path(f"/proc/{self.process.pid}/status").read_text()
            match = re.search(r"^VmRSS:\s+(\d+) kB$", status, re.M)
            return int(match[1]) if match else None
        except (OSError, ValueError):
            return None

    def sample_memory(self):
        while not self.memory_stop.is_set():
            rss = self.rss_kib()
            if rss is not None:
                self.rss_peak_kib = max(self.rss_peak_kib or 0, rss)
            self.memory_stop.wait(0.05)

    def read_messages(self):
        try:
            while True:
                headers = {}
                while True:
                    line = self.process.stdout.readline()
                    if not line:
                        self.messages.put({"_closed": True})
                        return
                    if line in (b"\r\n", b"\n"):
                        break
                    key, value = line.decode("ascii").split(":", 1)
                    headers[key.lower()] = value.strip()
                self.messages.put(json.loads(self.process.stdout.read(int(headers["content-length"]))))
        except Exception as error:
            self.messages.put({"_reader_error": repr(error)})

    def send(self, payload):
        data = json.dumps(payload).encode("utf-8")
        self.process.stdin.write(f"Content-Length: {len(data)}\r\n\r\n".encode("ascii") + data)
        self.process.stdin.flush()

    def notify(self, method, params):
        self.send({"jsonrpc": "2.0", "method": method, "params": params})

    def configuration(self, section):
        if section in (None, "", "rust-analyzer"):
            return self.settings
        section = section.removeprefix("rust-analyzer.")
        value = self.settings
        for part in section.split("."):
            if not isinstance(value, dict):
                return None
            value = value.get(part)
        return value

    def receive(self, timeout):
        item = self.messages.get(timeout=max(0.001, timeout))
        if "_closed" in item or "_reader_error" in item:
            raise RuntimeError(f"rust-analyzer transport stopped: {item}")
        elapsed = time.perf_counter() - self.started
        if "method" in item:
            self.events.append({"seconds": elapsed, **item})
            if "id" in item:
                method = item["method"]
                if method == "workspace/configuration":
                    result = [self.configuration(x.get("section")) for x in item["params"]["items"]]
                elif method == "workspace/workspaceFolders":
                    result = self.workspace_folders
                elif method in ("window/workDoneProgress/create", "client/registerCapability", "client/unregisterCapability"):
                    result = None
                else:
                    self.send({"jsonrpc": "2.0", "id": item["id"], "error": {"code": -32601, "message": "Unsupported benchmark-client request"}})
                    return
                self.send({"jsonrpc": "2.0", "id": item["id"], "result": result})
            elif item["method"] == "experimental/serverStatus":
                self.last_status = item["params"]
                self.saw_busy |= not self.last_status.get("quiescent", False)
                if self.saw_busy and self.last_status.get("quiescent", False) and self.ready_seconds is None:
                    self.ready_seconds = elapsed
        elif "id" in item:
            self.responses[item["id"]] = item

    def request(self, method, params, timeout=90):
        self.sequence += 1
        identifier = self.sequence
        self.send({"jsonrpc": "2.0", "id": identifier, "method": method, "params": params})
        deadline = time.perf_counter() + timeout
        while identifier not in self.responses:
            remaining = deadline - time.perf_counter()
            if remaining <= 0:
                raise TimeoutError(method)
            try:
                self.receive(remaining)
            except queue.Empty:
                raise TimeoutError(method) from None
        return self.responses.pop(identifier)

    def wait_ready(self, timeout):
        deadline = time.perf_counter() + timeout
        while self.ready_seconds is None:
            remaining = deadline - time.perf_counter()
            if remaining <= 0:
                raise TimeoutError("indexing/cache priming")
            try:
                self.receive(remaining)
            except queue.Empty:
                raise TimeoutError("indexing/cache priming") from None
        if self.last_status.get("health") == "error":
            raise RuntimeError(f"server health error: {self.last_status}")
        return self.ready_seconds

    def close(self):
        self.memory_stop.set()
        try:
            self.request("shutdown", None, timeout=5)
            self.notify("exit", None)
            self.process.wait(timeout=5)
        except (Exception, KeyboardInterrupt):
            pass
        finally:
            # Only this client's process group; avoid orphan proc-macro/Cargo
            # workers contaminating later timing windows after a failed request.
            if os.name == "posix":
                try:
                    os.killpg(self.process.pid, signal.SIGTERM)
                except ProcessLookupError:
                    pass
            elif self.process.poll() is None:
                self.process.terminate()
            try:
                self.process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                if os.name == "posix":
                    try:
                        os.killpg(self.process.pid, signal.SIGKILL)
                    except ProcessLookupError:
                        pass
                else:
                    self.process.kill()
                self.process.wait()


def collect_failure_diagnostics(args, profile, client, uri, source, stem):
    client.notify("textDocument/didChange", {"textDocument": {"uri": uri, "version": 1000},
        "contentChanges": [{"text": source}]})
    for name, method, params in [
        ("trait-hover", "textDocument/hover", {"textDocument": {"uri": uri},
            "position": position(source, "benchmark-completion", "MetricLength")}),
        ("method-hover", "textDocument/hover", {"textDocument": {"uri": uri},
            "position": position(source, "benchmark-completion", "meters")}),
        ("status", "rust-analyzer/analyzerStatus", {"textDocument": {"uri": uri}}),
    ]:
        write_json(args.output / f"{stem}.failure-{name}.json", client.request(method, params, 8))
    if args.expand_on_failure:
        path = Path(profile["dependency_path"]) / "src/default_declarators.rs"
        macro_source = path.read_text()
        location = macro_source.rindex("generate_default_declarators!")
        prefix = macro_source[:location]
        point = {"line": prefix.count("\n"), "character": len(prefix.rsplit("\n", 1)[-1]) + 2}
        result = client.request("rust-analyzer/expandMacro", {
            "textDocument": {"uri": path.as_uri()}, "position": point}, args.request_timeout)
        write_json(args.output / f"{stem}.failure-expanded-declarators.json", result)


def source_fingerprint(root):
    digest = hashlib.sha256()
    files = []
    for directory in [root, root / "proc-macros", root / "whippyunits-core",
                      root / "whippyalgebra", root / "whippyalgebra-macros", root / "vendor/typenum",
                      root / "engine", root / "consumer"]:
        for name in ("Cargo.toml", "Cargo.lock", "rust-toolchain.toml", "build.rs"):
            path = directory / name
            if path.is_file():
                files.append(path)
        files.extend((directory / "src").rglob("*.rs"))
    # The isolated axis fixture keeps shared implementations alongside both
    # workspaces; include those inputs as well as representation.rs above.
    if (root / "engine").is_dir():
        files.extend((root.parent / "common").glob("*.rs"))
        core = root.parent / "source/whippyunits-core"
        files.extend((core / "src").rglob("*.rs"))
        for name in ("Cargo.toml", "build.rs"):
            if (core / name).is_file():
                files.append(core / name)
    for path in sorted(files):
        digest.update(os.path.relpath(path, root).encode())
        digest.update(b"\0")
        digest.update(path.read_bytes())
        digest.update(b"\0")
    return digest.hexdigest()


def make_project(directory, profile, import_style):
    directory.mkdir(parents=True, exist_ok=True)
    (directory / "src").mkdir(exist_ok=True)
    source = SOURCE.replace("__IMPORT__", IMPORTS[import_style])
    (directory / "src/main.rs").write_text(source)
    manifest = '\n'.join([
        '[package]', f'name = "ra_probe_{profile["label"].replace("-", "_")}"',
        'version = "0.0.0"', 'edition = "2024"', '[workspace]', '[dependencies]',
        'whippyunits = { path = ' + json.dumps(profile['dependency_path']) +
        ', default-features = false, features = ' + json.dumps(profile['features']) + ' }',
        '[profile.dev]', 'incremental = false', '',
    ])
    (directory / "Cargo.toml").write_text(manifest)
    return source


def measure(args, profile, cycle, project, source, environment):
    stem = f'{cycle:02d}-{profile["label"]}'
    row = {"profile": profile["label"], "cycle": cycle, "valid": False,
           "completion_status": "pending", "missing_completion_names": [],
           "primary_completion_valid": False, "completion_validation": {},
           "request_ms": {"hover": [], "completion": []}, "raw": {}}
    settings = {
        "checkOnSave": False,
        "cargo": {"allTargets": False, "extraArgs": ["--offline", "--locked"]},
        "procMacro": {"enable": True}, "cachePriming": {"enable": True},
        "completion": {"autoimport": {"enable": True}, "limit": None},
    }
    log_path = args.output / f"{stem}.stderr"
    log = log_path.open("w")
    client = LspClient(args.server, project, environment, log, settings)
    uri = (project / "src/main.rs").as_uri()
    client.workspace_folders = [{"uri": project.as_uri(), "name": profile["label"]}]
    try:
        initialized = client.request("initialize", {
            "processId": os.getpid(), "rootUri": project.as_uri(),
            "workspaceFolders": client.workspace_folders,
            "clientInfo": {"name": "whippyunits-ra-benchmark", "version": "1"},
            "capabilities": {
                "workspace": {"configuration": True, "workspaceFolders": True},
                "window": {"workDoneProgress": True},
                "general": {"positionEncodings": ["utf-16"]},
                "experimental": {"serverStatusNotification": True},
                "textDocument": {
                    "hover": {"contentFormat": ["markdown", "plaintext"]},
                    "completion": {
                        "contextSupport": True,
                        "completionItem": {"snippetSupport": True,
                            "documentationFormat": ["markdown", "plaintext"],
                            "insertReplaceSupport": True,
                            "resolveSupport": {"properties": ["documentation", "detail", "additionalTextEdits"]}},
                    },
                },
            },
            "initializationOptions": settings,
        })
        if "error" in initialized:
            raise RuntimeError(initialized)
        row["server_info"] = initialized["result"].get("serverInfo")
        write_json(args.output / f"{stem}.initialize.json", initialized)
        client.notify("initialized", {})
        client.notify("textDocument/didOpen", {"textDocument": {
            "uri": uri, "languageId": "rust", "version": 1, "text": source,
        }})
        row["ready_seconds"] = client.wait_ready(args.load_timeout)
        row["server_ready_rss_kib"] = client.rss_kib()
        row["server_status"] = client.last_status
        print(json.dumps({"event": "indexed", "profile": profile["label"], "cycle": cycle,
                          "seconds": row["ready_seconds"]}), flush=True)
        for kind in ("hover", "completion"):
            active_source = source
            if kind == "completion":
                receiver = "scalar" if args.receiver == "variable" else "1.0_f64"
                active_source = source.replace(
                    "whippyunits::default_declarators::MetricLength::meters(scalar)", receiver + ".me")
                client.notify("textDocument/didChange", {"textDocument": {"uri": uri, "version": 2},
                    "contentChanges": [{"text": active_source}]})
            cursor = position(active_source, "benchmark-hover", "velocity") if kind == "hover" else position(
                active_source, "benchmark-completion", receiver + ".me", end=True)
            for sample in range(1 + args.repeats):
                parameters = {"textDocument": {"uri": uri}, "position": cursor}
                if kind == "completion":
                    parameters["context"] = {"triggerKind": 1}  # Explicit completion invocation.
                before = time.perf_counter()
                result = client.request("textDocument/" + kind, parameters, args.request_timeout)
                milliseconds = (time.perf_counter() - before) * 1000
                raw_path = args.output / f"{stem}.{kind}-{sample}.json"
                write_json(raw_path, result)
                row["raw"].setdefault(kind, []).append(str(raw_path))
                row["request_ms"][kind].append(milliseconds)
                if "error" in result:
                    raise RuntimeError(f"{kind} request failed: {result['error']}")
                value = result.get("result")
                if kind == "hover":
                    contents = json.dumps(value) if value else ""
                    if not all(token in contents for token in ("Quantity", "_L<1>", "_T<-1>")):
                        raise RuntimeError("inferred velocity hover did not resolve length/time Quantity")
                else:
                    items = value.get("items", []) if isinstance(value, dict) else value or []
                    item = usable_completion(items, active_source, cursor, receiver)
                    row["completion_item_count"] = len(items)
                    row["completion_is_incomplete"] = value.get("isIncomplete") if isinstance(value, dict) else None
                    if item is None:
                        row["completion_labels"] = [x.get("label") for x in items]
                        row["completion_status"] = "missing"
                        row["primary_completion_valid"] = False
                        row["missing_completion_names"].append("meters")
                        row["completion_validation"]["meters"] = {"present": False}
                        row["rejected_completion_ms"] = row["request_ms"]["completion"]
                        row["request_ms"]["completion"] = []
                        if profile["label"] in args.allow_missing_completion:
                            break
                        raise RuntimeError("completion did not offer a usable meters() edit; no valid completion timing")
                    row["completion_status"] = "verified"
                    row["primary_completion_valid"] = True
                    row["completion_validation"]["meters"] = {"present": True, "item": item}
                    row["verified_completion"] = item
                    if args.import_style == "autoimport" and sample == 0:
                        resolved = client.request("completionItem/resolve", item, args.request_timeout)
                        write_json(args.output / f"{stem}.resolved-completion.json", resolved)
                        if "error" in resolved or not resolved.get("result", {}).get("additionalTextEdits"):
                            raise RuntimeError("autoimport completion lacks the required trait-import edit")
            print(json.dumps({"event": "requests", "profile": profile["label"], "cycle": cycle,
                              "kind": kind, "milliseconds": row["request_ms"][kind]}), flush=True)
        # Validate derived methods only after every timed meters() request,
        # so they cannot warm the first-completion measurement. Their timings
        # are diagnostic and are excluded from the latency summary.
        for index, method in enumerate(name for name in args.required_completions if name != "meters"):
            active_source = source.replace(
                "whippyunits::default_declarators::MetricLength::meters(scalar)",
                receiver + "." + method[:2])
            client.notify("textDocument/didChange", {"textDocument": {"uri": uri, "version": 3 + index},
                "contentChanges": [{"text": active_source}]})
            cursor = position(active_source, "benchmark-completion", receiver + "." + method[:2], end=True)
            started = time.perf_counter()
            result = client.request("textDocument/completion", {
                "textDocument": {"uri": uri}, "position": cursor,
                "context": {"triggerKind": 1}}, args.request_timeout)
            elapsed = (time.perf_counter() - started) * 1000
            write_json(args.output / f"{stem}.validation-{method}.json", result)
            if "error" in result:
                raise RuntimeError(f"{method} validation request failed: {result['error']}")
            value = result.get("result")
            items = value.get("items", []) if isinstance(value, dict) else value or []
            item = usable_completion(items, active_source, cursor, receiver, method)
            row["completion_validation"][method] = {"present": item is not None,
                "diagnostic_ms": elapsed, "item": item}
            if item is None:
                row["missing_completion_names"].append(method)
        if row["missing_completion_names"]:
            row["completion_status"] = "partial" if row["primary_completion_valid"] else "missing"
            if profile["label"] not in args.allow_missing_completion:
                raise RuntimeError(f"required unit completions missing: {row['missing_completion_names']}")
        row["valid"] = True
    except (Exception, KeyboardInterrupt) as error:
        row["error"] = repr(error)
    finally:
        # Diagnostics never seed a successful first request or contribute to
        # timing samples, including for an explicitly permitted broken baseline.
        if "error" in row or row["missing_completion_names"]:
            try:
                collect_failure_diagnostics(args, profile, client, uri, source, stem)
            except Exception as diagnostics_error:
                row["diagnostics_error"] = repr(diagnostics_error)
        row["server_final_rss_kib"] = client.rss_kib()
        observed_rss = [value for value in (client.rss_peak_kib, row["server_final_rss_kib"],
                        row.get("server_ready_rss_kib")) if value is not None]
        row["server_peak_rss_kib"] = max(observed_rss) if observed_rss else None
        client.close()
        write_json(args.output / f"{stem}.events.json", client.events)
        log.close()
        write_json(args.output / f"{stem}.result.json", row)
    return row


def summarize(rows):
    summary = {}
    for label in sorted({row["profile"] for row in rows}):
        valid = [row for row in rows if row["profile"] == label and row["valid"]]
        if not valid:
            continue
        result = {"valid_runs": len(valid),
                  "ready_seconds_median": statistics.median(row["ready_seconds"] for row in valid)}
        result["completion_missing_runs"] = sum(bool(row["missing_completion_names"]) for row in valid)
        for key in ("server_ready_rss_kib", "server_final_rss_kib", "server_peak_rss_kib"):
            available = [row[key] for row in valid if row.get(key) is not None]
            result[key + "_median"] = statistics.median(available) if available else None
        for kind in ("hover", "completion"):
            available = [row for row in valid if row["request_ms"][kind]
                         and (kind != "completion" or row["primary_completion_valid"])]
            result[kind + "_valid_runs"] = len(available)
            result[kind + "_first_ms_median"] = statistics.median(
                row["request_ms"][kind][0] for row in available) if available else None
            repeated = [sample for row in available for sample in row["request_ms"][kind][1:]]
            result[kind + "_repeated_ms_median"] = statistics.median(repeated) if repeated else None
        summary[label] = result
    return summary


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--server", type=Path, required=True)
    parser.add_argument("--profile", nargs=3, action="append", default=[], metavar=("LABEL", "DEPENDENCY_PATH", "FEATURES"))
    parser.add_argument("--project-profile", nargs=2, action="append", default=[], metavar=("LABEL", "PROJECT"),
                        help="Use an existing locked Cargo probe, preserving its dependency patches, source and imports")
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--runs", type=int, default=3)
    parser.add_argument("--repeats", type=int, default=5, help="Additional cached requests after the first request")
    parser.add_argument("--import-style", choices=IMPORTS, default="glob")
    parser.add_argument("--required-completions",
                        default="meters,kilograms,seconds,radians,hertzs,newtons,joules,newton_meters,watts,pascals",
                        help="Comma-separated methods validated after timed meters() requests")
    parser.add_argument("--receiver", choices=["variable", "literal"], default="variable")
    parser.add_argument("--target-dir", type=Path, help="Optional shared Cargo artifact directory")
    parser.add_argument("--load-timeout", type=float, default=180)
    parser.add_argument("--request-timeout", type=float, default=90)
    parser.add_argument("--prepare-only", action="store_true")
    parser.add_argument("--allow-missing-completion", action="append", default=[], metavar="LABEL",
                        help="Permit missing completion only for this baseline; omit its completion timing")
    parser.add_argument("--expand-on-failure", action="store_true", help="Save a recursive declarator macro expansion for diagnosis")
    parser.add_argument("--skip-warmup", action="store_true", help="Use only with already warmed profile projects; recorded in report")
    parser.add_argument("--continue-on-error", action="store_true", help="Collect failures for every profile, still exit nonzero")
    args = parser.parse_args()
    if args.runs < 1 or args.repeats < 0:
        parser.error("runs must be positive and repeats nonnegative")
    args.required_completions = list(dict.fromkeys(["meters"] + args.required_completions.split(",")))
    if not all(re.fullmatch(r"[a-z_][a-z0-9_]*", method) for method in args.required_completions):
        parser.error("required completion methods must be lowercase identifiers")
    args.server = args.server.expanduser().resolve()
    args.output = args.output.expanduser().resolve()
    args.output.mkdir(parents=True, exist_ok=True)
    profiles = []
    for label, dependency, features in args.profile:
        if not re.fullmatch(r"[a-z][a-z0-9_-]*", label):
            parser.error("profile labels must be lowercase identifiers")
        if any(p["label"] == label for p in profiles):
            parser.error("profile labels must be unique")
        dependency = str(Path(dependency).expanduser().resolve())
        if not (Path(dependency) / "Cargo.toml").is_file():
            parser.error(f"dependency has no Cargo.toml: {dependency}")
        profiles.append({"label": label, "dependency_path": dependency,
                         "features": [x for x in features.split(",") if x], "default_features": False,
                         "source_sha256": source_fingerprint(Path(dependency))})
    for label, directory in args.project_profile:
        if not re.fullmatch(r"[a-z][a-z0-9_-]*", label) or any(p["label"] == label for p in profiles):
            parser.error("project profile labels must be unique lowercase identifiers")
        project = Path(directory).expanduser().resolve()
        if not (project / "Cargo.toml").is_file() or not (project / "src/main.rs").is_file():
            parser.error(f"project profile needs Cargo.toml and src/main.rs: {project}")
        profiles.append({"label": label, "project_path": str(project), "dependency_path": str(project.parent),
                         "source_sha256": source_fingerprint(project.parent),
                         "probe_sha256": source_fingerprint(project)})
    if not profiles:
        parser.error("at least one --profile or --project-profile is required")
    if set(args.allow_missing_completion) - {p["label"] for p in profiles}:
        parser.error("--allow-missing-completion labels must match a --profile")
    report = {"scope": "fresh server, warm Cargo artifacts; direct LSP without editor/formatter",
              "rss_scope": "Linux rust-analyzer process RSS only; sampled every 50 ms; child processes excluded",
              "profiles": profiles, "server": str(args.server), "import_style": args.import_style,
              "receiver": args.receiver, "allow_missing_completion": args.allow_missing_completion,
              "timed_completion_method": "meters", "required_completions": args.required_completions,
              "cargo_warmup": not args.skip_warmup, "cache_priming": True,
              "cargo_lock_policy": "Existing project profiles use --locked for warmup and analysis; generated profiles may create a lock during warmup, then analysis uses --locked",
              "runs": args.runs, "repeated_requests": args.repeats, "samples": []}
    projects = {}
    for profile in profiles:
        if "project_path" in profile:
            project = Path(profile["project_path"])
            projects[profile["label"]] = (project, (project / "src/main.rs").read_text())
        else:
            project = args.output / "projects" / profile["label"]
            projects[profile["label"]] = (project, make_project(project, profile, args.import_style))
    write_json(args.output / "results.json", report)
    if args.prepare_only:
        print(json.dumps({"prepared": [str(project) for project, _ in projects.values()]}))
        return 0
    if not args.server.is_file():
        parser.error(f"server executable not found: {args.server}")
    environment = dict(os.environ, CARGO_NET_OFFLINE="true", CARGO_INCREMENTAL="0",
                       CARGO_TARGET_DIR=str(args.target_dir.resolve() if args.target_dir else args.output / "target"))
    report["server_version"] = subprocess.check_output([str(args.server), "--version"], text=True).strip()
    report["rustc_version"] = subprocess.check_output(["rustc", "--version"], text=True).strip()
    if not args.skip_warmup:
        for profile in profiles:
            project, _ = projects[profile["label"]]
            with (args.output / f'{profile["label"]}.cargo-warmup.log').open("w") as log:
                command = ["cargo", "check", "--offline"]
                if "project_path" in profile:
                    command.append("--locked")
                subprocess.run(command, cwd=project, env=environment,
                               stdout=log, stderr=subprocess.STDOUT, check=True)
    for cycle in range(1, args.runs + 1):
        order = profiles if cycle % 2 else list(reversed(profiles))
        for profile in order:
            project, source = projects[profile["label"]]
            row = measure(args, profile, cycle, project, source, environment)
            report["samples"].append(row)
            report["summary"] = summarize(report["samples"])
            write_json(args.output / "results.json", report)
            print(json.dumps({"event": "sample", "profile": profile["label"], "cycle": cycle,
                              "valid": row["valid"], "error": row.get("error")}), flush=True)
            if not row["valid"] and not args.continue_on_error:
                return 1
    return 0 if all(row["valid"] for row in report["samples"]) else 1


if __name__ == "__main__":
    raise SystemExit(main())
