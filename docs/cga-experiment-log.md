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

## 2. Native backend integrated, with compiler and editor limitations

The opt-in `nightly-gca` feature is forwarded through whippyalgebra, whippyunits,
proc macros and core. Core selects a small optional `whippyunits-num` crate;
its stable `num` implementation is unchanged. The native leaf implements
Add/Sub/Neg/Halve/NumEq using six named generic constants, plus two boolean
adapters. It keeps typenum B0/B1 compatibility and nalgebra's independent
use of typenum. This does not remove typenum from the dependency graph.

The production implementation uses only `min_generic_const_args`,
`generic_const_args`, and `generic_const_items`, with explicit
`core::direct_const_arg!` at the seven arithmetic/predicate uses. These feature
gates are confined to the leaf. The default leaf is empty so ordinary stable
workspace builds do not require nightly; downstream stable builds do not
activate the optional dependency at all. The public `whippyunits_core::num::*`
paths stay intact. Native mode changes the defining crate/type identity of
those items and omits the hidden typenum bridge traits; it is an alternative
backend, not an interoperability layer between numeral implementations.

Integration exposed two problems absent from the initial arithmetic probes:

1. Enabling all five handoff gates in core changes the interpretation of
   unrelated floating-point, prefix and metadata constants, producing ordinary
   compilation errors. Removing the const-item macroless gate fixes those
   ordinary errors but does not fix the test-harness failure below.
2. A trivial `#[test] fn trivial() {}` crashes this compiler when the crate has
   `min_generic_const_args` plus `generic_const_args`. No unit code, typenum or
   arithmetic is needed. The leaf disables its library test harness and puts
   its tests in an ordinary external consumer crate. The core/root/algebra
   test harnesses have no CGA gates and run normally.

The failed attempts are preserved in [gate investigation](performance-results/13-cga-gate-investigation.json).
The [minimal test-harness reproducer and control results](../scripts/experiments/cga/README-probes.md#pinned-compiler-test-harness-crash)
make the compiler issue reproducible without this repository.

Native validation passes 194 core/quantity/algebra tests, the generic example,
no-std check and executed dynamics probe. The native leaf adds eight arithmetic
tests and an odd-root compile-fail doctest. The different core test counts
reflect moving numeral tests to the external leaf suite. Stable core still
passes all 26 narrow-profile tests, and the empty leaf compiles on stable.
See [production validation](performance-results/13-cga-native-validation.json).

An independent real-library consumer passes all twelve cases: three executed
positive fixtures and nine rejection checks covering odd dimension/scale
roots, Cholesky, incompatible dimensions/gauges/brands, and forced i16
Add/Sub/Neg overflow. Its typenum control passes the eight applicable cases.
Every rejection verifies the diagnostic codes and meaning, excluding setup
errors and crashes. External dependency versions and checksums are identical;
only the local leaf package/edge changes. Reports:
[typenum](performance-results/13-cga-integration-baseline.json),
[native](performance-results/13-cga-integration-native.json).

The first native rust-analyzer smoke check **fails inferred-unit correctness**:
velocity has a Quantity shape but every exponent displays `_`. Rustc compiles
and executes the same consumer correctly. Do not treat the fast unknown hover
as a performance improvement. Native remains an opt-in compiler experiment;
the stable backend remains the editor-safe default while this is investigated.

Replacing the explicit builtin with plain named-constant references and the
macroless argument gate also fails the same hover check after a successful
Cargo warmup. This bounds the result: the installed analyzer cannot infer
these native associated outputs under either tested syntax. Completion timing
was deliberately not attempted after hover correctness failed. The two raw
hover responses are embedded in [editor smoke evidence](performance-results/13-cga-editor-smoke.json).
