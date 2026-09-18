#!/usr/bin/env python3
"""Compile isolated GCA contracts; failures must match meaningful diagnostics."""

import argparse
import hashlib
import json
from pathlib import Path
import re
import subprocess
import sys


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--toolchain", default="nightly-2026-09-13")
    parser.add_argument("--output", type=Path, default=Path("/tmp/whippyunits-cga/probes"))
    parser.add_argument("--typenum", type=Path, default=Path("/home/bc/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/typenum-1.20.1"))
    args = parser.parse_args()
    sources = Path(__file__).resolve().parent / "probes"
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    rustc = ["rustc", "+" + args.toolchain]
    version = subprocess.check_output(rustc + ["--version", "--verbose"], text=True)
    report = {"compiler": version, "scope": "isolated correctness probes; no timing claim", "commands": []}

    def command(name, command, *, expected="success", patterns=(), error_codes=(), run=False):
        result = subprocess.run(command, text=True, capture_output=True, cwd=output)
        log = output / (name + ".log")
        log.write_text(result.stdout + result.stderr)
        crash = bool(re.search(r"internal compiler error|the compiler unexpectedly panicked|rustc unexpectedly panicked|thread .* panicked", log.read_text(), re.I)) or result.returncode < 0
        actual_error_codes = sorted(set(re.findall(r"^error\[(E[0-9]+)\]:", log.read_text(), re.M)))
        # All substantive errors must be the expected kind. A missing import,
        # undeclared name, or unsupported feature must not accidentally satisfy
        # a broad diagnostic substring in an expected-failure test.
        unclassified_error = bool(re.search(r"^error: (?!aborting due to|could not compile)", log.read_text(), re.M))
        if expected == "success":
            accepted = result.returncode == 0 and not crash
        else:
            accepted = (result.returncode != 0 and not crash and not unclassified_error
                        and bool(actual_error_codes) and set(actual_error_codes) <= set(error_codes)
                        and all(re.search(pattern, log.read_text(), re.S) for pattern in patterns))
        row = {"name": name, "command": command, "cwd": str(output), "expected": expected,
               "patterns": list(patterns), "exit_code": result.returncode,
               "allowed_error_codes": list(error_codes), "actual_error_codes": actual_error_codes,
               "unclassified_error": unclassified_error,
               "compiler_crash": crash, "passed": accepted, "log": str(log)}
        report["commands"].append(row)
        if accepted and run:
            execution = subprocess.run([str(output / name)], text=True, capture_output=True, cwd=output)
            row["execution"] = {"command": [str(output / name)], "exit_code": execution.returncode,
                                "stdout": execution.stdout, "stderr": execution.stderr}
            row["passed"] = execution.returncode == 0
        print(name, "PASS" if row["passed"] else "FAIL", flush=True)
        (output / "results.json").write_text(json.dumps(report, indent=2) + "\n")
        return row["passed"]

    typenum_lib = args.typenum.resolve() / "src/lib.rs"
    typenum_inputs = {str(path.relative_to(args.typenum)): digest(path) for path in sorted(args.typenum.rglob("*.rs"))}
    report["typenum_source"] = str(args.typenum.resolve())
    report["typenum_sources_sha256"] = typenum_inputs
    report["probe_sources_sha256"] = {p.name: digest(p) for p in sorted(sources.glob("*.rs"))}
    if not command("build_typenum", rustc + ["--crate-name", "typenum", "--crate-type", "rlib", "--edition=2018", str(typenum_lib), "-o", str(output / "libtypenum.rlib")]):
        return 1
    externs = ["-L", "dependency=" + str(output), "--extern", "typenum=" + str(output / "libtypenum.rlib")]
    if not command("build_native_backend", rustc + ["--edition=2024", "--crate-name", "cga_probe", "--crate-type", "rlib", str(sources / "backend.rs"), *externs, "-o", str(output / "libcga_probe.rlib")]):
        return 1
    externs += ["--extern", "cga_probe=" + str(output / "libcga_probe.rlib")]
    command("positive", rustc + ["--edition=2024", "-C", "overflow-checks=yes", str(sources / "positive.rs"), *externs, "-o", str(output / "positive")], run=True)

    cases = {
        "odd_number_bound": ("use cga_probe::{N,Halve}; fn witness<T: Halve>() {} fn main() { witness::<N<3>>(); }", [r"Bool<false>.*True|True.*Bool<false>"]),
        "negative_odd_bound": ("use cga_probe::{N,Halve}; fn witness<T: Halve>() {} fn main() { witness::<N<-3>>(); }", [r"Bool<false>.*True|True.*Bool<false>"]),
        "odd_dimension_bound": ("use cga_probe::{Unit,witness_root}; fn main() { witness_root::<Unit<1,0>>(); }", [r"Bool<false>.*True|True.*Bool<false>"]),
        "odd_scale_bound": ("use cga_probe::{Unit,witness_root}; fn main() { witness_root::<Unit<2,-3>>(); }", [r"Bool<false>.*True|True.*Bool<false>"]),
        "add_overflow": ("use cga_probe::N; fn main() { let value = N::<32767> + N::<1>; std::hint::black_box(value); }", [r"E0080", r"overflow"]),
        "subtract_overflow": ("use cga_probe::N; fn main() { let value = N::<-32768> - N::<1>; std::hint::black_box(value); }", [r"E0080", r"overflow"]),
        "negate_overflow": ("use cga_probe::N; fn main() { let value = -N::<-32768>; std::hint::black_box(value); }", [r"E0080", r"overflow"]),
        "wrong_canonical_output": ("use cga_probe::N; fn main() { let _: N<6> = N::<2> + N::<3>; }", [r"E0308", r"5.*6|6.*5"]),
        "wrong_equality_dispatch": ("use cga_probe::{AutoReduce,Uniform,Unit}; fn main() { let _: std::marker::PhantomData<Uniform> = std::marker::PhantomData::<<Unit<1,0> as AutoReduce<Unit<2,0>>>::Output>; }", [r"E0308", r"Uniform", r"Mixed"]),
        "generic_zero_identity": ("use cga_probe::N; fn zero<const A: i16>(a: N<A>) -> N<A> { a + N::<0> } fn main() {}", [r"E0284|E0308", r"N<A>"]),
    }
    for name, (source, patterns) in cases.items():
        source_path = output / (name + ".rs")
        source_path.write_text(source + "\n")
        report.setdefault("generated_sources_sha256", {})[source_path.name] = digest(source_path)
        if name in {"odd_number_bound", "negative_odd_bound", "odd_dimension_bound", "odd_scale_bound"}:
            error_codes = ("E0277",)
        elif "overflow" in name:
            error_codes = ("E0080",)
        elif name == "generic_zero_identity":
            error_codes = ("E0284", "E0308")
        else:
            error_codes = ("E0308",)
        command(name, rustc + ["--edition=2024", "-C", "overflow-checks=yes", str(source_path), *externs, "-o", str(output / name)], expected="rejection", patterns=patterns, error_codes=error_codes)

    # A blanket Add impl has different availability semantics from a guarded
    # Halve impl: this bound can hold until somebody forces the output.
    source = output / "overflow_bound_only.rs"
    source.write_text("use cga_probe::N; use std::ops::Add; fn witness<T: Add<N<1>>>() {} fn main() { witness::<N<32767>>(); }\n")
    report["generated_sources_sha256"][source.name] = digest(source)
    command("overflow_bound_only", rustc + ["--edition=2024", str(source), *externs, "-o", str(output / "overflow_bound_only")], run=True)
    report["passed"] = all(row["passed"] for row in report["commands"])
    (output / "results.json").write_text(json.dumps(report, indent=2) + "\n")
    return 0 if report["passed"] else 1


if __name__ == "__main__":
    sys.exit(main())
