# Typenum and four-axis dynamics experiments

The requirements for this round are mass, length, time and angle, including
degrees and a fixed tick of 1/60 second. All variants retain conversion scale
information. The production catalog added in `3eb0f11` provides the ten coherent
dynamics constructors plus `Degree`/`.degrees()` and `Tick`/`.ticks()` through
`units-dynamics-conversions`. The existing broader features remain additive.

These experiments follow the earlier [performance work](performance-experiments.md).
Their workload includes nalgebra and conversions, so their absolute times must
not be compared with the earlier three-crate/scalar-only measurements.

## Variants and attribution

| Variant | Change relative to its control |
| --- | --- |
| `control` | Production conversion catalog, narrow exponents, existing typenum backend and const-generic mappings |
| `mapping` | Makes the unused typenum const-generic mapping feature optional for nalgebra builds |
| `direct` | Starts from `mapping`; replaces reduced signed arithmetic with 12,773 explicit impls |
| `full` | Starts from `mapping`; patches in an unchanged local copy of typenum 1.20.1 |
| `constants` | Starts from `full`; retains U0..400 and P/N1..400, removes larger aliases and const mappings |
| `modules` | Starts from `full`; disconnects arrays, tuple implementations and the `op!` DSL |
| `slim` | Combines `constants` and `modules` |

The vendor control is necessary: switching a dependency from a registry source
to a local path must not be counted as a pruning benefit. Every vendor variant
uses one `[patch.crates-io]` package identity throughout the consumer graph;
no global Cargo registry was edited. The active nalgebra profile already omits
the const mapping feature, so removing its inactive mapping declarations does
not explain a difference between vendor variants.

The constants experiment reduces 3,440 aliases to 1,203 and that source file
from 280,251 to 64,786 bytes. The retained range also accommodates the existing
wide exponent outputs (±400) and nalgebra's U0..127 dimension bridge. It is an
intentional typenum API restriction, not an upstream-compatible replacement.
The underlying binary integer representations and arithmetic algorithms remain
unchanged. The removed modules are not used by the measured graph; numerical
division, remainder, comparison and related helpers are retained.

The direct arithmetic table contains 4,225 additions, 4,225 subtractions,
4,225 equality decisions, 65 negations and 33 even-only halvings. It preserves
the public `N<const X>` result types and the existing ±32 input/±64 output
contract. The original wide backend remains selected by `wide-exponents`.
The generated table occupies 1,284,593 bytes / 85,157 lines; pregenerating it
removes expansion work but still leaves the compiler and editor all those impls.

## Measurement method

[The consumer](../scripts/dynamics_probe.rs) exercises tick-based velocity,
acceleration and force, degrees and angular acceleration, scalar rescaling,
and a real 3×3 uniform matrix with rescaling, dot product, norm and determinant.
Runtime assertions verify conversion values separately from `cargo check`.

[The check harness](../scripts/measure_variants.py) installs immutable snapshots
at one fixed source path with one shared target directory, alternates variant
order, disables incremental compilation, and uses `-j1`. Each dependency-chain
sample cleans typenum, core/proc-macro/root units, whippyalgebra, nalgebra, simba
and the consumer. Unrelated third-party dependencies are warm. A consumer-only
sample cleans just the probe after building its exact dependency graph.
These are check times, not release builds or cold downloads.

GNU time records maximum individual-process RSS including waited-for children;
it does not report the sum of simultaneously resident processes. This matters
because nalgebra can remain the largest compiler process even when another
crate becomes smaller. Rust Analyzer measurements instead sample the server's
own RSS every 50 ms, excluding Cargo and proc-macro child processes.

Editor measurements use fresh direct LSP servers with warm Cargo artifacts and
cache priming. Each run checks an inferred velocity hover and validates usable
constructor completion edits. Degrees and ticks are required alongside the ten
coherent constructors. First requests and repeated requests are reported
separately; these measurements exclude VS Code and the formatter extension.
Existing `--project-profile` probes retain their source imports; `--import-style`
does not rewrite them. The recorded runs use glob imports, not autoimport tests.
The maintained harness now enforces `--locked` for existing profiles and includes
lockfiles/toolchain inputs in its source fingerprint. Those safeguards were
added after the recorded runs; the measured variants used their pinned versions.

## Reproduction

Retain snapshots and their lockfiles. Start with the `3eb0f11` conversion commit,
then apply [optional-const-mappings.patch](../scripts/experiments/optional-const-mappings.patch)
for the second snapshot. Its [documentation patch](../scripts/experiments/optional-const-mappings-docs.patch)
records the backend-free feature migration. These patches are experimental;
neither is applied to the production library. Apply the direct table patch to
that mapping snapshot to reproduce the third variant without checking its large
generated source into production:

