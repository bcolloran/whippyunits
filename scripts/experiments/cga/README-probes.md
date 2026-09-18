# Native exponent feasibility probes

These are isolated correctness experiments, not production backend code or
performance measurements. Run from the repository root:

```sh
python3 scripts/experiments/cga/probe.py
```

The runner uses `nightly-2026-09-13`, the handoff's five feature gates, and the
read-only cached typenum 1.20.1 source. Override `--typenum` if the registry cache
is elsewhere. Generated programs, binaries, diagnostics, and `results.json` go
to `/tmp/whippyunits-cga/probes`; override `--output` to change this. Every command,
exit status, expected diagnostic, source SHA-256, and compiler version is
recorded. A compiler crash fails the experiment even for an expected rejection.
Expected failures also require every substantive compiler error code to match
the case's allowed set, rejecting unrelated missing-name/import/feature errors.

The library keeps `N<const X: i16>` and associated-output operations. A small
two-exponent `Unit<Dimension, Scale>` model uses the exact explicit output-const
equality bounds used by production `UnitMul` and `UnitSqrt`. It tests the
inference contract without duplicating twelve axes. This model is not evidence
that all production operations or rust-analyzer work with the new backend.

Results on rustc 1.100.0-nightly (809936eac 2026-09-12): all 14 compiler commands
matched the expected outcomes. The two accepted consumer programs also executed
successfully. There were no compiler crashes.

| Probe | Result |
| --- | --- |
| Add/Sub/Neg associated outputs and concrete canonical types | Accepted |
| Existing `N<A>: Add<N<B>, Output=N<O>>` inference | Accepted across crates, including an unannotated result |
| Nested generic arithmetic and generic Unit operation forwarding | Accepted |
| Consumer without nightly feature attributes | Accepted on the pinned nightly compiler |
| Reused arithmetic above the former ±200 bridge limit | Accepted, including 30000 |
| `Bool<EVEN<A>>: True` exact halving | Accepted for positive/negative even values, including i16::MIN |
| Bound-only odd numerator, odd physical exponent, odd scale exponent | Rejected through `Bool<false>: True` |
| Equality to typenum B0/B1, conjunction, uniform/mixed dispatch | Accepted |
| Wrong concrete result or wrong dispatch target | Rejected |
| Forced i16 addition/subtraction/negation overflow | Rejected with E0080 |
| Generic `A+0=A` with no equality evidence | Rejected with E0284 across crates |
| Overflowing addition used only as a trait-availability bound | **Accepted; output was never evaluated** |

The last row is an intentional contract observation. The model's blanket Add
implementation computes a checked constant when its output is evaluated, but
its bare trait bound does not certify representability. In contrast, Halve has
a predicate on the implementation itself; its bare bound rejects odd exponents.
If production promises representability from bare Add/Sub/Neg bounds, it needs
validity predicates on those implementations as well. Do not describe deferred
overflow rejection as equivalent to a partial trait implementation.

The boolean compatibility layer consists of two implementations mapping native
boolean witnesses to typenum's existing bits. It retains typenum as a dependency
and retains the existing `BitAll` fold. It does not yet replace nalgebra's or
whippyalgebra's other typenum use.

The frozen report is [probes/results.json](probes/results.json); its diagnostic
text is embedded so the negative evidence remains reviewable after `/tmp` is
removed. Rerunning writes a new scratch report rather than silently replacing
the frozen result.

## Pinned compiler test-harness crash

The later production integration exposed a separate compiler crash. This
minimal source reproduces it without arithmetic, typenum, metadata, or even a
test body:

```rust
#![allow(incomplete_features)]
#![feature(min_generic_const_args, generic_const_args)]
#[test]
fn trivial() {}
```

```sh
rustc +nightly-2026-09-13 --edition=2024 --test \
  scripts/experiments/cga/probes/test-harness-ice.rs \
  -o /tmp/whippyunits-cga-test-harness-ice
```

This exits 101 with two delayed internal compiler errors: a type-system
constant with a non-valtree type and an unevaluated constant in a MIR body.
An otherwise identical test with no gates, `min_generic_const_args` alone, or
`generic_const_items` alone compiles and runs. The `generic_const_args` gate
alone produces an ordinary diagnostic requiring `min_generic_const_args`.
See [test-harness-results.json](probes/test-harness-results.json) for the six
commands, source hashes, control executions, and full diagnostics. Compiler
crashes here are recorded failures, never successful compile-fail tests.

The narrow workaround is to disable the native library's generated test
harness and exercise it from ordinary external integration-test crates without
these feature attributes. Isolating the native implementation also avoids
applying unstable const handling to unrelated unit-catalog code. Removing the
macroless feature gates does not fix this test-harness crash.

## Minimal editor inference probe

```sh
python3 scripts/experiments/cga/probe_editor.py \
  --server /path/to/rust-analyzer \
  --output /tmp/whippyunits-cga-editor-fresh
```

The output directory must be fresh or empty. `--toolchain` defaults to the
pinned nightly. The script copies only the native numeral leaf into a small
workspace and first requires a successful locked, offline compiler check.
It then queries four hovers via direct LSP. Exit 0 means all resolve correctly;
exit 1 records completed requests with incorrect/unknown inference; exit 2
indicates a compiler/setup/LSP error. There are no latency claims.

On the installed rust-analyzer 0.3.3049, inferred `N::<3> + N::<4>` and
`N::<9> - N::<2>` both show `N<_>`. An explicitly annotated result and the
`N::<7>` constructor show `N<7>`. The compiler accepts the whole consumer.
The annotated control does not demonstrate arithmetic inference; it only
checks that an explicit numeral type can be represented by the analyzer.
This reproduces the problem without Quantity, Unit or the unit DSL macros.
Frozen commands, source hashes and raw hovers are in
[the minimal editor report](../../../docs/performance-results/14-cga-editor-minimal.json).
