#!/usr/bin/env python3
"""Check real backend contracts using an external, locked consumer.

Does not modify --repo. Run stable/typenum and nightly/typenum before the same
nightly with --native. This is correctness validation, not a timing harness.
--native enables nightly-gca and adds full-i16 and forced-overflow cases.
"""

import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import signal
import subprocess
import sys
import tomllib


COMMON_POSITIVE = ("acceptance", "generic_and_roots")
NEGATIVE = {
    "odd_dimension_bound": (("E0277",), (r"UnitSqrt|Halve|PartialDiv",)),
    "odd_scale_bound": (("E0277",), (r"UnitSqrt|Halve|PartialDiv",)),
    "odd_cholesky": (("E0277",), (r"UnitSqrt|Halve|PartialDiv", r"cholesky")),
    "wrong_dimensions": (("E0308",), (r"mismatched types",)),
    "wrong_gauge": (("E0277",), (r"gauge|GaugeReproduces",)),
    "wrong_brand": (("E0308",), (r"mismatched types", r"FrameA", r"FrameB")),
}
NATIVE_NEGATIVE = {
    name: (("E0080",), (r"overflow",))
    for name in ("add_overflow", "sub_overflow", "neg_overflow")
}
CRASH = re.compile(
    r"internal compiler error|the compiler unexpectedly panicked|"
    r"rustc unexpectedly panicked|thread .* panicked", re.I
)


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def source_hashes(repo):
    paths = []
    packages = ("", "whippyunits-core", "whippyunits-num", "proc-macros", "whippyalgebra", "whippyalgebra-macros")
    for package in packages:
        base = repo / package
        paths.extend(p for p in (base / "Cargo.toml", base / "build.rs") if p.is_file())
        paths.extend(sorted((base / "src").rglob("*.rs")))
    paths.extend(p for p in (repo / "Cargo.lock", repo / "rust-toolchain.toml") if p.is_file())
    return {str(p.relative_to(repo)): digest(p) for p in sorted(set(paths))}


def locked_packages(path):
    """Local nodes may change; every external identity must remain pinned."""
    packages = tomllib.loads(path.read_text())["package"]
    external = [{key: package.get(key) for key in ("name", "version", "source", "checksum")}
                for package in packages if "source" in package]
    external.sort(key=lambda package: (package["name"], package["version"], package["source"]))
    local = sorted((package for package in packages if "source" not in package),
                   key=lambda package: (package["name"], package["version"]))
    return external, local


def messages(diagnostic):
    """Semantic diagnostics, excluding source snippets that can match by accident."""
    text = [diagnostic.get("message", "")]
    for child in diagnostic.get("children", []):
        text.extend(messages(child))
    return text