```sh
git apply /path/to/scripts/experiments/direct-exponents.patch
python3 tools/generate-direct-exponents.py
```

Run [prepare_dynamics_probe.py](../scripts/prepare_dynamics_probe.py) on each
snapshot. It copies the identical workload and dependency lockfile. Run
[prepare_typenum_experiment.py](../scripts/prepare_typenum_experiment.py) against
the mapping snapshot and an untouched typenum 1.20.1 source directory to prepare
the full/constants/modules/slim copies. Refresh only the vendor source entries
with `cargo metadata --offline --format-version 1 --manifest-path probe/Cargo.toml`
before using `--locked` checks, and verify other dependency versions are unchanged.

The raw check reports record source hashes, commands, samples and scratch paths.
Use `--reset` to identify changed dependency packages before a consumer-only
comparison; copying older source timestamps over a shared target without that
reset can otherwise reuse stale artifacts. `--consumer-after` obtains warmed
consumer measurements directly after each dependency-chain sample.

The [four-axis package](../scripts/experiments/four_axis/README.md) reconstructs
its pinned `b45de40` source inputs and generates both representations from one
template. It includes runtime/negative validation and a separate benchmark
driver. It retains the four scale parameters and changes actual physical
dimension arity from eight to four. It deliberately measures two minimal
engines rather than claiming that the production library has been converted.

## Results

All times below are medians. Negative percentages mean less time. Comparisons
use each experiment's own control; percentages cannot be added together.

### Mapping feature and explicit arithmetic

Five alternating samples per variant measured the selected dependency-chain
check. Three additional samples measured the warmed consumer alone.

| Variant | Dependency-chain check | Change against its control | Consumer-only check | Chain maximum process RSS |
| --- | ---: | ---: | ---: | ---: |
| Existing backend | 10.833 s | — | 0.172 s | 472.7 MiB |
| Optional const mappings | 10.479 s | −3.3% | 0.176 s | 472.8 MiB |
| Explicit arithmetic table | 20.049 s | +91.3% vs optional mappings | 0.182 s | 621.3 MiB |

The small consumer differences are too short and noisy to establish an improvement.
The direct table nearly doubles dependency checking. In the three-run editor
comparison it also increases first hover from 777 to 1,413 ms, first completion
from 448 to 990 ms, and final server RSS from 1,701 to 2,373 MiB. **Rejected.**
Expanding the arithmetic impl set costs substantially more than the recursive
binary arithmetic it replaces in this workload.

Optional const mappings showed a modest compiler benefit, but a focused
five-run editor follow-up measured first hover at 515 → 540 ms (+4.9%) and first
completion at 325 → 333 ms (+2.6%). Indexing and memory were essentially unchanged.
The initial three-run campaign also had slower first hovers in every paired run.
The feature would require backend-free users with defaults disabled to opt into
`standalone-indexing`, although normal defaults preserve that API. **Not adopted:**
there is no demonstrated editor benefit to justify that migration here.

Raw evidence: [dependency checks](performance-results/07-backends-check.json),
[consumer checks](performance-results/07-backends-consumer-check.json),
[initial editor comparison](performance-results/09-typenum-rust-analyzer.json),
[focused mapping follow-up](performance-results/09-mapping-rust-analyzer-repeat.json).

### Slim typenum

Five alternating samples per variant:

| Vendored variant | Typenum-only check | Typenum maximum process RSS | Dependency-chain check | Consumer-only check |
| --- | ---: | ---: | ---: | ---: |
| Unchanged | 0.500 s | 139.3 MiB | 10.912 s | 0.208 s |
| Fewer constants | 0.405 s | 125.4 MiB | 10.618 s | 0.188 s |
| Fewer modules | 0.427 s | 128.4 MiB | 10.582 s | 0.165 s |
| Both reductions | 0.326 s | 115.1 MiB | 10.311 s | 0.177 s |

The slim crate itself checks 34.9% faster with 17.4% less maximum process RSS.
The whole dependency-chain median improves 5.5%, with substantial sample noise;
its largest process still uses about 473 MiB. These samples do not establish a
stable whole-application improvement of that size.

Three editor runs per vendor variant measured first hover at 670 → 539 ms and
first completion at 368 → 344 ms. Final server RSS changed from 1,705.2 to
1,693.5 MiB: only 11.7 MiB, or 0.7%. Those latency samples are suggestive, but
too few and variable to promise a dependable improvement. Removing source
aliases does not shorten the binary integer types used by arithmetic.

