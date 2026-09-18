Macro and trait audit of the current working tree, 2026-09-17.

The strongest trait replacement is angular erasure: hundreds of implementations
can become a few const-generic implementations constrained by a small marker
trait. There are also substantial opportunities that need ordinary functions or
constant tables, rather than traits. Many remaining macros are useful syntax
frontends, or merely generate implementations that are already generic.

This report records the initial audit of the scalar crate, its core and
procedural macro crates, the nalgebra adapter, and its attribute macros. Initial
experiments used isolated copies under `/tmp/whippy-macro-audit`, including the
earlier `vqty` fixes. The subsequent implementation and final measurements are
recorded at the end of this report; earlier sections describe the audit baseline.

**How the system works**

The central runtime object is `Quantity<U, T, Brand>`. It stores a `T`; the unit
and brand are phantom type information. A unit is `Unit<Scale, Dimension>`:

- Dimension contains eight integer exponents: mass, length, time, current,
  temperature, amount, luminosity, and angle.
- Scale contains four exponents representing `2^p2 * 3^p3 * 5^p5 * π^pπ`.
- Brand distinguishes otherwise identical units belonging to different domains.

`qty!(m / s, f64)` parses the unit expression, calculates its exponents during
macro expansion, and produces a concrete quantity type. `quantity!(3.0, m / s)`
constructs a value of that type. `unit!` produces the unit type without storage.
The expansion also contains extra type/const declarations for IDE hover support.

Ordinary Rust operator traits then enforce the relationships. Addition requires
the same unit and brand. Multiplication adds unit exponents; division subtracts
them. Runtime arithmetic acts on the stored values. Explicit rescaling changes
the storage scale while preserving the dimension.

The type-level exponent calculations use `N<const X: i16>` and a bridge to
typenum. This exists because the supported stable compiler cannot express a
generic output type such as `N<{A + B}>`. That restriction was independently
confirmed with the installed rustc 1.98.1. Replacing the mapping macros with
handwritten implementations would leave essentially the same compiler work.

There are four distinct macro jobs here: parsing the unit DSL; calculating
concrete types; generating catalogs and conversion implementations; and working
around derive/editor/generic-bound limitations. These jobs should not all be
treated as one problem.

**What is actually large**

A default-feature expansion of the scalar library contains approximately
131,912 lines and 6.47 MB of expanded Rust, including documentation and retained
macro definitions. These are source-volume measurements, not executable sizes
or a direct measure of compile time.

| Expanded module | Approximate lines | Bytes |
| --- | ---: | ---: |
| `default_declarators` | 86,985 | 3,729,767 |
| `quantity` | 30,487 | 1,357,287 |
| `scale_conversion` | 7,530 | 1,020,889 |
| `api` | 3,375 | 189,535 |

The default literal catalog alone contains 918 symbols times eight macro
definitions: **7,344 literal macros**, plus their reexports. The declarator
module also contains its method traits, aliases, implementations, and four
generator macro definitions.

The important compiler cost is the resulting Rust program: name resolution,
type checking, trait solving, borrow checking, metadata, and code generation.
In the measured baseline, macro expansion itself took about 0.58 seconds,
compared with about 2.88 seconds for type checking and 0.90 seconds for MIR
borrow checking. Replacing macros with equally large generated trait machinery
would not address most of that work.

**Measured experiments**

Measurements use rustc 1.98.1 on x86_64 Linux, default features, the existing dev
profile with incremental compilation disabled, and three forced scalar-library
recompilations per variant. Figures are medians of rustc's `total` timing;
dependency compilation is excluded. They are not cold-workspace build times,
editor latency measurements, or downstream application benchmarks. Variants
were run sequentially, without CPU pinning or a statistical benchmark harness.

| Isolated variant | Median compiler time | Change from baseline |
| --- | ---: | ---: |
| Unchanged snapshot | 9.32 s | — |
| Angular exponent marker trait | 8.60 s | about 8% faster |
| Compact float power tables | 8.43 s | about 10% faster |
| Both changes | 7.66 s | about 18% faster |

Each build was forced by touching the isolated library source. The diagnostic
command was `RUSTC_BOOTSTRAP=1 cargo rustc --offline --lib -- -Ztime-passes`.
`RUSTC_BOOTSTRAP` was used only to expose compiler diagnostics on the installed
toolchain; the prototypes themselves use stable language features.
The phase timings overlap and should not be summed.

