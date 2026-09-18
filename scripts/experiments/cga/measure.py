#!/usr/bin/env python3
"""Compare explicit source/backend/compiler variants using one dynamics workload.

Each input is copied into an immutable prepared snapshot, then installed at the
same source path for each sample. Selected dependencies are cleaned before the
warmup and again inside the measured dependency-chain check. Unrelated cached
dependencies remain warm. A separate consumer-only check follows by default.
No input snapshot or registry source is modified. Partial results survive errors.
"""

import argparse
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import statistics
import subprocess
import sys
import time


PACKAGES = ["typenum", "whippyunits-core", "whippyunits-proc-macros", "whippyunits",
            "whippyalgebra", "nalgebra", "simba", "dynamics_probe"]
IGNORED = {".git", "target", "__pycache__"}
TIMING_FORMAT = '{"max_rss_kib":%M,"user_seconds":%U,"system_seconds":%S}'


def fingerprint(root):
    digest = hashlib.sha256()
    for path in sorted(root.rglob("*")):
        if path.is_file() and not IGNORED.intersection(path.relative_to(root).parts):
            digest.update(str(path.relative_to(root)).encode() + b"\0")
            digest.update(path.read_bytes() + b"\0")
    return digest.hexdigest()


def file_hash(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def write_report(path, report):
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_name(path.name + ".tmp")
    temporary.write_text(json.dumps(report, indent=2) + "\n")
    temporary.replace(path)


def command_for(variant):
    command = ["rustup", "run", variant["toolchain"], "cargo", "check", "--offline",
               "--locked", "--manifest-path", "probe/Cargo.toml", "-p", "dynamics_probe", "-j1"]
    if variant["features"]:
        command += ["--features", variant["features"]]
    return command


def clean_for(variant, packages):
    command = ["rustup", "run", variant["toolchain"], "cargo", "clean", "--offline",
               "--locked", "--manifest-path", "probe/Cargo.toml"]
    for package in packages:
        command += ["-p", package]
    return command


def update_medians(report):
    medians = {}
    keys = ["seconds", "max_rss_kib", "user_seconds", "system_seconds"]
    for variant in report["variants"]:
        label = variant["label"]
        results = {}
        for scope in ("dependency_chain", "consumer"):
            rows = [row[scope] for row in report["samples"]
                    if row["variant"] == label and scope in row and row[scope]["status"] == "passed"]
            if rows:
                results[scope] = {"sample_count": len(rows),
                                  **{key: statistics.median(row[key] for row in rows) for key in keys}}
        if results:
            medians[label] = results
    report["medians"] = medians


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--variant", nargs=3, action="append", required=True,
                        metavar=("LABEL", "TREE", "TOOLCHAIN"))
    parser.add_argument("--features", nargs=2, action="append", default=[],
                        metavar=("LABEL", "FEATURES"), help="Extra Cargo features for one variant")
    parser.add_argument("--samples", type=int, default=5)
    parser.add_argument("--scratch", type=Path, required=True, help="Fresh dedicated experiment directory")
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--prepare-only", action="store_true",
                        help="Copy/validate fixtures and record commands/compiler versions without compiling")
    parser.add_argument("--no-consumer-after", action="store_true",
                        help="Skip the separately timed warm consumer-only check")
    args = parser.parse_args()
    labels = [label for label, _, _ in args.variant]
    if args.samples < 1 or len(set(labels)) != len(labels):
        parser.error("positive samples and unique variant labels required")
    if any(not re.fullmatch(r"[A-Za-z0-9_-]+", label) for label in labels):
        parser.error("labels must contain only ASCII letters, digits, underscores, and hyphens")
    features = {}
    for label, value in args.features:
        if label not in labels or label in features:
            parser.error("--features requires a known label and may be specified once per label")
        features[label] = value
    scratch = args.scratch.resolve()
    output = args.output.resolve()
    if scratch.exists() and any(scratch.iterdir()):
        parser.error("scratch must be absent or empty; choose a fresh directory")
    trees = [(label, Path(tree).resolve(), toolchain) for label, tree, toolchain in args.variant]
    for _, tree, _ in trees:
        if not (tree / "Cargo.toml").is_file():
            parser.error(f"not a source tree: {tree}")
        if tree == scratch or tree in scratch.parents or scratch in tree.parents:
            parser.error("scratch and input trees must not overlap")
        if output == tree or tree in output.parents:
            parser.error("output must not modify an input source tree")
    scratch.mkdir(parents=True, exist_ok=True)
    source = scratch / "source"
    target = scratch / "target"
    prepared = scratch / "prepared"
    prepare_script = Path(__file__).resolve().parents[2] / "prepare_dynamics_probe.py"
    expected_main_hash = file_hash(prepare_script.with_name("dynamics_probe.rs"))
    expected_lock_hash = file_hash(prepare_script.with_name("dynamics_probe.Cargo.lock"))
    env = dict(os.environ, CARGO_INCREMENTAL="0", CARGO_TARGET_DIR=str(target))
    report = {
        "started_at_utc": datetime.now(timezone.utc).isoformat(), "status": "preparing",
        "scope": "selected dependency-chain packages rebuilt; unrelated dependencies warm; incremental off; jobs=1",
        "rss_scope": "GNU time maximum individual-process RSS including waited-for children; KiB, not aggregate memory",
        "comparison_rule": "Compare backends on the same compiler; stable versus nightly also includes compiler changes",
        "scratch": str(scratch), "samples_requested": args.samples, "consumer_after": not args.no_consumer_after,
        "environment": {key: env.get(key) for key in
                        ("CARGO_INCREMENTAL", "CARGO_TARGET_DIR", "RUSTFLAGS", "CARGO_ENCODED_RUSTFLAGS",
                         "RUSTDOCFLAGS", "RUSTC_WRAPPER", "RUSTC_WORKSPACE_WRAPPER", "RUSTC_BOOTSTRAP",
                         "CARGO_BUILD_RUSTFLAGS", "CARGO_PROFILE_DEV_OPT_LEVEL", "CARGO_PROFILE_DEV_DEBUG")},
        "fixture": {"preparation_script": str(prepare_script), "script_sha256": file_hash(prepare_script),
                    "main_sha256": expected_main_hash, "lock_sha256": expected_lock_hash},
        "clean_packages": PACKAGES, "variants": [], "samples": [], "events": [], "medians": {}}
    write_report(output, report)
    try:
        for label, tree, toolchain in trees:
            variant = {"label": label, "tree": str(tree), "input_sha256": fingerprint(tree),
                       "toolchain": toolchain, "features": features.get(label, "")}
            report["variants"].append(variant)
            write_report(output, report)
            destination = prepared / label
            shutil.copytree(tree, destination, ignore=shutil.ignore_patterns(*IGNORED))
            probe = destination / "probe"
            preparation_log = scratch / f"prepare-{label}.log"
            if not probe.exists():
                preparation_command = [sys.executable, str(prepare_script), str(destination)]
                with preparation_log.open("w") as stream:
                    subprocess.run(preparation_command, stdout=stream, stderr=subprocess.STDOUT, check=True)
                variant["preparation_command"] = preparation_command
            else:
                # A prepared input may include a local dependency patch. Preserve
                # its manifest, but enforce an identical body and locked versions.
                variant["preparation_command"] = "existing probe validated; not regenerated"
            if file_hash(probe / "src/main.rs") != expected_main_hash:
                raise ValueError(f"{label}: probe body differs from the checked-in workload")
            if file_hash(probe / "Cargo.lock") != expected_lock_hash:
                raise ValueError(f"{label}: probe lock differs; use identical resolved dependencies")
            variant["probe_manifest"] = (probe / "Cargo.toml").read_text()
            variant["probe_manifest_sha256"] = file_hash(probe / "Cargo.toml")
            variant["prepared_sha256"] = fingerprint(destination)
            variant["prepared_tree"] = str(destination)
            variant["rustc_verbose"] = subprocess.check_output(
                ["rustup", "run", toolchain, "rustc", "--version", "--verbose"], text=True).strip()
            variant["cargo_version"] = subprocess.check_output(
                ["rustup", "run", toolchain, "cargo", "--version"], text=True).strip()
            variant["check_command"] = command_for(variant)
            variant["clean_command"] = clean_for(variant, PACKAGES)
            variant["consumer_clean_command"] = clean_for(variant, ["dynamics_probe"])
            variant["time_prefix"] = ["/usr/bin/time", "-f", TIMING_FORMAT]
            write_report(output, report)
        if args.prepare_only:
            report["status"] = "prepared_only"
            write_report(output, report)
            print(f"Prepared {len(trees)} variants; no compiler jobs run. Report: {output}", flush=True)
            return

        report["status"] = "running"
        for cycle in range(args.samples):
            order = report["variants"] if cycle % 2 == 0 else list(reversed(report["variants"]))
            for variant in order:
                label = variant["label"]
                row = {"variant": label, "cycle": cycle, "status": "running",
                       "log": str(scratch / f"{cycle}-{label}.log")}
                report["samples"].append(row)
                write_report(output, report)
                if source.exists():
                    shutil.rmtree(source)
                shutil.copytree(Path(variant["prepared_tree"]), source,
                                ignore=shutil.ignore_patterns(*IGNORED))
                with Path(row["log"]).open("w") as stream:
                    def run(phase, command):
                        event = {"variant": label, "cycle": cycle, "phase": phase,
                                 "command": command, "cwd": str(source), "status": "running"}
                        report["events"].append(event)
                        write_report(output, report)
                        stream.write("\n" + json.dumps({"phase": phase, "command": command}) + "\n")
                        stream.flush()
                        before = time.perf_counter()
                        result = subprocess.run(command, cwd=source, env=env,
                                                stdout=stream, stderr=subprocess.STDOUT)
                        event["wall_seconds"] = time.perf_counter() - before
                        event.update(returncode=result.returncode,
                                     status="passed" if result.returncode == 0 else "failed")
                        write_report(output, report)
                        return result

                    def measured(phase):
                        memory = scratch / f"{cycle}-{label}-{phase}-time.json"
                        command = variant["time_prefix"] + ["-o", str(memory)] + variant["check_command"]
                        result = run(phase, command)
                        elapsed = report["events"][-1]["wall_seconds"]
                        # GNU time can prefix a failure diagnostic to the JSON.
                        stats = {}
                        if memory.exists():
                            for line in memory.read_text().splitlines():
                                if line.startswith("{"):
                                    stats = json.loads(line)
                        row[phase] = {"status": "passed" if result.returncode == 0 else "failed",
                                      "seconds": elapsed, **stats, "time_output": str(memory)}
                        update_medians(report)
                        write_report(output, report)
                        result.check_returncode()

                    # Reset before warmup prevents old snapshot mtimes reusing
                    # artifacts produced by another backend at this same path.
                    run("reset_before_warmup", variant["clean_command"]).check_returncode()
                    run("warmup", variant["check_command"]).check_returncode()
                    run("clean_dependency_chain", variant["clean_command"]).check_returncode()
                    measured("dependency_chain")
                    if not args.no_consumer_after:
                        run("clean_consumer", variant["consumer_clean_command"]).check_returncode()
                        measured("consumer")
                row["status"] = "passed"
                write_report(output, report)
                print(json.dumps(row), flush=True)
        report["status"] = "complete"
        report["finished_at_utc"] = datetime.now(timezone.utc).isoformat()
        write_report(output, report)
        print(json.dumps(report["medians"], indent=2), flush=True)
    except BaseException as error:
        report["status"] = "interrupted" if isinstance(error, KeyboardInterrupt) else "failed"
        report["error"] = {"type": type(error).__name__, "message": str(error)}
        if report["samples"] and report["samples"][-1]["status"] == "running":
            report["samples"][-1]["status"] = report["status"]
        report["finished_at_utc"] = datetime.now(timezone.utc).isoformat()
        write_report(output, report)
        raise


if __name__ == "__main__":
    main()
