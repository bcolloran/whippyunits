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
| Coherent SI dynamics catalog | Preparing | Additive lean entry point; preserve existing full catalogs and DSL |
| Pre-generated fixed catalogs | Queued | Compare identical declarations before changing catalog size |
| Remove phantom trait bounds | Queued | Preserve layout and numeric behavior |
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
