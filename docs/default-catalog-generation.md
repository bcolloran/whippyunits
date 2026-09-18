The default constructor catalogs are checked-in Rust files under
`src/generated/default_declarators`. Ordinary builds select those files with
Cargo feature guards, so disabled catalogs are not loaded. The public trait,
alias, method, numeric-conversion, and branding behavior is unchanged.

Regenerate after editing canonical unit metadata or the default generator:

```sh
python3 tools/regenerate-default-catalogs.py
python3 tools/regenerate-default-catalogs.py --check
```

The command requires Python 3 and the pinned Rust toolchain. It invokes a
developer-only ignored test in the proc-macro crate for the complete catalog
and the coherent SI dynamics catalog, then formats the results with rustfmt.
There is no build script or regeneration during consumer compilation.
`--check` compares names and contents without modifying the checked-in files.

Canonical catalog selection, names, scale factors, and conversion metadata are
shared by the snapshots and the procedural macro. The developer-only exporter
compiles direct Rust emitters for the four constructor templates under
`cfg(test)`. Ordinary builds retain the compact macro-invocation emitters for
the procedural entry point, preserving its existing caller-helper contract
without compiling the large developer emitters into the proc-macro library.
Nonmetric groups use sorted keys for deterministic output.
Generated files are partitioned by the physical axes each declaration requires;
composite dimensions require every contributing feature. The coherent dynamics
file is selected only when `units-si-dynamics` is enabled without the broader
`units-dynamics` catalog. Broader catalog features continue to combine additively.

A proc-macro unit test compares the parsed generated declarations against the
selected checked-in declarations for its active features. The catalog feature
matrix exercises that comparison, while public API and compile-fail tests cover
available and deliberately absent constructors.

Literal macros retain their existing generator and hygiene. Local and branded
custom declarators also remain generated, since their user-selected storage
scales and brand types are not fixed catalogs. This change therefore removes
fixed default constructor expansion, rather than every procedural macro use.
