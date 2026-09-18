# CGA experiment log

Work starts from `1838a9d` on `experiments/generic-const-args`.
The preserved performance milestone is `perf/selectable-unit-catalogs` at
`f6f3323`. This log follows the [implementation plan](cga-backend-plan.md).

## 1. Baseline and focused feasibility

The immutable baseline is an archive of `1838a9d` with the established
`scripts/dynamics_probe.rs` and its pinned standalone lockfile. It includes
ticks, degrees, scalar arithmetic and real nalgebra matrix operations. The
work directory is `/tmp/whippyunits-cga`; normal production toolchain selection
remains stable 1.98.1.

| Compiler | Exact version | Backend |
| --- | --- | --- |
| Stable | 1.98.1, 48a229cea, LLVM 22.1.8 | Existing typenum |
| Pinned nightly | 1.100.0-nightly, 809936eac, LLVM 23.1.1 | Existing typenum |

The unchanged nightly baseline passes 207 positive tests: 26 core tests under
each exponent profile, 24 quantity/catalog/storage tests, and 131 algebra
tests. The generic centripetal-acceleration example, no-std algebra check and
the executed scalar/matrix probe also pass. Two catalog diagnostic-snapshot
tests were deliberately skipped because they compare stable compiler wording;
these counts do not include them or doctests. Commands and summaries are in
[the baseline validation record](performance-results/12-cga-baseline-validation.json).

The independent [native feasibility probes](../scripts/experiments/cga/README-probes.md)
pass all 14 compiler commands and execute both accepted consumers. They verify
cross-crate output inference, including an unannotated output; nested generic
composition; exact-root predicates for both dimensions and scales; compatibility
boolean dispatch; canonical concrete types; and forced i16 overflow rejection.
Every expected rejection validates the compiler error codes and rejects
unrelated setup errors or crashes. These are reduced-model results, not yet
production integration results.

One contract difference is explicit: native Add/Sub/Neg use blanket impls, so a
bare trait bound does not prove that evaluating the result will avoid overflow.
Concrete overflowing results reject. Halve uses an evenness predicate on the
impl, preserving rejection even when a bound-only caller never uses its output.

The [measurement driver](../scripts/experiments/cga/README-measure.md) compares
identical sources using explicit compiler selection, a shared source/target
path, alternating order, locked dependencies and isolated compiler jobs.
Dependency-chain and warmed-consumer checks are separate. Only the comparison
between backends on the same nightly can attribute a difference to CGA.

Five alternating samples per compiler measured:

| Measurement | Stable + typenum | Nightly + typenum |
| --- | ---: | ---: |
| Dependency-chain check | 10.774 s | 12.097 s |
| Warmed consumer-only check | 0.156 s | 0.138 s |
| Largest compiler-process RSS during chain check | 472.5 MiB | 484.3 MiB |

The nightly dependency-chain median is 12.3% slower in this workload; the
consumer-only timings are short and their difference is weak evidence. These
are compiler controls, not a CGA result. No other experiment compiler jobs ran
during timing. Raw samples, hashes, versions and commands are in
[the compiler baseline record](performance-results/12-cga-compiler-baselines.json).

A fresh-server nightly editor smoke run resolved the inferred velocity and all
12 constructors, including degrees and ticks. It took 4.36 s to report ready,
608 ms for the first hover and 353 ms for the first completion. One run establishes
functionality only; it is not a latency comparison. See
[the editor baseline](performance-results/12-cga-nightly-editor-baseline.json).
