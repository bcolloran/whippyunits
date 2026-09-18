#!/usr/bin/env python3
"""Add the fixed tick/degree + nalgebra workload to an isolated repo snapshot.

Requires the units-dynamics-conversions feature. The standalone consumer has
default features disabled; no other workspace member can broaden its catalog.
Copies the dependency lockfile used in the recorded comparisons. Vendor source
patches need a lockfile refresh (cargo metadata --offline); retain all versions.
"""

import argparse
from pathlib import Path
import shutil


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("snapshot", type=Path)
    args = parser.parse_args()
    destination = args.snapshot.resolve() / "probe"
    if destination.exists():
        parser.error("probe already exists; use a fresh snapshot")
    (destination / "src").mkdir(parents=True)
    shutil.copy2(Path(__file__).with_name("dynamics_probe.rs"), destination / "src/main.rs")
    shutil.copy2(Path(__file__).with_name("dynamics_probe.Cargo.lock"), destination / "Cargo.lock")
    (destination / "Cargo.toml").write_text('''[package]
name = "dynamics_probe"
version = "0.0.0"
edition = "2024"
[workspace]
[dependencies]
whippyunits = {path="..", default-features=false, features=["std", "units-dynamics-conversions"]}
whippyalgebra = {path="../whippyalgebra", default-features=false, features=["std", "nalgebra", "units-dynamics-conversions"]}
''')
    print(destination)


if __name__ == "__main__":
    main()
