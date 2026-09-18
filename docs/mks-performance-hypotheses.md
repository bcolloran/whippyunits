Follow-up audit for coherent MKS dynamics, 2026-09-17. Source baseline:
`perf/selectable-unit-catalogs` at `3daafb6`.

The intended subset is kilograms, meters, seconds, radians, and quantities
derived from them, without custom brands or alternative storage scales. The
strongest next opportunities are a much smaller constructor catalog, compact
rational power generation, fewer unnecessary type obligations in algebra, and
a smaller exponent bridge. Removing one phantom brand parameter is a lower
priority than these changes.

This report combines a source audit with isolated compilation experiments.
Production Rust sources were not changed. Raw samples and measurement scope
are in [mks-performance-results.json](mks-performance-results.json). Counts of
generated declarations and trait obligations are not predicted timing ratios.

| Candidate | Evidence | Assessment |
| --- | --- | --- |
| Coherent MKS+radian constructors; optional prefixes and other units | Dynamics still generates 691 constructor entries and 1,423 bodies; the proposed coherent catalog has 10 entries and 20 bodies | Strong next catalog experiment |
| Compact rational power generation | 29,700 repeated multiplications remain; prototype cut the measured rebuild/check chain about 20% | Strong measured candidate; preserve overflow and runtime behavior before adopting |
| Narrow exponent bridge to +/-32 | 1,202 bridge impls become 194; measured rebuild/check chain improved about 8% | Worth an optional profile; audit intermediate exponents |
| Remove phantom bounds and redundant raw-construction proofs | Concrete unnecessary obligations in scalar traits and matrix accessors | Small compatible changes; measure editor requests |
| Separate runtime type arithmetic from parser/catalog | Runtime core currently depends on syn and proc-macro2 | Moderate architectural change; target and host costs differ |
| Four-axis, coherent-scale unit representation | Twelve exponent obligations per multiplication become four | Promising structural experiment, substantial API work |
| Separate branded quantity struct | Removes one generic argument but may duplicate operations and conversions | Low priority without evidence |

The existing dynamics catalog is still broad. The
[default generator](../proc-macros/src/generate_default_declarators_macro.rs:349)
prefixes 26 dynamics storage spellings in 25 ways: unprefixed plus 24 SI
prefixes. That produces 650 aliases and two constructor bodies per alias. An
additional 41 nonstorage constructor entries each get three bodies. Hence
`650 * 2 + 41 * 3 = 1,423` constructor bodies, before optional literals.
These are static counts obtained by mirroring generation rules over the
canonical metadata, rather than measurements of compiled machine code. The 691
entries also have distinct method names. The inventory script and output are
`/tmp/whippyunits-mks-catalog-counts.py` and
`/tmp/whippyunits-mks-catalog-counts.json`.

A coherent catalog could retain `kg`, `m`, `s`, `rad`, `Hz`, `N`, `J`, `Nm`,
`W`, and `Pa`. `Nm` is an optional redundant spelling in this representation.
Named derived units such as newtons are useful: they have exactly the same
type as their base-unit expression and need no scale conversion. Compound
velocities, accelerations, inertias, and densities remain expressible through
the DSL without separate named constructors.

Filtering `System::Metric` is insufficient: the catalog calls minutes, degrees,
liters, hectares, bars, and other noncoherent units metric. A conversion factor
of one is also insufficient: some nonidentity scales are represented exactly
through prime-factor exponents. Selection must require identity resolved scale,
identity conversion factor, and zero affine offset. Kilogram needs explicit handling because the metadata
starts from gram and the current prefix loop creates kilogram; suppressing all
prefixes blindly would remove the MKS mass unit.