The angular prototype passed 81 existing unit/integration tests covering
arithmetic, comparisons, storage conversions and custom storage, and both the
scalar and angular erasure examples. The float-table prototype passed 972
bitwise comparisons in both const and runtime evaluation, including supported
exponents, values outside the supported range, and `i32::MIN`/`MAX`.
The combined prototype also passed those 81 tests and the existing compile-fail
harness's ten cases. Additional isolated compilation probes accepted the ±9 and
zero angular boundaries and rejected ±10, unsupported storage, and branded
erasure. Before promoting these prototypes, retain those probes as permanent
regression tests and measure representative downstream and release builds.

Experiment artifacts are in `/tmp/whippy-macro-audit`: `benchmarks.json` contains
the individual timings, `angle-prototype.patch` contains the angular changes,
`power_parity_probe.rs` checks the floating-point results, and
`boundary-results.json` records the angular boundary checks. The isolated source
trees and build/test logs are alongside them; these temporary files are not
part of the repository.

**Recommended trait simplifications**

| Candidate | Current situation | Simpler design | Assessment |
| --- | --- | --- | --- |
| Angular erasure | 432 conversion bodies | 24 generic bodies + 18 empty exponent markers | Strongest measured trait opportunity |
| Primitive casts | 196 `LossyFrom` impls | Adapter over existing `num_traits::AsPrimitive` | Promising, compatibility review needed |
| Arithmetic scaffolding | 16 storage-generic operators, plus 28 scalar-left impls | Reuse `UnitMul`, `UnitDiv`, `UnitInv` directly | Large readability gain; speed unmeasured |
| Local declarator namespaces | Thousands of repeated nested expansions | Shared aliases and numeric conversion policies | Strong reduction mechanism; prototype needed |
| Default declarator methods | Repeated constructor bodies for storage types | Generic extension-trait implementations | Moderate candidate; check inference and coherence |
| Unit formatting/reflection | Repeated twelve-exponent signatures | Shared associated metadata constants | Primarily maintainability |
| Numeric rescaling dispatch | Concrete const functions and type-name matching | Typed policies plus small concrete const wrappers | Preserve const support and arithmetic semantics |

The angular implementation is in [quantity.rs](../src/quantity.rs:1214) and
[its generator](../proc-macros/src/generate_all_radian_erasures_macro.rs:19).
It enumerates 18 nonzero exponents, seven same-storage scalar conversions,
ten cross-storage scalar conversions, and seven compound conversions:
`18 * (7 + 10 + 7) = 432` bodies. A private marker implemented only for the
supported `_A<N>` values lets the exponent become a const generic in those
24 bodies. The marker excludes zero, preserving separation from dimensionless
conversions and from `core`'s identity `From<T> for T`. The isolated prototype
retains the existing numeric allowlists, default-brand restriction, exponent
range, and conversion arithmetic. It also removes 408 copies of generated
documentation examples.

[LossyFrom](../src/quantity.rs:76) duplicates the primitive `as` conversions
already supplied by the existing num-traits dependency. A blanket adapter over
`AsPrimitive` can replace 196 implementations with one. Sealed primitive markers
can retain the exact type set. Since `LossyFrom` is public, check downstream
custom implementations for overlap before adding a blanket implementation.

Dimensionless erasure is a separate, harder case: it generates 14 same-type
and 182 cross-type `From` implementations. A generic target `impl<T>
From<Quantity<..., T>> for T` violates the orphan rules; a small set of concrete
targets with generic source policies is possible. The numerical policy must be
explicit: current cross-type paths go through `f64`, including unity scale;
same-type paths preserve the raw value at unity scale and otherwise use native
rescaling. Substituting direct casts would change large-integer behavior.
Do not silently combine that semantic change with code cleanup.

Arithmetic already supports generic storage, as documented and emitted in
[api.rs](../src/api.rs:745). It should not acquire a broad new `Storage: Copy +
NumCast` bound: that would regress the existing support for storage types that
implement only selected operations. The existing unit traits allow this shape:

```rust
impl<U, V, T, B> Mul<Quantity<V, T, B>> for Quantity<U, T, B>
where
    U: UnitMul<V>,
    T: Mul<Output = T>,
{
    type Output = Quantity<<U as UnitMul<V>>::Output, T, B>;

    fn mul(self, rhs: Quantity<V, T, B>) -> Self::Output {
        Quantity::new(self.unsafe_value * rhs.unsafe_value)
    }
}
```

A standalone equivalent compiled, including coexistence with scalar-right
multiplication. Addition and negation need even less machinery. Reusing
[UnitMul/UnitDiv/UnitInv](../src/unit.rs:69) centralizes the unit proofs instead
of repeatedly spelling twelve exponents. It introduces associated-type
normalization, so its compile-time benefit remains a hypothesis.