def compiler_errors(stdout):
    errors = []
    for line in stdout.splitlines():
        try:
            item = json.loads(line)
        except json.JSONDecodeError:
            continue
        if item.get("reason") == "compiler-message":
            diagnostic = item["message"]
            if diagnostic.get("level") == "error":
                errors.append(diagnostic)
    return errors


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo", type=Path, required=True)
    parser.add_argument("--toolchain", required=True)
    parser.add_argument("--output", type=Path, required=True,
                        help="external directory for consumer, logs, and results.json")
    parser.add_argument("--target", type=Path,
                        help="external Cargo target (default: OUTPUT/target)")
    parser.add_argument("--native", action="store_true")
    parser.add_argument("--timeout", type=int, default=300)
    args = parser.parse_args()
    repo, output = args.repo.resolve(), args.output.resolve()
    target = args.target.resolve() if args.target else output / "target"
    for package in ("", "whippyunits-core", "whippyalgebra"):
        if not (repo / package / "Cargo.toml").is_file():
            parser.error("--repo is missing a required manifest: " + str(repo / package / "Cargo.toml"))
    for path in (output, target):
        if path == repo or repo in path.parents:
            parser.error("output and target must be outside --repo to preserve its source")
    if (output / "results.json").exists() or (output / "consumer").exists():
        parser.error("use a fresh --output directory; previous results are preserved")
    if args.timeout <= 0:
        parser.error("--timeout must be positive")
    sources = Path(__file__).resolve().parent / "integration"
    lock_source = Path(__file__).resolve().parents[2] / "dynamics_probe.Cargo.lock"
    project = output / "consumer"
    bins = project / "src/bin"
    bins.mkdir(parents=True)
    names = list(COMMON_POSITIVE) + list(NEGATIVE)
    if args.native:
        names += ["native_full_i16"] + list(NATIVE_NEGATIVE)
    for name in names:
        shutil.copy2(sources / (name + ".rs"), bins / (name + ".rs"))

    native = ', "nightly-gca"' if args.native else ""
    manifest = f'''[package]
name = "dynamics_probe"
version = "0.0.0"
edition = "2024"
[workspace]
[dependencies]
whippyunits = {{path={json.dumps(str(repo))}, default-features=false, features=["std", "units-dynamics-conversions"{native}]}}
whippyalgebra = {{path={json.dumps(str(repo / "whippyalgebra"))}, default-features=false, features=["std", "nalgebra", "units-dynamics-conversions"{native}]}}
whippyunits-core = {{path={json.dumps(str(repo / "whippyunits-core"))}, default-features=false}}
typenum = {{version="1", default-features=false}}
'''
    (project / "Cargo.toml").write_text(manifest)
    # Only add existing dependency edges, preserving every pinned package/version.
    # Direct numeral/Bit access needs core and typenum; the measured consumer
    # already includes both transitively. The metadata preparation below may
    # refresh local packages/edges, but must preserve all external identities.
    old_entry = '''name = "dynamics_probe"
version = "0.0.0"
dependencies = [
 "whippyalgebra",
 "whippyunits",
]'''
    new_entry = '''name = "dynamics_probe"
version = "0.0.0"
dependencies = [
 "typenum",
 "whippyalgebra",
 "whippyunits",
 "whippyunits-core",
]'''
    lock = lock_source.read_text()
    if lock.count(old_entry) != 1:
        parser.error("unexpected baseline lockfile: dynamics_probe dependency entry changed")
    (project / "Cargo.lock").write_text(lock.replace(old_entry, new_entry))
    env = dict(os.environ, RUSTUP_TOOLCHAIN=args.toolchain, CARGO_INCREMENTAL="0",
               CARGO_TARGET_DIR=str(target))
    # An ambient flag must not silently enable bootstrap or a different solver.
    inherited = {key: env[key] for key in
                 ("RUSTFLAGS", "CARGO_ENCODED_RUSTFLAGS", "RUSTC_BOOTSTRAP") if key in env}
    if inherited:
        parser.error("unset compiler overrides before validation: " + ", ".join(inherited))
    version_command = ["rustc", "-Vv"]
    rustc = subprocess.check_output(version_command, env=env, text=True)
    inputs = source_hashes(repo)
    report = {
        "scope": "production integration correctness; no performance claim",
        "repo": str(repo), "toolchain": args.toolchain, "native": args.native,
        "rustc_command": version_command, "rustc": rustc,
        "environment": {k: env[k] for k in
                        ("RUSTUP_TOOLCHAIN", "CARGO_INCREMENTAL", "CARGO_TARGET_DIR")},
        "source_sha256": inputs,
        "probe_sha256": {name + ".rs": digest(bins / (name + ".rs")) for name in names},
        "manifest_sha256": digest(project / "Cargo.toml"),
        "baseline_lock_sha256": digest(lock_source),
        "consumer_lock_seed_sha256": digest(project / "Cargo.lock"),
        "lock_adjustment": "add direct core/typenum edges and resolve local graph changes only; external identities fixed",
        "planned_cases": names, "status": "preparing", "all_passed": False, "checks": [],
    }

    def save():
        (output / "results.json").write_text(json.dumps(report, indent=2) + "\n")

    def run_case(name, expected, codes=(), patterns=()):
        # cargo check forces the native exponent outputs in the negative bins;
        # positive bins also execute their conversion/numerical assertions.
        command = ["cargo", "run" if expected == "success" else "check", "--offline", "--locked",
                   "-j2", "--manifest-path", str(project / "Cargo.toml"), "--bin", name,
                   "--message-format=json"]
        process = subprocess.Popen(command, cwd=project, env=env, text=True,
                                   stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                                   start_new_session=True)
        try:
            stdout, stderr = process.communicate(timeout=args.timeout)
            status = process.returncode
        except subprocess.TimeoutExpired:
            # Cargo can have rustc children; terminate the whole group so a
            # timeout never leaves compilation running during later timings.
            os.killpg(process.pid, signal.SIGKILL)
            stdout, stderr = process.communicate()
            status = "timeout"
        diagnostics = compiler_errors(stdout)
        actual_codes = sorted({d["code"]["code"] for d in diagnostics if d.get("code")})
        semantic = "\n".join(text for d in diagnostics for text in messages(d))
        log = output / (name + ".log")
        log.write_text(stdout + "\n--- stderr ---\n" + stderr)
        crashed = bool(CRASH.search(stdout + stderr)) or (isinstance(status, int) and status < 0)
        unclassified = any(not d.get("code") for d in diagnostics)
        if expected == "success":
            passed = status == 0 and not diagnostics and not crashed
        else:
            passed = (isinstance(status, int) and status != 0 and not crashed
                      and bool(diagnostics) and not unclassified
                      and set(actual_codes) <= set(codes)
                      and all(re.search(p, semantic, re.I | re.S) for p in patterns))
        row = {"name": name, "expected": expected, "command": command,
               "cwd": str(project), "status": status, "passed": passed,
               "allowed_error_codes": list(codes), "actual_error_codes": actual_codes,
               "required_diagnostic_patterns": list(patterns), "diagnostics": diagnostics,
               "compiler_crash": crashed, "unclassified_error": unclassified, "log": str(log)}
        report["checks"].append(row)
        save()
        print(name + ": " + ("PASS" if passed else "FAIL"), flush=True)
        return passed

    save()
    expected_external, reference_local = locked_packages(lock_source)
    metadata_command = ["cargo", "metadata", "--offline", "--format-version=1",
                        "--manifest-path", str(project / "Cargo.toml")]
    metadata_log = output / "lock-refresh.log"
    try:
        metadata = subprocess.run(metadata_command, cwd=project, env=env, text=True,
                                  stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                                  timeout=args.timeout)
        metadata_log.write_text(metadata.stderr)
        metadata_status = metadata.returncode
    except subprocess.TimeoutExpired as error:
        stderr = error.stderr or ""
        metadata_log.write_text(stderr.decode(errors="replace") if isinstance(stderr, bytes) else stderr)
        metadata_status = "timeout"
    report["lock_refresh"] = {"command": metadata_command, "cwd": str(project),
                              "status": metadata_status, "log": str(metadata_log)}
    if metadata_status != 0:
        report["status"] = "lock-preparation-failed"
        save()
        return 1
    actual_external, actual_local = locked_packages(project / "Cargo.lock")
    report["external_packages"] = expected_external
    report["external_packages_match_reference"] = expected_external == actual_external
    report["reference_local_packages"] = reference_local
    report["consumer_local_packages"] = actual_local
    report["consumer_lock_sha256"] = digest(project / "Cargo.lock")
    if actual_external != expected_external:
        report["actual_external_packages"] = actual_external
        report["status"] = "external-lock-drift"
        save()
        print("FAIL: external dependency versions, sources, or checksums changed", flush=True)
        return 1
    report["status"] = "running"
    save()
    for name in COMMON_POSITIVE + (("native_full_i16",) if args.native else ()):
        if not run_case(name, "success"):
            # A broken dependency or setup must never make negative cases look valid.
            report["status"] = "positive-preflight-failed"
            report["source_unchanged"] = inputs == source_hashes(repo)
            save()
            return 1
    for name, (codes, patterns) in {**NEGATIVE, **(NATIVE_NEGATIVE if args.native else {})}.items():
        run_case(name, "rejection", codes, patterns)
    report["source_unchanged"] = inputs == source_hashes(repo)
    report["consumer_lock_unchanged"] = report["consumer_lock_sha256"] == digest(project / "Cargo.lock")
    report["all_passed"] = (all(row["passed"] for row in report["checks"])
                            and report["source_unchanged"] and report["consumer_lock_unchanged"])
    report["status"] = "complete"
    save()
    return 0 if report["all_passed"] else 1


if __name__ == "__main__":
    sys.exit(main())
