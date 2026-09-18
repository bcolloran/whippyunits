#!/usr/bin/env python3
"""Regenerate checked-in default constructor catalogs, or verify with --check.

Run from any directory. Requires the repository's pinned Rust toolchain and
Python 3; no cargo-expand, nightly features, build.rs, or additional packages.
"""
import argparse
import os
from pathlib import Path
import subprocess
import tempfile


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="fail if checked-in catalogs are stale")
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[1]
    destination = root / "src/generated/default_declarators"
    env = dict(os.environ)
    env.setdefault("CARGO_TARGET_DIR", str(root / "target/catalog-regeneration"))
    with tempfile.TemporaryDirectory(prefix="whippyunits-catalog-") as scratch:
        generated = Path(scratch)
        for mode, features in [("full", "all-units"), ("coherent", "units-si-dynamics"),
                               ("conversions", "units-dynamics-conversions")]:
            env.update(WHIPPY_CATALOG_OUT=str(generated), WHIPPY_CATALOG_MODE=mode)
            subprocess.run([
                "cargo", "test", "--locked", "-p", "whippyunits-proc-macros", "--lib",
                "--no-default-features", "--features", features, "export_default_catalog",
                "--", "--ignored", "--nocapture",
            ], cwd=root, env=env, check=True)
        required = {"mod.rs", "si_dynamics.rs", "dynamics_conversions.rs"}
        missing = sorted(name for name in required if not (generated / name).is_file())
        if missing:
            parser.exit(1, "Catalog export did not produce required files: " + ", ".join(missing) + "\nNo checked-in files were changed.\n")
        paths = sorted(generated.glob("*.rs"))
        subprocess.run(["rustfmt", "--edition", "2024", *map(str, paths)], cwd=root, check=True)
        expected = {p.name: p.read_bytes() for p in paths}
        actual = {p.name: p.read_bytes() for p in destination.glob("*.rs")} if destination.exists() else {}
        changed = sorted(name for name in expected.keys() | actual.keys() if expected.get(name) != actual.get(name))
        if args.check:
            if changed:
                parser.exit(1, "Generated catalogs differ: " + ", ".join(changed) + "\nRun python3 tools/regenerate-default-catalogs.py\n")
            print("Default constructor catalogs are up to date.")
        else:
            destination.mkdir(parents=True, exist_ok=True)
            for name in actual.keys() - expected.keys():
                (destination / name).unlink()
            for name, content in expected.items():
                (destination / name).write_bytes(content)
            print(f"Wrote {len(expected)} default constructor catalog files.")


if __name__ == "__main__":
    main()