Scalar-left operations still require concrete primitive implementations because
of the orphan rules. Keep a small primitive-list macro. The current
[forwarding generators](../src/api.rs:300) pass large bundles of multiplication,
division, and dimension arguments to
[receivers](../src/arithmetic.rs:201) that now only generate scalar-left
operations. Removing unused forwarding and merging identical signed/unsigned
scaffolding is worthwhile even if compiler timings barely change.

[Local declarators](../proc-macros/src/define_unit_declarators_macro.rs:422)
generate eight base dimensions times 25 prefix choices: 200 method names per
rescaling namespace. One nested `local_unit!` appears per trait signature, and
three more appear per method implementation for each of three storage types:
**2,000 nested `local_unit!` calls per namespace**. Compute each resulting unit
once, reuse aliases, and centralize numeric conversion bodies. The generator
already knows the scale configuration. Avoid repeatedly running the rich
user-facing macro frontend for internally generated types.

[Default declarators](../src/default_declarators.rs:47) already generate traits,
but their pure storage constructors repeat the same body for `f64` and `i32`.
Generic implementations can share those bodies. Preserve or deliberately change
the supported scalar set, unsuffixed-literal inference, and downstream trait
implementation compatibility. Affine and nonstorage units require separate
conversion policies rather than the same identity constructor.

The open [display macro](../src/print/display_traits.rs:3) is invoked once and
produces only two implementations. Ordinary `Display`/`Debug` implementations
using a `UnitMetadata` trait with associated dimension and scale constants would
be clearer. The metadata can also serve `UnitDisplayExt`, `UnitLabel`, and
serialization. This is not an implementation-count hotspot.
[alloc.rs](../src/alloc.rs:9) is already just conditional reexports.

Rescaling can use associated scale constants to shorten its signatures while
retaining concrete per-primitive `const fn` wrappers. A standalone const-function
probe using generic associated metadata compiled. Replacing those wrappers with
ordinary trait-method calls would lose their stable const-call capability.
Keep integer rational arithmetic, float arithmetic, π approximation, and the
integer overflow heuristic distinct. Also replace type-spelling dispatch in
[value!](../proc-macros/src/value_macro.rs:101): matching strings such as `f32`
cannot reliably determine the meaning of aliases or generic types.

**Other compile-time opportunities**

[Power generation](../proc-macros/src/pow_lookup_macro.rs:143) emits a complete
multiplication chain separately for each exponent. The six ±100 tables contain
59,400 multiplication operators; floating π adds another 56. The measured
prototype replaces only the four floating tables with sequentially constructed
constant arrays, removing 29,756 repeated expressions while preserving rounding
order, reciprocals, and out-of-range identity behavior. Rational tables were
left alone: some supported powers exceed `i128`, so eagerly evaluating every
entry would change when overflow occurs. This is a function/data simplification,
not a trait abstraction.

The [literal generator](../proc-macros/src/utils/literal_macros.rs:129) emits
eight exported macros per symbol regardless of which suffixes are used.
Make this catalog optional or selectable, share it where namespace semantics
allow, or evolve the suffix attribute to dispatch only the suffixes present in
its input. Traits cannot parse `5.0m`. Removing the catalog has API implications;
the source-volume count alone does not establish a percentage speedup.

[utils/culit.rs](../proc-macros/src/utils/culit.rs:5) also constructs two complete
legacy macro vectors, discards them, and invokes the newer shared generator.
Delete that unused generation pass. It affects `define_literals!`; the default
catalog does not take this particular path.

Hover support in [unit_macro.rs](../proc-macros/src/unit_macro.rs:115) wraps types
in `<Helper<{ documentation declarations; 0 }, ActualType> as
GetSecondGeneric>::Type`. Each identifier can add an anonymous const and alias.
Reduce this machinery in generated internal aliases first. Removing it from
user-written unit expressions would sacrifice the per-token IDE information.

The recent `vqty` support parses its grammar in both the declarative macro and
the `expand_units` frontend. A shared non-proc-macro expansion module or
algebra-owned attribute frontend would reduce duplication. Traits cannot change
the fact that derives run before a field's type macro expands. Named type aliases
also let user code avoid many uses of the expansion attribute.

For [uniform matrix constructors](../whippyalgebra/src/nalgebra/macros.rs:143),
a normal constructor accepting `[[Quantity<U, T, B>; C]; R]` can infer shape,
enforce rectangular rows, and use one generic body. The macro can remain a thin
DSL wrapper. Mixed matrices require different types per entry, so homogeneous
arrays cannot replace their checks. Tuple-based trait recursion may merely move
the complexity into trait solving.

