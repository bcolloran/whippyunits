Implementation experiments, starting from `6196218` on
`perf/selectable-unit-catalogs`.

Each experiment is checked for behavior compatibility and measured against the
preceding retained state. Compiler checks, downstream checks, editor indexing,
and runtime benchmarks are recorded separately. Small improvements may compound;
noise and regressions are recorded rather than counted as wins. Earlier isolated
prototypes are documented in [mks-performance-hypotheses.md](mks-performance-hypotheses.md).

| Experiment | State | Measurement / decision |
| --- | --- | --- |
| Compact rational power tables | Retained | Rebuild/check median 4.458 s → 3.399 s (23.8% lower); runtime lookup improved in all nine measured cases |
| Optional narrow exponent bridge | Retained | Rebuild/check median 3.275 s → 3.008 s (8.2% lower); defaults preserve +/-200, reduced builds use +/-32 |
| Coherent SI dynamics catalog | Retained | Rebuild/check median 3.007 s → 2.114 s (29.7% lower), broad dynamics vs coherent SI conveniences |
| Pre-generated fixed catalogs | Retained for editor correctness | Revised checks effectively unchanged: SI 2.159 s → 2.162 s; broad dynamics 3.064 s → 3.079 s. All ten constructor completions verified |
| Remove phantom trait bounds | Retained as a bound cleanup | Rebuild/check 2.156 s → 2.151 s (0.2%, effectively unchanged); opaque markers now support Clone/Copy/Pod |
| Simplify raw matrix element construction | Queued | Remove redundant twelve-exponent obligations |
| Improve type-level unit powers | Queued | Preserve intermediate-domain behavior or document deliberate limits |
| Separate arithmetic core from parsing | Queued | Measure host and target dependency costs separately |
| Optional numeric conversions and decomposition APIs | Queued | Preserve defaults; benchmark reduced consumers |
| Four-axis coherent representation | Queued | Isolated compatibility and performance prototype |
| Separate branded representation | Queued | Adopt only if measurement justifies extra API machinery |
| Hover scaffolding and incremental-build settings | Queued | Measure editor behavior, including documentation quality |

Production experiments use the performance worktree. The original checkout is
left unchanged. Independent preparation happens in isolated copies; timings and
adoption are sequential so unrelated experiments do not contaminate comparisons.

The first comparison rebuilds `whippyunits-core`, `whippyunits-proc-macros`, and
the root library with `std,units-dynamics`, warm third-party dependencies, and
incremental compilation disabled. It measures `cargo check`, not editor latency
or a cold download/build. See [raw measurements](performance-results/01-rational-check.json).
Reproduce from the experiment's candidate source with:

```sh
python3 scripts/measure_check.py --baseline 6196218 --samples 5 \
  --output /tmp/rational-check.json
```

The rational change replaces 29,700 generated multiplication expressions with
three constant arrays. It keeps the original range, identity fallback, exact
integer results, and configured overflow behavior, including const evaluation.
The full default library/integration suite passed (142 test cases). An additional
639-case comparison against the actual legacy proc macros passed under debug,
release, debug with overflow checks off, and release with checks on. Permanent
CI also checks const success/failure and wrapped values against the old macros.
The maintained const checker passed 34 cases in each of the four configurations.

The release microbenchmark compared the actual old macro output and a verified
snapshot of the new table code in the same compilation/inlining context. Across
three bases and zero/small/wide valid exponent distributions, median times were
10.10–15.89 ns/call before and 2.50–2.99 ns/call after, including identical loop
and checksum overhead. Each case used five alternating-order samples of twenty
million calls per implementation. These are lookup microbenchmarks, not whole
application speedups. See [runtime samples](performance-results/01-rational-runtime.json).

