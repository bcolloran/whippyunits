# Generic const arguments backend: investigation and plan

Date: 2026-09-18. This is analysis and a proposed experiment sequence, not an
implemented backend or a new performance result.

The branch `experiments/generic-const-args` starts at `f6f3323`.
`perf/selectable-unit-catalogs` remains at that performance milestone. The
original `main` checkout is unchanged. Retain the current representation and
degree/tick conversions initially; investigate four physical axes separately.

## Findings from the handoff

The source investigation is recorded in the typenum checkout at
`/data/code_projects/rust/typenum/experiments/const-generics/WHIPPYUNITS-HANDOFF.md`.
Its experiments are now committed at `608c892` on
`experiments/nightly-const-generics`; the handoff's warning that they were
untracked is stale. This planning pass inspected the examples and recorded
results, but did not rerun their 890 programs or integrate them into whippyunits.

The recorded suite has 779 executed successes, 108 expected rejections and
three compiler crashes. The crashes concern invalid typenum reference programs,
not the native replacements. The supplementary const-backed operator example
shows associated outputs and nested generic composition; separate cases cover
signed i16 arithmetic. These are meaningful feasibility results, with no
compiler or editor performance claim.

Use the installed `nightly-2026-09-13` first. The working example uses
`min_generic_const_args`, `generic_const_args`, `generic_const_items`,
`macroless_generic_const_args`, and `macroless_const_item_generic_const_args`.
Treat these as the tested combination, not a permanent minimum gate set. The
public feature is usually abbreviated GCA in Rust discussions; CGA in this
document's filename follows the project's existing terminology.

## Historical implementation and upstream intent

| Event | Commit | Significance |
| --- | --- | --- |
| Stable polyfill introduced alongside CGE | `0e527beff5b58b810f2a70b2271f180fb0defc9b` | Feature forwarding and separate stable/nightly arithmetic paths |
| Last upstream state before removal | `33a48761380a27b8264644eb3cd73166942ddc40` | Reference source for the complete old CGE implementation |
| CGE removal / initial algebra draft | `e1bcd26bd394ce6177e9ca11d3919303ec0969cd` | Removes gating and adds today's Unit/algebra requirements |
| Storage-generic port to new API | `585c5defec2225ed9f911b4dfa22c9e55c2ef03e` | Explicitly omits the removed CGE arm |

The old root feature `cge` forwarded to core. Small build scripts emitted
`cfg(has_generic_const_exprs)` and crate attributes enabled the nightly feature.
Core's `num.rs` selected between typenum and direct `A+B`, `A-B`, `-X` outputs
with evaluatability bounds. Root `api.rs` and `arithmetic_quantity_types.rs`
also selected backend-specific macro arms. It was more than one swappable
numeric module: nightly and stable operator generation were partly duplicated.

Removal also introduced `NumEq`, `Halve`, `BitAll`, `UnitEq`, `UnitSqrt` and
whippyalgebra. The previous CGE path never implemented that complete surface.
Restoring it would not supply equality dispatch or exact-root trait evidence.

The direct upstream rationale found is the changelog's plan to revisit a
full-i16 implementation after GCA stabilization. The removal commit provides
no detailed compiler limitation or linked design discussion. Earlier fork
commits report successful CGE tests. There is insufficient evidence to attribute
a specific motive or a known fatal GCA blocker to the author.

