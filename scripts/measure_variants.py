#!/usr/bin/env python3
"""Interleave isolated source variants at one path, measuring check time and RSS.

Each input is a complete, immutable tree. --manifest is relative to that tree.
The caller supplies a consumer and dependency patches inside each snapshot.
Only this run's source/target directories are modified. Dependencies are warmed
before each sample; --clean selects the packages rebuilt inside the window.
Without --clean only the consumer package is rebuilt. GNU time reports maximum
single-process RSS in the Cargo child tree, not aggregate concurrent memory.
"""

import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import statistics
import subprocess
import time


def fingerprint(root):
    digest = hashlib.sha256()
    for path in sorted(root.rglob("*")):
        if path.is_file() and not any(x in {"target", ".git", "__pycache__"} for x in path.parts):
            digest.update(str(path.relative_to(root)).encode() + b"\0" + path.read_bytes() + b"\0")
    return digest.hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--variant", nargs=2, action="append", required=True, metavar=("LABEL", "TREE"))
    parser.add_argument("--manifest", default="probe/Cargo.toml")
    parser.add_argument("--package", default="dynamics_probe")
    parser.add_argument("--clean", action="append", default=[], help="Rebuild this package in each measured sample")
    parser.add_argument("--reset", action="append", default=[],
                        help="Clean a changed dependency BEFORE warmup; required for consumer-only source comparisons")
    parser.add_argument("--consumer-after", action="store_true",
                        help="After each cold sample, rebuild only the consumer and record its warmed check separately")
    parser.add_argument("--samples", type=int, default=5)
    parser.add_argument("--scratch", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    if args.samples < 1 or len({v[0] for v in args.variant}) != len(args.variant):
        parser.error("positive samples and unique variant labels required")
    args.scratch = args.scratch.resolve()
    args.scratch.mkdir(parents=True, exist_ok=True)
    source = args.scratch / "source"
    target = args.scratch / "target"
    if source.exists() or target.exists():
        parser.error("scratch source/target already exists; choose a fresh directory")
    variants = [(name, Path(tree).resolve()) for name, tree in args.variant]
    command = ["cargo", "check", "--offline", "--locked", "--manifest-path", args.manifest,
               "-p", args.package, "-j1"]
    clean = ["cargo", "clean", "--offline", "--manifest-path", args.manifest]
    for package in args.clean or [args.package]:
        clean += ["-p", package]
    reset = ["cargo", "clean", "--offline", "--manifest-path", args.manifest]
    for package in args.reset or args.clean or [args.package]:
        reset += ["-p", package]
    env = dict(os.environ, CARGO_INCREMENTAL="0", CARGO_TARGET_DIR=str(target))
    consumer_clean = ["cargo", "clean", "--offline", "--manifest-path", args.manifest, "-p", args.package]
    rows = []
    report = {"scope": "check after selected package clean; unrelated dependencies warm; incremental off; jobs=1",
              "rss_scope": "GNU time maximum single-process RSS including children; KiB, not aggregate RSS",
              "variants": [{"label": n, "tree": str(p), "sha256": fingerprint(p)} for n, p in variants],
              "command": command, "clean_command": clean, "reset_before_warmup_command": reset,
              "consumer_after": args.consumer_after,
              "rustc": subprocess.check_output(["rustc", "--version"], text=True).strip(),
              "samples": rows}
    args.output.parent.mkdir(parents=True, exist_ok=True)
    for cycle in range(args.samples):
        for label, tree in variants if cycle % 2 == 0 else list(reversed(variants)):
            if source.exists():
                shutil.rmtree(source)
            shutil.copytree(tree, source, ignore=shutil.ignore_patterns(".git", "target", "__pycache__"))
            log = args.scratch / f"{cycle}-{label}.log"
            with log.open("w") as stream:
                # Older snapshot mtimes must not let Cargo reuse an artifact
                # produced by a different variant at this same source path.
                subprocess.run(reset, cwd=source, env=env, stdout=stream, stderr=subprocess.STDOUT, check=True)
                # Warm this exact graph before cleaning the declared packages.
                subprocess.run(command, cwd=source, env=env, stdout=stream, stderr=subprocess.STDOUT, check=True)
                subprocess.run(clean, cwd=source, env=env, stdout=stream, stderr=subprocess.STDOUT, check=True)
                memory = args.scratch / "memory.json"
                def measured():
                    before = time.perf_counter()
                    subprocess.run(["/usr/bin/time", "-f", '{"max_rss_kib":%M,"user_seconds":%U,"system_seconds":%S}',
                                    "-o", str(memory)] + command, cwd=source, env=env,
                                   stdout=stream, stderr=subprocess.STDOUT, check=True)
                    return {"seconds": time.perf_counter() - before, **json.loads(memory.read_text())}
                row = {"variant": label, "cycle": cycle, **measured()}
                if args.consumer_after:
                    subprocess.run(consumer_clean, cwd=source, env=env, stdout=stream, stderr=subprocess.STDOUT, check=True)
                    row.update({"consumer_" + key: value for key, value in measured().items()})
            rows.append(row)
            report["medians"] = {name: {key: statistics.median(r[key] for r in rows if r["variant"] == name)
                                       for key in (("seconds", "max_rss_kib", "consumer_seconds", "consumer_max_rss_kib")
                                                   if args.consumer_after else ("seconds", "max_rss_kib"))}
                                 for name, _ in variants if any(r["variant"] == name for r in rows)}
            args.output.write_text(json.dumps(report, indent=2) + "\n")
            print(json.dumps(row), flush=True)
    print(json.dumps(report["medians"]), flush=True)


if __name__ == "__main__":
    main()