The exponent experiment compares against `ba13451`, retaining the rational
tables and the same dynamics catalog in both variants. Disabling default
features reduces the const/typenum bridge from 1,202 implementations to 194;
`wide-exponents` restores the original range additively and is enabled by
default. Type layouts and the arithmetic algorithms are unchanged. The
selected range applies to both dimension and scale exponents, including every
intermediate operation; the README documents this constraint. See
[raw measurements](performance-results/02-exponents-check.json).
Both profiles passed seven core arithmetic tests and five negative doctests;
public quantity boundary tests passed (two reduced, three wide), along with the
reduced dynamics catalog tests. Workspace feature inspection confirms the editor
helper crates do not accidentally re-enable the wide bridge.

The coherent catalog comparison uses `a2a7c72` with `std,units-dynamics`
against the candidate with `std,units-si-dynamics`. Both use the reduced
exponent bridge and compact rational tables. The new option selects ten
coherent constructor spellings and, when requested, ten literal suffixes.
Other units remain available through the complete expression DSL. Existing
catalog features remain additive. See
[raw measurements](performance-results/03-coherent-check.json).

Validation passed: 145 default library/integration tests, 10 lean catalog tests,
14 lean-with-literals tests, 19 additive full-plus-lean tests, six proc-macro
selection tests, and the reduced-profile `block_roundtrip` algebra example.
Negative fixtures cover disabled aliases, default/local/branded methods, and
literal suffixes. Included tests exercise omitted-unit DSL expansion, inferred
integer conversions, signed zero, local scales, and branding.

The first pre-generation attempt regressed checks in both profiles: SI
2.166 s → 2.426 s, broad dynamics 3.081 s → 3.329 s. That version was not
retained. It compiled four large developer export templates into the normal
proc-macro library. The revised version builds those templates only during
developer tests/regeneration and keeps the existing compact procedural entry
point for compatibility. Raw rejected-attempt measurements:
[SI](performance-results/04-pregenerated-si-check.json),
[dynamics](performance-results/04-pregenerated-dynamics-check.json).

Direct Rust Analyzer diagnostics also exposed a functional problem in the
existing nested catalog macros. Broad dynamics failed to expose `MetricLength`
and `meters`; lean catalogs exposed base constructors but omitted derived
constructors with negative dimension exponents. Recursive expansion reported
`unexpected token in input` / `expected Expr` and left macro placeholders in the
output. This is consistent with the negative-literal bridge issue already
documented by `const_exp`, although individual offending tokens have not been
isolated. Checked-in textual catalogs passed a fresh-server probe for **all ten
constructor methods**, including `newtons`, `joules`, and `pascals`. Diagnostic
timings are excluded because other correctness builds overlapped. See
[recorded evidence](performance-results/04-completion-diagnosis.json).

After excluding developer emitters from ordinary compilation, five interleaved
samples per variant measured +0.1% SI and +0.5% broad dynamics median check time,
within the run-to-run spread. This is **not a demonstrated compile-time win**;
the reason to retain it is reliable constructor visibility in Rust Analyzer and
ordinary source navigation. The rejected 8–12% regressions are absent in the
revised measurements. See [SI samples](performance-results/04-revised-si-check.json)
and [dynamics samples](performance-results/04-revised-dynamics-check.json).

The cost is approximately 1.5 MB of checked-in generated Rust, of which the lean
catalog selects about 16 KB. Nine feature-selection comparisons, deterministic
regeneration, default/lean/literal public tests, and full-catalog no_std/no_alloc
checks pass. The integrated full library/integration suite passed all 145
tests. Independent comparison verified that the four exported constructor
templates preserve their original method bodies, numeric casts, and operation
order. See [regeneration instructions](default-catalog-generation.md).

The phantom-bound change manually implements `Clone` using only the stored
value, makes `Copy` depend only on `T: Copy`, and removes the unnecessary marker
`Clone` requirements from `Pod`. Layout and required `'static` bounds are
unchanged. Derived `PartialEq` stays in place: replacing it with a manual
implementation would lose Rust's structural equality marker used by constant
patterns. Twenty integrated tests passed, including unconstrained markers,
non-Copy storage, branded arithmetic, Pod views, and constant patterns; the
five new cases also passed with default features disabled in isolation.
The measured 0.2% difference is noise, not a demonstrated compile-time gain.
See [raw measurements](performance-results/05-phantom-bounds-check.json).
