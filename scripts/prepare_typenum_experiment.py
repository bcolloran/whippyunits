#!/usr/bin/env python3
"""Prepare PRIVATE typenum forks for controlled experiments; never edits a registry.

The constants fork preserves U0..U400 and signed +/-400 by default, enough for
whippyunits' wide exponent bridge and nalgebra's U0..U127 dimension bridge.
Larger named constants/const mappings are deliberately removed. The modules
fork removes arrays, tuple implementations and the op! DSL. Neither fork is
an API-compatible replacement for upstream typenum. Licenses stay in each copy.
Use the full vendor control to avoid confounding a registry-to-path switch.
"""

import argparse
import json
from pathlib import Path
import re
import shutil
import tomllib


def replace(path, old, new):
    source = path.read_text()
    if old not in source:
        raise ValueError(f"expected source fragment absent in {path}: {old}")
    path.write_text(source.replace(old, new))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source", type=Path, required=True, help="Repository snapshot including probe/Cargo.toml")
    parser.add_argument("--typenum", type=Path, required=True, help="Upstream typenum 1.20.1 directory, read only")
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--keep-range", type=int, default=400)
    args = parser.parse_args()
    args.output = args.output.resolve()
    if args.output.exists():
        parser.error("output already exists; use a fresh directory")
    if args.keep_range < 127:
        parser.error("nalgebra requires unsigned aliases through 127")
    package = tomllib.loads((args.typenum / "Cargo.toml").read_text())["package"]
    if (package["name"], package["version"]) != ("typenum", "1.20.1"):
        parser.error("this source transformation is pinned to typenum 1.20.1")
    stats = {}
    for label in ("full", "constants", "modules", "slim"):
        root = args.output / label
        shutil.copytree(args.source, root, ignore=shutil.ignore_patterns(".git", "target", "__pycache__"))
        vendor = root / "vendor/typenum"
        shutil.copytree(args.typenum, vendor)
        for manifest, relative in ((root / "Cargo.toml", "vendor/typenum"),
                                   (root / "probe/Cargo.toml", "../vendor/typenum")):
            with manifest.open("a") as out:
                out.write('\n[patch.crates-io]\ntypenum = { path = ' + json.dumps(relative) + ' }\n')
        if label in ("constants", "slim"):
            path = vendor / "src/gen/consts.rs"
            source = path.read_text()
            pattern = r"^pub type ([UPN])(\d+)\s*=\s*[^;]+;\n?"
            source = re.sub(pattern, lambda m: m[0] if int(m[2]) <= args.keep_range else "", source, flags=re.M)
            found = {(kind, int(number)) for kind, number in re.findall(r"^pub type ([UPN])(\d+)\s*=", source, re.M)}
            expected = {("U", n) for n in range(args.keep_range + 1)}
            expected |= {(kind, n) for kind in "PN" for n in range(1, args.keep_range + 1)}
            if found != expected:
                raise ValueError("constant pruning did not produce the exact expected alias set")
            path.write_text(source)
            path = vendor / "src/gen/generic_const_mappings.rs"
            source = path.read_text()
            pattern = r"(?:#\[cfg\([^\n]+\)\]\n)?impl ToUInt for Const<(\d+)> \{\n    type Output = U\d+;\n\}\n?"
            source = re.sub(pattern, lambda m: m[0] if int(m[1]) <= args.keep_range else "", source)
            if {int(n) for n in re.findall(r"impl ToUInt for Const<(\d+)>", source)} != set(range(args.keep_range + 1)):
                raise ValueError("mapping pruning did not produce the exact expected implementation set")
            path.write_text(source)
        if label in ("modules", "slim"):
            lib = vendor / "src/lib.rs"
            replace(lib, "pub mod array;\npub mod tuple;", "// Arrays and tuples removed in private performance experiment.")
            replace(lib, "    array::{ATerm, TArr},\n", "")
            replace(lib, "        ATerm, Bit, Equal, Greater, Less, NInt, NonZero, PInt, TArr, UInt, UTerm, Unsigned, B0, B1,", "        Bit, Equal, Greater, Less, NInt, NonZero, PInt, UInt, UTerm, Unsigned, B0, B1,")
            replace(lib, "    impl Sealed for ATerm {}\n    impl<V, A> Sealed for TArr<V, A> {}\n", "")
            replace(vendor / "src/gen.rs", "pub mod op;", "// op! DSL removed in private performance experiment.")
        stats[label] = {"constant_aliases": len(re.findall(r"^pub type ", (vendor / "src/gen/consts.rs").read_text(), re.M)),
                        "mapping_impls": len(re.findall(r"^impl ToUInt", (vendor / "src/gen/generic_const_mappings.rs").read_text(), re.M)),
                        "constants_bytes": (vendor / "src/gen/consts.rs").stat().st_size}
        (vendor / "EXPERIMENT.md").write_text(
            f"Private experiment: {label}. Based on typenum 1.20.1.\n"
            "Not a supported or API-compatible upstream replacement.\n"
            f"Pruned variants keep numeric aliases/mappings through {args.keep_range}.\n"
            "Upstream doctests referencing removed public APIs are not expected to pass.\n")
    (args.output / "variants.json").write_text(json.dumps(stats, indent=2) + "\n")
    print(json.dumps(stats, indent=2))


if __name__ == "__main__":
    main()