The block constructor itself is already a thin `hcat`/`vcat` wrapper.
`unblock_matrix!` creates bindings, which traits cannot do. The large
[generic algebra attributes](../whippyalgebra-macros/src/lib.rs:511) encode real
allocator, partition, and shape obligations. Operation traits with associated
outputs and methods can hide some of those details. Empty marker traits carrying
arbitrary `where` clauses do not automatically make all obligations available to
callers. Treat this as an API design project, with benchmarks, rather than an
obvious speed optimization.

Keep eager evaluation in `unit!`/`qty!`: emitting a single concrete unit type can
be cheaper than emitting a nested tree of associated-type multiplications and
divisions. Keep the numeral bridge initially: 401 forward and 801 reverse
mapping implementations support generic arithmetic. Per-pair arithmetic tables
would be much larger. The eight-dimension and named-dimension generators are
small enough to rank below the catalogs and conversions.

**A separate correctness issue found during the sweep**

[dimension_exponents.rs](../whippyunits-core/src/dimension_exponents.rs:474)
hardcodes `Add` predicates inside a generator also used for `Sub`. Against the
current core library, a probe requiring `Length - Length` to have exponent two
compiled; the expected exponent-zero result failed to compile. `Length - Time`
likewise has the product's exponents in this API. The main `UnitDiv` path uses a
separate correct implementation, so this does not mean ordinary quantity
division is broken. Fix this independently and give the helper direct operation
tests before restructuring its generator.

**Suggested sequence**

First, fix the confirmed dimension-helper subtraction bug separately. Then
promote the angular marker and float-table prototypes with boundary tests and
representative downstream checks. Simplify arithmetic using existing unit
traits and introduce shared metadata for readability. Next, measure catalog
selection and local-declarator deduplication, which affect very different user
workloads. Leave a broad change to the numeral representation or nalgebra
allocator API until those smaller improvements are measured.

Also test development builds with incremental compilation enabled:
[Cargo.toml](../Cargo.toml:156) explicitly disables it. That is an independent
edit/build iteration opportunity, not a claim about macro-expansion speed.

