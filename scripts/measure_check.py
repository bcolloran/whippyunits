#!/usr/bin/env python3
"""Compare a Git revision with the current working tree using interleaved checks.

Third-party dependencies are warm. Each sample rebuilds the selected packages
at one fixed source path, with incremental compilation disabled. The script
never checks out, cleans, or modifies the caller's repository.
"""

import argparse
import io
import json
import os
from pathlib import Path
import shutil
import statistics
import subprocess
import tarfile
import tempfile
import time


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--baseline", required=True)
    parser.add_argument("--features", default="std,units-dynamics")
    parser.add_argument("--baseline-features", help="Override --features for the baseline")
    parser.add_argument("--candidate-features", help="Override --features for the candidate")
    parser.add_argument("--samples", type=int, default=5)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--scratch", type=Path)
    args = parser.parse_args()
    if args.samples < 1:
        parser.error("--samples must be positive")
    root = Path(subprocess.check_output(["git", "rev-parse", "--show-toplevel"], text=True).strip())
    scratch = args.scratch or Path(tempfile.mkdtemp(prefix="whippyunits-check-"))
    scratch.mkdir(parents=True, exist_ok=True)
    baseline = scratch / "baseline"
    candidate = scratch / "candidate"
    source = scratch / "source"
    # Refuse a reused snapshot directory: these trees belong only to this run.
    for directory in (baseline, candidate, source):
        if directory.exists():
            parser.error(f"snapshot directory already exists: {directory}")
    baseline.mkdir()
    archive = subprocess.check_output(["git", "archive", args.baseline], cwd=root)
    with tarfile.open(fileobj=io.BytesIO(archive)) as tar:
        tar.extractall(baseline, filter="data")
    names = subprocess.check_output(
        ["git", "ls-files", "--cached", "--others", "--exclude-standard", "-z"], cwd=root
    ).split(b"\0")
    for raw in set(names) - {b""}:
        name = os.fsdecode(raw)
        original = root / name
        if original.is_file():
            destination = candidate / name
            destination.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(original, destination)
    env = dict(os.environ, CARGO_INCREMENTAL="0", CARGO_TARGET_DIR=str(scratch / "target"))
    command = ["cargo", "check", "--offline", "--locked", "-p", "whippyunits", "--lib",
               "--no-default-features", "--features", args.features]
    commands = {
        "baseline": command[:-1] + [args.baseline_features if args.baseline_features is not None else args.features],
        "candidate": command[:-1] + [args.candidate_features if args.candidate_features is not None else args.features],
    }
    clean = ["cargo", "clean", "--offline", "-p", "whippyunits-core",
             "-p", "whippyunits-proc-macros", "-p", "whippyunits"]
    rows = []
    report = {"baseline": args.baseline,
              "candidate_head": subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=root, text=True).strip(),
              "candidate_diff": subprocess.check_output(["git", "diff", "HEAD", "--stat"], cwd=root, text=True),
              "commands": commands, "clean_command": clean,
              "rustc": subprocess.check_output(["rustc", "--version"], text=True).strip(),
              "scope": "core, proc-macro and root library check; warm third-party dependencies; incremental disabled",
              "scratch": str(scratch), "samples": rows}
    args.output.parent.mkdir(parents=True, exist_ok=True)

    def run(variant, label, measured):
        if source.exists():
            shutil.rmtree(source)
        shutil.copytree(baseline if variant == "baseline" else candidate, source)
        subprocess.run(clean, cwd=source, env=env, check=True, capture_output=True)
        log = scratch / f"{label}-{variant}.log"
        start = time.perf_counter()
        with log.open("w") as stream:
            result = subprocess.run(commands[variant], cwd=source, env=env, stdout=stream, stderr=subprocess.STDOUT)
        seconds = time.perf_counter() - start
        if result.returncode:
            raise RuntimeError(f"{variant} failed; inspect {log}")
        if measured:
            row = {"variant": variant, "cycle": label, "seconds": seconds}
            rows.append(row)
            print(json.dumps(row), flush=True)
            args.output.write_text(json.dumps(report, indent=2) + "\n")

    for variant in ("baseline", "candidate"):
        run(variant, "warmup", False)
    for cycle in range(args.samples):
        order = ("baseline", "candidate") if cycle % 2 == 0 else ("candidate", "baseline")
        for variant in order:
            run(variant, cycle, True)
    medians = {variant: statistics.median(row["seconds"] for row in rows if row["variant"] == variant)
               for variant in ("baseline", "candidate")}
    report["medians_seconds"] = medians
    report["reduction_percent"] = 100 * (1 - medians["candidate"] / medians["baseline"])
    args.output.write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps({"medians": medians, "reduction_percent": report["reduction_percent"]}), flush=True)


if __name__ == "__main__":
    main()