There is relevant compiler context: Rust's new trait solver does not support
GCE, and Rust intends GCA to replace it. The current transition still provides
an old-solver fallback for GCE. This explains why continuing to invest in GCE
is unattractive; it does not establish the upstream author's reasoning.
[Rust solver transition](https://github.com/rust-lang/rust/issues/160895).

**Recommendation: keep the current base.** Recover ideas with `git show`, not
by rolling back the Unit redesign, storage support, algebra, or performance and
editor work. The surviving interfaces are sufficient to restore backend selection.

## Proposed architecture

Keep `N<const X: i16>` and its `Add`, `Sub`, `Neg`, `NumEq`, `Halve`, `BitAll`
interfaces in [core num.rs](../whippyunits-core/src/num.rs). Move implementations
into separate stable and GCA modules. Put each native operation in one named
generic constant, reused throughout the backend. The proposed native addition
has the shape `type Output = N<{ ADD::<A, B> }>`.

Preserve the associated-output unit interfaces in [unit.rs](../src/unit.rs):
`UnitMul`, `UnitDiv`, `UnitInv`, `UnitEq`, and `UnitSqrt`. Preserve the user DSL,
dimension/scale wrappers, storage types, brand semantics and conversions.

Start with an opt-in `nightly-gca` feature, forwarding it deliberately through
root, core, proc macros and algebra. Core participates in both host and target
graphs. Without the opt-in, defaults retain stable behavior. `nightly-gca`
selects native implementations; otherwise select stable implementations with cfg.
Do not reintroduce the old custom build-script cfg merely for feature selection.
Cargo feature unification is additive: implementation selection and removing
the typenum dependency must be tracked separately. Initially retain compatibility
booleans even if that keeps typenum compiled.

Most quantity code should depend only on Unit operation traits. If exact
normalization tests force native Unit implementations, confine backend-specific
code there instead of duplicating the quantity-operator macros again.

## Limitations that determine feasibility

GCA computations use named constants and definitional equality while generic;
they can evaluate when arguments become concrete. The feature remains incomplete,
and equality/coherence design questions remain open.
[Tracking issue](https://github.com/rust-lang/rust/issues/151972),
[unstable book](https://doc.rust-lang.org/nightly/unstable-book/language-features/generic-const-args.html).

| Boundary | Evidence / implication | Required probe |
| --- | --- | --- |
| Generic identities | The handoff rejects generic `ADD<N,0> = N`, commutativity, reassociation and separately named identical formulas; concrete results canonicalize | Keep associated outputs and scalar `Output = Self`; test actual generic clients |
| Output inference | Inverting `ADD<N,1> = 5` fails; existing whippyunits generally infers an output from known input exponents | Test the exact `N<A>: Add<N<B>, Output=N<O>>` pattern, nested and across crates |
| Exact roots | A delayed assertion in HALF is weaker than the current partial trait implementation | Gate Halve with a true-only evenness witness and propagate it through UnitSqrt/Cholesky |
| Equality dispatch | Current B0/B1 values choose actual uniform or mixed matrix types | Test native comparison, conjunction and AutoReduce/GaugeReproduces together |
| Tooling | Successful rustc compilation says nothing about this RA server's support | Require resolved inferred hovers and usable completions on the pinned nightly |
| Checked arithmetic | i16 operations can overflow; unused generic declarations may defer evaluation | Force concrete results, build/run code, and reject overflow and negating i16::MIN |

Lack of inverse inference is not automatically a blocker for `Output=N<O>`:
computing O from known A and B is forward evaluation. It does become a problem
if a generic caller expects the compiler to reconstruct an operand from the
result, or if opaque associated outputs fail to normalize into current bounds.

The handoff's typenum example proves `U0 + N = N` for unknown unsigned N using
a special impl. That does not prove current whippyunits depends on the same
reduction through its const-wrapper bridge. Inventory real dependencies on
generic identities before designing a replacement symbolic algebra engine.
Do not add overlapping zero-special-case impls beside a blanket Add impl.

For exact roots, aim for a local `Bool<{ EVEN::<X> }>: True` bound with only
`Bool<true>` implementing True. Test odd *scale* exponents as well as odd
physical dimensions. A bound-only caller that never uses the output must still
reject a concrete invalid root. Apply the same trait-availability scrutiny to
any overflow/domain predicates we claim provide validity evidence.

## How small a polyfill could be

| Need | Small candidate | Limit |
| --- | --- | --- |
| Compatibility equality outputs | Two mappings from `Bool<true/false>` to B1/B0 | Still retains a typenum dependency |
| Native boolean dispatch | Two boolean witnesses and a few AND/selection impls | Must migrate public constraints and algebra dispatch |
| Exact-root gate if predicate normalization fails | A finite even-exponent witness table for the chosen domain | Root domain remains deliberately bounded; generic propagation still needs tests |
| Stable exponent arithmetic | Keep the current working backend initially | Replacing it is a separate experiment |

Avoid the previously rejected all-pairs arithmetic table: its 12,773 impls
regressed compiler and editor performance. A small validity witness table is
different from generating every addition/subtraction pair.

Do not make eliminating the package name from `cargo tree` the first objective.
Algebra's [index module](../whippyalgebra/src/index.rs) uses unsigned naturals
for lists, indexing and powers; matrix product uses typenum arithmetic on
dimensions; nalgebra itself depends on typenum. Replacing unit-exponent
arithmetic can remove repeated solver work without removing that dependency.
Replacing list recursion is a later experiment with separate coherence and
zero/nonzero dispatch questions.

## Readability work beyond the backend swap

Replacing core's typenum bridge removes mappings and nested associated-type
projections, but by itself leaves repeated exponent bundles in
[quantity operator generation](../src/api.rs). The next independent change is
delegation to existing unit traits: quantity multiplication requires
`U: UnitMul<V>` and returns `Quantity<MulUnit<U,V>, T, Brand>` for the current
same-storage contract, with `T: Mul<Output=T>`.

That replaces a repeated 36-const-parameter / 12-equality-bound operator body
with its actual unit and value operations. Retain precise per-operation storage
bounds, custom/non-Copy storage and existing brand restrictions. Keep scalar
operations that preserve a unit returning that unit directly. Small primitive
scalar-left macros may remain necessary under Rust's orphan rules.

This cleanup may also benefit the stable backend. The earlier
[macro/trait audit](macro-trait-audit.md) already records a standalone coherence
probe; full production compatibility and performance still need validation.

## Ordered experiments and acceptance checks

| Stage | Deliverable | Evidence needed before expanding scope |
| --- | --- | --- |
| 0 | Isolated scripts/experiments fixture pinned to nightly-2026-09-13 | Existing typenum backend works on that same compiler; record solver/toolchain/server versions |
| 1 | Native N arithmetic and output-inference probes | Add/Sub/Neg, canonical concrete types, unknown associated outputs, existing explicit output parameters, forced overflow checks |
| 2 | Root, boolean and editor feasibility probes | Exact-root bound rejection; equal/unequal uniformity dispatch; inferred velocity hover and constructor completion |
| 3 | Opt-in backend in the current crates | Default stable path and nightly path pass real quantity/algebra tests, conversions, brands, no_std and generic examples |
| 4 | Quantity operators delegate to Unit traits | Same API and rejection behavior; custom storage preserved; source/expansion reduction documented |
| 5 | Minimize compatibility layer | Remove booleans/bridges only where measured useful; list remaining typenum paths explicitly |
| 6 | Controlled measurements, then four-axis composition | Same-compiler backend comparison first; combine representation reduction only after separate attribution |

Keep experimental runners/helpers under `scripts/`. Keep generated fixtures and
artifacts out of production source. The root stable toolchain pin need not change
for the first isolated probes. Record feature gates and any solver flags for
both rustc and rustdoc; an old-solver workaround for typenum cannot silently be
applied to a GCA comparison requiring the new solver.

Use the actual generic centripetal-acceleration example, custom storage tests,
dimension/scale exponent tests, ticks/degrees/local-branded catalog tests,
generic matrices/blocks, auto-reduction, gauges, determinant/powers and Cholesky.
Include negative tests for incompatible dimensions, odd scales, wrong matrix
inner units and brands. Existing out-of-range tests must distinguish the stable
bridge domain from native checked i16 behavior. Compare failure meaning, not
old stable diagnostic snapshots verbatim on a different compiler.

Measure three controls separately: stable compiler with typenum; pinned nightly
with typenum; pinned nightly with GCA. The last pair isolates the backend change.
Use the established tick/degree/nalgebra workload, a solver-heavy consumer and
real generic examples. Measure dependency checks, consumer checks, an incremental
edit, peak memory and direct RA first/repeated hover and completion. Do not count
unknown hovers, empty completions or compiler crashes as fast valid results.

Readability results should record handwritten/generated code, macro expansion
volume, duplicated bounds and representative generic-call syntax. Shorter source
is valuable even when times are flat; timing regressions must remain visible.

## Expectations and scope limits

The strongest initial hypothesis is simpler exponent implementation with fewer
recursive proof obligations. Native constant evaluation still has normalization
and inference costs; no speedup has been measured here. Editor benefit is the
least established part and should be tested early. A useful outcome may retain
typenum for nalgebra and a small compatibility layer while removing it from
most unit arithmetic.

Four-axis reduction and GCA target different work and might combine, but the
earlier roughly 21% result used typenum and a synthetic reduced engine. Removing
typenum recursion can change the marginal benefit of eliminating zero axes;
the percentages cannot be added or assumed to multiply.

Finally, full-i16 *exponent arithmetic* does not imply full-i16 *numeric
conversion*. Existing [scale power tables](../src/scale_conversion.rs) cover
prime powers up to magnitude 100 and pi up to 8, with historical identity
fallback outside those ranges and separate integer overflow behavior. Degrees
and 1/60-second ticks use small exponents and stay within that machinery.
Broader arithmetic should not be advertised as arbitrary-scale conversion
support without a separate correction and tests.

This turn establishes the branch, history, architectural plan and acceptance
criteria. No GCA implementation, tests or benchmarks were run in whippyunits.
