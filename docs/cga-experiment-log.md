# CGA experiment log

Work starts from `1838a9d` on `experiments/generic-const-args`.
The preserved performance milestone is `perf/selectable-unit-catalogs` at
`f6f3323`. This log follows the [implementation plan](cga-backend-plan.md).

Current decision (2026-09-18): the stable default remains unchanged. The native
backend is implemented behind `nightly-gca`, but the installed rust-analyzer
cannot infer its computed exponents. The separate quantity-trait rewrite stays
as an unapplied experiment because it regresses the stable check workload.

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

## 3. Quantity operators through existing unit traits

The second candidate replaces the private `emit_dimension_combining!` macro
and its two uses with ordinary quantity Mul/Div implementations. Each operation
uses four type parameters and delegates unit arithmetic to UnitMul/UnitDiv,
instead of repeating 36 const parameters and twelve exponent constraints.
The underlying Unit implementation still performs the full twelve-component
arithmetic; this does not remove physical axes or scale conversions.

Storage and brand remain identical on both operands and the result. There are
no new Copy, Clone or NumCast requirements. Four new tests exercise abstract
unit bounds, owned non-Copy storage, brands, scalar/quantity coherence, and
millimeter/tick scale cancellation. Default stable tests (46) and all root
examples pass; the native version passes 54 targeted tests. Independent probes
still reject mismatched storage and brands. The old exported hidden helper
macro is retained for source compatibility. The new ordinary implementations
also permit custom unit types implementing the existing unit-operation traits.

Five interleaved samples per variant on the same pinned nightly give:

| Variant | Dependency-chain check | Warm consumer check | Chain max process RSS |
| --- | ---: | ---: | ---: |
| Reduced typenum baseline | 11.868 s | 132.7 ms | 483.8 MiB |
| Quantity traits | 11.814 s | 130.5 ms | 484.4 MiB |
| CGA | 11.801 s | 129.3 ms | 484.2 MiB |
| CGA + quantity traits | 11.663 s | 125.5 ms | 484.1 MiB |

The dependency-chain medians change by -0.5%, -0.6%, and -1.7%, respectively.
Individual ranges overlap: baseline 11.669–12.083 s, traits 11.638–11.969 s,
CGA 11.554–11.894 s, combined 11.613–11.901 s. The warmed consumer medians
change by -1.6%, -2.5%, and -5.4%, but these are only 2–7 ms differences.
There is no meaningful process-memory reduction. On this nightly, the results support simpler
code without a detected large compiler regression; they do not establish a
large speedup or guarantee that gains compound in a bigger application.

All measured builds succeeded with the identical dynamics/matrix source,
locked external dependencies, jobs=1, incremental compilation disabled and no
competing experiment compiler/editor jobs. This is a selected dependency-chain
check, not a fully cold build. Raw data and reproduction commands are in
[the four-variant report](performance-results/14-cga-backend-comparison.json).

On stable 1.98.1 with the installed rust-analyzer 0.3.3049, three fresh-server
runs per variant all resolve the inferred velocity and all twelve constructor
completions, including degrees and ticks:

| Editor measurement | Typenum baseline | Quantity traits |
| --- | ---: | ---: |
| Ready | 3.876 s | 3.851 s |
| First inferred hover | 528.6 ms | 521.9 ms |
| Repeated hover | 0.471 ms | 0.458 ms |
| First completion | 355.3 ms | 356.2 ms |
| Repeated completion | 19.3 ms | 18.6 ms |
| Analyzer peak RSS | 1700.1 MiB | 1694.6 MiB |

This small sample shows preserved editor functionality and essentially unchanged
latency, not a substantial editor speedup. Measurements use direct LSP, warm
Cargo artifacts, glob-imported constructors and no VS Code formatter/proxy;
they do not measure macro auto-imports or a large application's edit cycles.
See [editor report](performance-results/14-quantity-traits-editor.json).

A follow-up with the actual stable compiler changes the adoption decision.
Three interleaved samples on 1.98.1 give:

| Stable measurement | Existing implementation | Quantity-trait candidate |
| --- | ---: | ---: |
| Dependency-chain median | 10.618 s | 10.935 s (+3.0%) |
| Dependency-chain range | 10.503–10.700 s | 10.785–11.092 s |
| Warm consumer median | 142.3 ms | 144.4 ms (+1.5%) |

All three candidate chain samples are slower than their paired controls, and
the ranges do not overlap. This small experiment does not establish the cause
or predict every project, but it is enough evidence to **hold adoption** under
the no-regression requirement. No compiler/editor workloads overlapped these
measurements. [Raw stable results](performance-results/14-quantity-traits-stable.json).

The production quantity implementation was restored exactly. The candidate,
including its four meaningful tests, is preserved as
[quantity-traits.patch](../scripts/experiments/cga/quantity-traits.patch).
Apply it only in a separate checkout with `git apply`; it works against the
current experiment branch. Both its successful correctness/editor results and
its compiler-dependent timing behavior remain useful evidence. The small
nightly gains are not a reason to impose a measured stable slowdown.

These comparisons use the previously reduced dynamics catalog and ±32 typenum
input profile. They do not measure the difference from an unmodified upstream
build with every unit and the wide ±200 bridge.

The release-profile positive full-i16 program also executes successfully; forced
Add/Sub/Neg overflow each rejects solely with E0080. Checked-overflow behavior
is therefore verified in release as well as the debug checks above.
[Release contract report](performance-results/14-cga-release-contract.json).

Final broad native root validation also passes: 155 tests, zero failures or
compiler crashes; two active exact-text diagnostic snapshot tests are excluded.
The independent semantic rejection suite covers the backend-specific negative
contracts without relying on compiler-version-specific wording. See
[final native validation](performance-results/14-cga-final-native-validation.json).

The final unchanged-default stable build passes all root tests and doctests:
435 passed, 13 ignored; all 13 embedded trybuild cases pass without updating
snapshots. Core doctests pass all five cases under both narrow and wide
profiles. [Final stable validation](performance-results/14-cga-final-stable-validation.json).

The editor failure is now reduced to the numeral leaf alone:
`let sum = N::<3> + N::<4>` and `let difference = N::<9> - N::<2>` both
hover as `N<_>`, while explicit `N<7>` annotations/constructors display
correctly. The compiler accepts this small consumer. The reproducible
[editor probe](../scripts/experiments/cga/README-probes.md#minimal-editor-inference-probe)
records incorrect inference as exit 1 and setup/compiler failures separately.
[Raw minimal evidence](performance-results/14-cga-editor-minimal.json).

## Next experiments, in order

1. Use the reduced native inferred-hover reproducer to track analyzer support
   before considering the native backend for everyday editing. Explicit syntax
   and macroless syntax both fail the tested full-library hover.
2. Profile the stable trait rewrite's extra work before accepting it. An added
   associated-output normalization is a plausible hypothesis, not an established
   cause. Narrow the change to one operator or preserve more concrete Self type
   structure, then rerun identical compiler/editor workloads.
3. Once the backend/editor boundary works, separately compare four physical
   axes and a smaller boolean adapter. Retain degree and tick scale arithmetic;
   do not remove conversion machinery to manufacture a faster baseline.

No default backend switch, dimensional representation change, typenum vendoring,
or upstream rollback was needed for this first pass.