Dimension selection and catalog extensions should be independent, additive
choices. Proposed extensions include prefixes, extra metric units, imperial
units, and astronomical units, with current full defaults preserved. A new
lean entry point can avoid changing what the existing `units-dynamics` flag
promises. A negative `mks-only` flag would conflict with Cargo's
[additive feature model](https://doc.rust-lang.org/cargo/reference/features.html#feature-unification).
Other dependencies can still enable broader catalogs through feature
unification. Default, local, branded, literal, and hover generators should all
use the same resolved selection. In particular,
[hover alias fallback](../proc-macros/src/utils/shared_utils.rs:113) currently
tests whether a dimension is enabled; it must also handle omitted units within
an enabled dimension.

An alternative already available on this branch is to disable all convenience
catalogs and use `qty!`, `quantity!`, and explicit type aliases. That retains
unit checking and the complete DSL. It provides a useful lower-bound experiment
for the cost of catalog generation, but sacrifices constructor-method discovery.
It does not remove the parser, scale conversions, or exponent representation.

The [exponent bridge](../whippyunits-core/src/num.rs:57) maps 401 input values
(-200 through 200) into typenum and 801 possible output values (-400 through
400) back into `N<const X>`. There is no 401-by-401 addition table. Addition
maps two inputs to typenum, performs type-level arithmetic, then maps the
result back. Typenum uses recursive binary digits: 200 has eight binary
digits, rather than 200 successor nodes. See its
[UInt representation](https://docs.rs/typenum/1.20.1/typenum/uint/struct.UInt.html).
Reducing the supported domain removes declarations; the representation and
trait arithmetic for an existing small calculation such as `1 + 2` remain the
same.

| Input range | Output range | Bridge impls | Core check median | Dynamics check with core and macro rebuilds |
| --- | --- | ---: | ---: | ---: |
| +/-200 | +/-400 | 1,202 | 0.554 s | 4.296 s |
| +/-32 | +/-64 | 194 | 0.342 s | 3.966 s |
| +/-16 | +/-32 | 98 | 0.333 s | 3.942 s |

Each entry is the median of three interleaved wall-clock `cargo check` runs,
with warm third-party dependencies and incremental compilation disabled. Core
checks force a core rebuild; dynamics checks force core, proc-macro, and root
rebuilds with defaults off and `std,units-dynamics` enabled. These are not cold
dependency builds, representative application edits, or rust-analyzer timings.
The small additional difference between +/-32 and +/-16 does not justify
choosing the tighter bound without application evidence.

Both dimension and scale exponents use this same bridge. Removing prefixes
makes a small scale domain easier to justify, but matrix intermediates still
matter. Determinants, squared quantities, and whole-matrix products can reach
higher exponents than the final physical output. The current domain is not
closed: `200 + 200` produces 400, but 400 cannot be an input to another
bridge-backed operation or equality check. A +/-32 variant similarly allows
32+32 to produce 64 without allowing that 64 to be reused. A production profile
needs explicit boundary tests and diagnostics, including intermediate results.

Direct arithmetic tables are a separate experiment. With inputs +/-8 there
are 289 pairs per binary operation, or 578 Add/Sub impls. With +/-16 there are
2,178 Add/Sub impls, before equality, negation, and halving. Such tables trade
recursive proof work for more declarations; they are not automatically faster
than the 98-impl narrow bridge. Removing this bridge also would not remove all
typenum usage, because the algebra adapter uses it for matrix shapes and lists.

The three remaining
[rational power generators](../src/scale_conversion.rs:1) each unroll positive
and negative powers through 100. They still produce 29,700 multiplication
expressions in total, despite the earlier compact floating tables. An isolated
prototype replaced these three generators with a shared, left-to-right
`const fn` loop, preserving the existing out-of-range identity result.

| Rational-power experiment | Original median | Loop prototype median |
| --- | ---: | ---: |
| Force only root library check | 2.371 s | 1.416 s |
| Force core, proc-macro, and root check | 4.175 s | 3.330 s |

These were separate three-sample interleaved experiments using the full +/-200
bridge and the same dynamics features. The roughly 40% root-only and 20%
rebuild-chain changes describe different scopes. They must not be added to
the exponent-domain improvement or earlier build benchmarks.

Both implementations passed 483 representable-power and out-of-range cases,
plus two constant evaluations. This prototype is not yet a production change:
overflow cases, release behavior, and conversion runtime speed still need
validation. A runtime loop may be slower than the existing constant lookup
when its exponent is not folded at compile time. Checked constant tables with
a carefully preserved overflow path are another implementation option. Merely
eagerly evaluating all powers into an i128 array is invalid because some
supported powers exceed i128. A coherent-only backend could instead remove
scale-conversion APIs and their generation altogether; removing constructor
names alone does not do that.

For branding, [Quantity](../src/quantity.rs:384) stores `Brand` only through
`PhantomData`, and no implementations are enumerated over possible brands.
Generic code can cost compilation time through checking and concrete code
generation, but Rust does not instantiate every possible brand. The compiler
collects concrete instantiations that are actually needed; see the
[monomorphization guide](https://rustc-dev-guide.rust-lang.org/backend/monomorph.html).
An alias fixing the brand to `()` only changes spelling. Two actual quantity
structs might simplify unbranded signatures, but supporting both can add
forwarding implementations, conversions, and interoperability work.

There is a smaller improvement to make first: the derived `Clone` and
`PartialEq` implementations impose phantom `U`/`Brand` bounds, which flow into
`Copy`, `Eq`, `Pod`, and ordering. Manual implementations based only on the
stored value could remove those obligations without splitting the public
type. The matrix wrappers already use that approach. Necessary lifetime
requirements for `Pod` must remain.

Several algebra-specific hypotheses are more directly related to complicated
hover and completion requests:

- [FromRaw](../whippyalgebra/src/entry.rs:100) destructures all twelve unit
  exponents just to construct or borrow a quantity. The adapter has 31 explicit
  bound lines and 35 qualified calls to it. Existing inherent raw/ref/mut
  constructors accept opaque `U`; using them can avoid proving the unit shape
  again after already computing its type.
- [UnitPow](../whippyalgebra/src/index.rs:233) builds an N-element repeated
  list and folds multiplication over it. Direct exponent multiplication or a
  binary-power algorithm could avoid this linear intermediate structure for
  determinants and products, subject to the bounded-intermediate semantics.
- Mixed-unit indexing traverses dimension lists and divides row/column units.
  Explicit uniform blocks avoid those particular operations. Automatic
  uniformity detection compares twelve exponents per adjacent pair and folds
  the Boolean results. This does not imply that every mixed-matrix operation
  is slower: ordinary mixed multiplication can simply match its inner lists.
- Optional decomposition APIs could reduce adapter declarations for consumers
  that only need basic vectors and matrices. The linalg module currently
  includes 21 Rust files, about 2,227 nonblank noncomment lines, whenever the
  nalgebra adapter is enabled. It would not remove nalgebra's own definitions.

If only floating-point storage is needed, concrete conversion generation is
another possible cut: 196 `LossyFrom` implementations, 182 cross-storage
dimensionless conversions, 14 same-storage conversions, and 28 scalar-left
operators remain. Most quantity arithmetic is already generic over storage,
so removing integer support would not eliminate separate integer copies of
every arithmetic implementation. Constructor inference, literal defaults, and
integer rescaling would need consistent optional-feature behavior.

The largest representation experiment is a distinct coherent-dynamics unit
with mass, length, time, and angle only. Current
[UnitMul](../src/unit.rs:180) always has twelve exponent obligations: eight
physical dimensions and four scale exponents. A four-axis unit at a fixed
coherent scale needs four. A type alias fixing the other eight to zero does
not change the existing implementation. A distinct representation or backend
can remove the obligations, but repeated zero proofs may already be cached,
so a threefold obligation-count reduction is not a threefold speed prediction.
It also needs explicit integration wherever APIs destructure the existing
`Unit<Scale, Dimension>` shape. Catalog features should not silently change
that public shape through Cargo feature unification.

Finally, [whippyunits-core](../whippyunits-core/Cargo.toml:12) currently includes
the parser, all catalog metadata, syn, and proc-macro2 in its normal dependencies.
A small arithmetic/type kernel, with parsing and the rich catalog above it,
would reduce the target-side dependency graph. The proc-macro host still needs
its parser, and optional display/serialization may need the catalog. Separating
these costs is more useful than assuming they vanish when constructor flags
are disabled. Rich per-token hover aliases and the `Helper` projection are
also worth an isolated editor A/B experiment, with any loss of token-level
documentation reported explicitly. Internal generators should emit already
resolved type tokens rather than repeatedly invoking documented DSL macros.

The next measurements should use an actual dynamics consumer with scalar,
vector, and mixed-matrix expressions. Measure dependency rebuilds, warm
consumer checks after an edit, fresh rust-analyzer indexing, and uncached hover
and completion separately. Enable ordinary incremental development builds as
a separate experiment: the workspace currently disables incremental compilation.
Prioritize coherent catalog generation and compact rational powers, then the
moderate exponent bound and algebra obligation cleanup. Prototype a smaller
unit representation only after those measurements establish what remains slow.