**Kept as a reproducible experiment, not a production fork.** There is a real
typenum compilation saving, but the editor memory reduction is small, and
removing public APIs introduces maintenance and compatibility costs. The
generator preserves upstream license files and leaves the registry untouched.

Raw evidence: [typenum checks](performance-results/08-typenum-only-check.json),
[application checks](performance-results/08-vendor-check.json),
[editor comparison](performance-results/09-typenum-rust-analyzer.json).

### Four physical axes, retaining conversions

This candidate really uses `Dimension<M, L, T, ANG>`. Both sides retain four
scale exponents, storage type and brand. It reduces binary arithmetic from
36 exponent parameters / 12 Add-or-Sub obligations to 24 / 8. Repeated zero-axis
proofs may be cached, so those counts alone do not predict the performance gain.

Seven alternating samples per variant, using the final ordinary-file layout:

| Check workload | Eight axes | Four axes | Change |
| --- | ---: | ---: | ---: |
| Small engine + consumer | 0.216 s | 0.196 s | −9.2% |
| Small consumer only | 0.128 s | 0.112 s | −12.4% |
| 96-function stress engine + consumer | 1.500 s | 1.185 s | −21.0% |
| 96-function stress consumer only | 1.423 s | 1.129 s | −20.7% |

The heavier workload is the stronger result: every four-axis sample was faster,
and it retains scalar conversions, matrix multiplication, determinant, square
root and recursive unit proofs. Maximum process RSS differs by only about
1–2 MiB. The small workload has very short absolute times.

Three direct editor runs per variant measured:

| Minimal-engine editor measurement | Eight axes | Four axes |
| --- | ---: | ---: |
| Ready/indexing | 3.785 s | 3.793 s |
| First inferred velocity hover | 49.3 ms | 43.9 ms |
| First usable local constructor completion | 829 ms | 728 ms |
| Final server RSS | 1,521.1 MiB | 1,520.3 MiB |

The hover median improved even though the candidate was slower in two of three
paired runs; these editor results are not a reliable speedup estimate. Indexing
and memory are effectively unchanged. The constructor is a local adapter, not
the full production catalog, and this fixture omits production parsing,
display, integer rescaling, erasure and most matrix adapters. **Working prototype,
not adopted into production.** Its compiler result supports further integration
work; it does not establish a 21% improvement in the user's application.

The first fixture layout shared files outside its Cargo source roots. Rustc
accepted it, but Rust Analyzer returned an unknown inferred velocity. The
harness rejected those responses rather than counting their timings. Both
representations now use ordinary files within their crates; compiler benchmarks
were rerun for that same layout. The invalid editor run and original compiler
runs remain recorded for provenance, not as current results.

Production integration needs an explicit bridge from the full metadata and
macro output to compact dimensions, with unsupported axes rejected. A separate
compact representation avoids having Cargo feature unification change the
generic arity of the existing public `Dimension` type. The production library
still uses eight physical axes; selecting the lean catalog alone does not
remove them.

Raw evidence: [small checks](performance-results/10-four-axis-small.json),
[stress checks](performance-results/10-four-axis-stress.json),
[valid editor runs](performance-results/10-four-axis-rust-analyzer.json),
[rejected editor layout](performance-results/10-four-axis-rust-analyzer-rejected-layout.json).
See the [prototype README](../scripts/experiments/four_axis/README.md) for the
exact supported operations and integration limits.

## Validation and retained changes

The production addition is `units-dynamics-conversions`: select it with
`default-features = false` and `features = ["std", "units-dynamics-conversions"]`;
add `"nalgebra"` for whippyalgebra. It retains the ten coherent dynamics
constructors plus degrees and ticks. Tick is exactly 1/60 SI second. Other
conversion scales remain available through the unit-expression DSL. This is a
functional extension of the earlier reduced catalog, not a separately measured
performance improvement over the ten-constructor profile.

Validation passed 17 production feature/snapshot/regeneration commands, the
mapping and direct-backend compatibility checks, and numerical execution of the
identical scalar/matrix probe in all seven backend/vendor variants. Each final
four-axis/eight-axis fixture passed five runtime tests, compilation of 96 stress
functions, one positive and six negative compile probes, and its consumer binary.
All valid production editor runs exposed all 12 required constructor methods.
Commands, exit codes and source hashes are in the
[validation record](performance-results/11-experiment-validation.json).
