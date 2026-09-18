# CGA compiler/backend measurements

`measure.py` compares explicit `(label, source tree, toolchain)` variants with the
same tick/degree and nalgebra consumer used in the preceding experiments. It
prepares a copied snapshot with `scripts/prepare_dynamics_probe.py`; an existing
probe must match the checked-in workload. Its lockfile must retain exactly the
same external package names, versions, sources, and checksums. Local package
nodes and dependency edges may differ, allowing an isolated `whippyunits-num`
backend crate without silently changing third-party dependency versions. The
report preserves each full lockfile hash and local package records. Inputs are
never changed. Python 3.11 or later is required for `tomllib`.

If a candidate adds a local crate, prepare its probe and resolve the candidate
lockfile offline **with the measured feature set enabled** before making the
immutable input snapshot. For the native candidate, use
`cargo +nightly-2026-09-13 metadata --offline --manifest-path probe/Cargo.toml
--features whippyunits/nightly-gca,whippyalgebra/nightly-gca --format-version 1`.
Retain the
existing external dependency versions. The harness itself does not update
lockfiles, and every check still uses `--locked`.

Establish the compiler baseline first:

```sh
python3 scripts/experiments/cga/measure.py \
  --variant stable-typenum /tmp/whippyunits-cga/snapshots/baseline 1.98.1 \
  --variant nightly-typenum /tmp/whippyunits-cga/snapshots/baseline nightly-2026-09-13 \
  --samples 5 \
  --scratch /tmp/whippyunits-cga/baseline-check \
  --output docs/performance-results/cga-baseline-check.json
```

Then compare backends on the **same nightly**. Feature names below are supplied
by the caller; use the exact names implemented by the candidate:

```sh
python3 scripts/experiments/cga/measure.py \
  --variant nightly-typenum /tmp/whippyunits-cga/snapshots/baseline nightly-2026-09-13 \
  --variant nightly-cga /tmp/whippyunits-cga/snapshots/candidate nightly-2026-09-13 \
  --features nightly-cga whippyunits/nightly-gca,whippyalgebra/nightly-gca \
  --samples 5 \
  --scratch /tmp/whippyunits-cga/backend-check \
  --output docs/performance-results/cga-backend-check.json
```

Adding `--prepare-only` copies and validates the fixtures and records commands
and toolchain versions without running compiler jobs. It still requires the
specified toolchains to be installed. Every invocation needs a fresh or empty
scratch directory; a preparation-only directory is not reused for a measured
run. A failed run keeps its logs and completed measurements. Use a new scratch
directory to retry; there is deliberately no partial-run resume mechanism.

The harness alternates forward/reverse variant order. All variants use the same
absolute source path and shared target directory. It cleans typenum,
whippyunits-num when present in the lockfile, whippyunits-core, the procedural
macros, whippyunits, whippyalgebra, nalgebra,
simba, and the consumer before each warmup. After warming that exact dependency
graph, it cleans those packages again and times `cargo check --offline --locked
-j1`, with incremental compilation disabled. Unrelated dependencies remain
warm. It subsequently cleans and times only the consumer; use
`--no-consumer-after` to omit that separate scope.

The report records immutable input/prepared source hashes, identical consumer
body hashes, individual lock hashes and verified external package identities,
probe manifests, exact `rustc -vV` and Cargo versions,
effective environment overrides, commands, individual logs, each timed sample,
and medians. GNU time reports maximum individual-process RSS including
waited-for children, **not the sum of concurrently resident processes**. The
benchmark does not execute numerical assertions; run the same consumer with
`cargo run` separately for correctness before accepting a backend result.

Do not run compiler, test, or editor workloads concurrently with this harness.
Stable-typenum versus nightly-typenum measures compiler/solver changes. Only
nightly-typenum versus nightly-CGA on the same compiler isolates the backend
change. Consumer-only checks and dependency-chain checks answer different
questions; neither is a cold registry build, a runtime benchmark, or a Rust
Analyzer latency measurement. Preserve individual samples and report noise
alongside medians.