Supporting language references: the
[Rust coherence rules](https://doc.rust-lang.org/reference/items/implementations.html#trait-implementation-coherence)
explain the concrete-target and overlap constraints; the
[const generic restrictions](https://doc.rust-lang.org/reference/items/generics.html#const-generics)
explain the numeral bridge; and
[Cargo's timing documentation](https://doc.rust-lang.org/cargo/reference/timings.html)
distinguishes crate-build timelines from detailed compiler phases.

**Follow-up: a dynamics-only unit catalog**

A further isolated experiment filtered the default declarator and literal
generators to retain units whose current, temperature, amount, and luminous
intensity exponents are all zero. This keeps mass, length, time, angle, and
their derived quantities, including force, energy, power, pressure, and density.
It retains 15 of the catalog's 28 named dimensions. Electric charge needs no
separate base-axis decision: its dimension includes current times time.

The experiment leaves the eight-axis unit representation, the full core lookup
metadata, all conversion generators, and `Brand` unchanged. It is a generation
filter in a temporary source copy, not a completed public feature-flag API.

| Variant | Median compiler time, three runs | Literal macros |
| --- | ---: | ---: |
| Fresh unchanged baseline | 9.408 s | 7,344 |
| Dynamics catalog | 7.944 s | 4,104 |

That is about a 15.6% reduction in the scalar library's compiler time under the
same diagnostic/dev-build method used above. Expanded source declined from
131,912 lines to 96,366 lines. Neither the earlier angular nor power-table
optimization was included; these percentages must not be added together.
The centripetal-acceleration example compiled and ran successfully. A separate
smoke example checks selected dynamics literals, constructor methods, and
force/energy arithmetic. Raw timings and the temporary source tree are in
`/tmp/whippy-macro-audit/catalog-benchmarks.json` and
`/tmp/whippy-macro-audit/dynamics-catalog-prototype`.

This benefit applies when compiling the library. Once Cargo reuses an unchanged
dependency, ordinary application edits do not pay that complete library build
again. Smaller metadata and fewer declarations may also help downstream
checking and editor analysis, but neither was timed here. Merely narrowing
`use` imports does not stop the library from generating its public catalog.

A practical feature design would keep the existing full catalog as the default
for compatibility and allow additive families such as dynamics, electrical,
thermal, chemical, and photometric units. Literal suffix catalogs could be
independently optional. Gate generation itself, not just reexports, and ensure
the proc-macro crate receives the selected catalog configuration. The parser's
nonstorage-unit dispatch currently refers to generated constructor traits, so
all affected generation and dispatch paths need a consistent policy.

Keep the eight-dimensional type representation initially. Catalog filtering
does not remove the four irrelevant zero-axis arithmetic obligations from a
dynamics multiplication. Removing those coordinates might reduce trait work,
but would affect public types, macro output, formatting, serialization, and
algebra. That is a separate unmeasured redesign. Cargo's additive feature
unification also means a dependency enabling another catalog family enables it
for the shared library build; a feature cannot guarantee exclusive access to a
smaller set.

`Brand` has a different cost profile. `Quantity<U, T, Brand = ()>` stores the
brand only in `PhantomData`, and generic implementations do not enumerate a
catalog of possible brands. Brand-specific declarator catalogs are generated
only when explicitly requested. Default-brand users already avoid that extra
generation. Carrying the generic argument and derived bounds has some compiler
cost, but there is no measurement showing that removing it is worthwhile.
Retain `Brand = ()`; unnecessary `Clone`/`PartialEq` bounds on phantom parameters
could be cleaned up separately without a feature-dependent public type shape.

At the time of the initial audit, public features were `std`, `alloc`, and
`serde`; disabling defaults did not trim catalogs. The implementation below
adds catalog selection.


**Implementation and final measurements**

Implemented on `perf/selectable-unit-catalogs` in a separate worktree, preserving
all original-checkout edits. The branch first carries the earlier `vqty` fix,
then records the following changes separately:

1. Selectable `units-dynamics`, `units-electrical`, `units-thermal`,
   `units-chemical`, and `units-photometric` catalogs, with independent `literals`.
   Existing defaults keep `all-units` and `literals`. `whippyalgebra` forwards
   these features. The complete unit-expression DSL remains usable without
   catalogs, including local/branded namespaces and inferred numeric conversion
   behavior for nonstorage and affine units. Hover aliases use concrete types
   when their named catalog aliases are absent. `culit` is now a development
   dependency because library code does not require it.
2. Compact floating-point power tables, preserving constant evaluation,
   multiplication order, reciprocal behavior, and fallback values.
3. Angular erasure through 24 generic bodies and 18 private exponent markers,
   preserving numeric type support and conversion boundaries.
4. Removal of discarded literal token generation in `define_literals!`.
5. The dimension-exponent subtraction fix, with direct typed-output tests.

The eight-dimensional representation and `Brand = ()` remain unchanged. The
larger, unmeasured arithmetic and public conversion-trait redesigns were not
included. An inherited example typo (`J` instead of `s` in the third row of
`block_roundtrip`) was corrected in this worktree to match its aliases and data.

A fresh benchmark of the completed implementation held `std` and `serde` enabled
in every variant, so removing serialization is not part of these improvements:

| Configuration | Median compiler time | Reduction from baseline |
| --- | ---: | ---: |
| Pre-optimization snapshot | 9.268 s | — |
| Optimized, full default catalog | 8.049 s | 13.2% |
| Optimized, dynamics with literals | 6.567 s | 29.1% |
| Optimized, dynamics without literals | 5.905 s | 36.3% |

These remain three-run, sequential development-library measurements excluding
dependency compilation, not release, cold-workspace, editor-latency, or
application benchmarks. Detailed timings and the command are retained in
[compile-time-results.json](compile-time-results.json). Earlier prototype
percentages must not be added to these combined measurements.

Validation passed: 313 tests across the scalar/core/procedural-macro/algebra
library and integration suites, with 36 additional UI compilation cases checked
by their harnesses; documentation builds for those crates; and the dynamics-only
`block_roundtrip` example. Catalog API tests passed all nine configurations in
[the new CI matrix](../.github/workflows/catalog-features.yml), including empty
catalogs, literals alone, each family, and full catalogs. Negative UI cases
verify that excluded aliases and literal macros are absent. Permanent angular
boundary tests and bitwise const/runtime power-table parity tests cover the
optimized implementations.

For the requested dynamics use case:

```toml
whippyunits = { version = "0.3", default-features = false, features = ["std", "units-dynamics"] }
```

Add `literals` for suffix syntax and `serde` for serialization. If depending on
`whippyalgebra`, disable its defaults too and select `std`, `nalgebra`, and
`units-dynamics`; otherwise its default full catalog is unified back in.
