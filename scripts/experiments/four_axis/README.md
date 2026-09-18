# Four physical axes with full scale conversions

This is a reproducible **minimal-engine experiment**, not an implementation
change to whippyunits. It compares four physical axes (mass, length, time,
angle) with eight physical axes, while keeping all four independent scale
exponents (2, 3, 5, π). The candidate's `Dimension<M,L,T,ANG>` is a genuinely
smaller representation; it does not alias an eight-axis dimension.

The checked-in package contains only scripts, four common source templates,
one shared Cargo lockfile, and baseline source hashes. Preparation retrieves
commit `b45de40f08f9b65277cf4a4a924f3afb7e0cd86d` with local `git archive` into
the external work directory, verifies the relevant input hashes, and generates
both fixtures. No copied repository, generated fixture or target directory
belongs in the performance branch.

Requirements: Python 3.12+, a checkout containing the baseline commit, Rust
1.98.1, and GNU time for Linux RSS measurement. Cargo commands use `--locked
--offline`, so the lockfile's dependencies must already be available locally.
The generated workspaces and both drivers pin Rust 1.98.1 explicitly.

From the repository root:

```sh
python3 scripts/experiments/four_axis/generate.py --repo . --work-dir /tmp/whippyunits-four-axis
python3 scripts/experiments/four_axis/validate.py --work-dir /tmp/whippyunits-four-axis
```

Preparation does not invoke Cargo or rustc. Validation checks the generated
eight- and four-axis engines independently and records `validation.json`,
`validation-*.log`, and `probes/*/*.stderr` under the work directory.

Run benchmarks with other compiler jobs stopped:

```sh
python3 scripts/experiments/four_axis/benchmark.py --work-dir /tmp/whippyunits-four-axis --scope both --repeats 7 --output /tmp/four-axis-small.json
python3 scripts/experiments/four_axis/benchmark.py --work-dir /tmp/whippyunits-four-axis --scope both --stress --repeats 7 --output /tmp/four-axis-stress.json
```

The driver installs both representations at the **same** `active/` source path
and uses **one shared target directory**. Only `representation.rs` changes
between variants; package names, other sources, lockfile, dependencies and
toolchain remain identical. It verifies every common Rust source in both
variants and refreshes its generated active copy before warming up. It
alternates variant order between repetitions,
disables incremental compilation, warms dependencies outside the samples, and
cleans only the selected fixture packages before each timed check.

`--scope both` measures an engine-plus-consumer check and then a consumer-only
check using the engine just built. `--scope consumer` instead builds the selected
engine outside timing before each sample. `--scope engine-consumer` measures
only the first case. These are checks with warmed dependencies; they are not
fresh end-to-end builds or incremental editor checks.

Each sample records wall time and GNU time `%M` maximum RSS in KiB. On Linux
this is the largest individual process RSS in the waited-for process tree,
**not** the sum of simultaneously resident Cargo/rustc processes. Raw samples,
medians, toolchain details and inherited compiler flags are preserved in JSON;
compiler output and raw RSS readings are under `benchmark-logs/`.

## Minimal-engine rust-analyzer probe

Each generated consumer binary also contains identical hover/completion markers
for the repository's direct LSP harness. The hover resolves a velocity inferred
from converted millimetres and 1/60-second ticks; the surrounding code retains
degree/radian conversions, acceleration/force and matrix helper calls. The
consumer library and its compile benchmarks are unchanged by this binary.

All generated engine, consumer and test sources are ordinary Rust files inside
their Cargo crates. Shared templates are copied during preparation rather than
loaded with `include!` or `#[path]` from outside the workspaces. The original
external-include layout produced an unknown velocity in rust-analyzer despite
passing rustc checks; the local layout keeps those definitions within its source
roots. The arithmetic/types and consumer token streams are unchanged. Compiler
measurements should use this same layout as the validated LSP probe, since
removing include expansion can affect checking overhead in both variants.

The `whippyunits` name in this probe is a **local module**, exposing only a
`MetricLength` trait on `f64`. Its `meters()` method constructs this experiment's
quantity type. It is an adapter for the harness's replacement expression, not a
production whippyunits dependency or a measurement of its constructor catalog.
Use `--required-completions meters`; the prototype supplies no other constructor
traits. The glob import is intentional, so this does not test autoimport.

After generation, use a rust-analyzer executable compatible with Rust 1.98.1:

```sh
RUSTUP_TOOLCHAIN=1.98.1 python3 scripts/measure_rust_analyzer.py --server /path/to/rust-analyzer --project-profile eight-axis /tmp/whippyunits-four-axis/eight/consumer --project-profile four-axis /tmp/whippyunits-four-axis/four/consumer --required-completions meters --import-style glob --runs 3 --repeats 5 --output /tmp/four-axis-ra
```

These measurements concern first/cached inference and one local-trait completion
in the minimal engines. They do not establish production whippyunits completion
latency or performance with its full macro/catalog/display implementation.

## What the fixture exercises

Both engines use the pinned whippyunits-core numeral bridge with the narrow
exponent domain. The generated Unit and Quantity Mul/Div implementations keep
the current direct per-exponent `N<const>` obligations and explicit const output
variables. They also retain UnitInv, UnitSqrt and UnitEq. Current float power
tables and dimension-list/indexing/unit-power/uniformity recursion are copied
from the pinned source during generation; nalgebra 0.34.2 supplies real matrix
storage, multiplication and determinant evaluation.

The identical consumer covers:

- Millimetres divided by 1/60-second ticks, mass in grams, acceleration, force,
  and explicit conversion to m/s, m/s² and newtons.
- Degrees represented by π/180, conversion to radians, and angular velocity.
- A 3×3 uniform matrix multiply and determinant with unit powers, and a square
  root that requires even unit exponents.
- A mixed position/velocity state transition with matching inner unit lists
  and compile-time element indexing.
- Unit equality and recursive uniformity proofs. The optional `stress` feature
  checks 96 larger mixed-scale function bodies; this is a synthetic workload.

Validation contains five runtime tests and one positive plus six negative
external compilation probes per representation. Negative probes reject adding
length to time, rescaling incompatible dimensions, treating angle as scalar,
erasing scale through assignment, taking an odd-dimension square root, and
multiplying matrices with incompatible inner unit lists. Both variants passed
these checks before this experiment was packaged. No production performance
claim follows from those correctness checks.

## Limits and integration cost

The minimal Quantity/matrix wrappers omit the production parser and default
catalog, display, integer rescaling, scalar/angular erasure, decomposition
wrappers and most storage polymorphism. Rescaling uses dimension-type equality
and the same float tables on both sides. This isolates a representation
hypothesis; it does not measure whole-library compilation or production
rust-analyzer completion/hover latency.

Binary unit/quantity arithmetic goes from 36 exponent parameters and 12 Add/Sub
obligations to 24 and 8. Unary operations go from 24 parameters and 12 obligations
to 16 and 8; UnitEq folds 8 bits instead of 12. Compiler caching of repeated
zero-axis obligations means these counts are not proportional time estimates.

Production integration would affect Unit/Quantity arithmetic and conversions,
generated declarator and macro output, reflection/formatting, and concrete
whippyalgebra adapters. The parser/catalog's eight-element vectors would need
an explicit checked bridge to compact dimensions. Unsupported axes must be
rejected rather than discarded. Layout, brands, constant patterns, conversions
and negative unit-compatibility tests would need to remain compatible.

A Cargo feature that changes a public type's generic arity also complicates
dependency feature unification. A distinct compact dimension type or separate
mechanics facade with explicit conversion to full units would isolate that API
cost more clearly. This experiment does not choose or implement that migration.
