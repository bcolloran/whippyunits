Rust-analyzer follow-up, updated 2026-09-18.

The latest [typenum and four-axis experiments](typenum-dynamics-experiments.md)
use a larger nalgebra consumer with degrees and 1/60-second ticks. Explicit
arithmetic tables substantially regress editor requests; slimming typenum saves
less than 1% of server memory. A real four-axis prototype improves heavier
compiler checks, but its three editor samples do not establish an application
hover/completion speedup. All twelve methods in the production
`units-dynamics-conversions` catalog passed the completion probe.

The earlier combined experiments compare `6196218` with `4494b79`, selecting
`std,units-dynamics` before and `std,units-si-dynamics` after, with defaults off.
Three interleaved fresh-server runs per snapshot measured indexing at **28.908 s
→ 3.180 s**, and first inferred velocity hover at **158.373 ms → 29.233 ms**.
All ten coherent constructor completions were verified in every final run;
all ten were missing in the baseline. Final first `meters()` completion cost
573 ms, with repeated requests at a median 12.9 ms. Missing baseline completions
are excluded, so no completion speedup ratio is claimed.

These are direct LSP measurements with warm Cargo artifacts, cache priming on,
and no competing compilation. They exclude VS Code, the formatter extension,
and the larger algebra workspace. See [the full experiment log](performance-experiments.md)
and [raw samples](performance-results/06-combined-rust-analyzer.json). The
checked-in probe is `scripts/measure_rust_analyzer.py`; it requires successful
unit-method text edits and correct inferred length/time hover types.

The earlier catalog-only experiment below predates compact rational tables,
the optional exponent bridge, coherent catalogs, and pre-generated constructors.

Reducing the generated catalogs improved indexing in a direct language-server
experiment. It did not materially improve a simple scalar hover once the server
was ready. Compiler benchmarks alone would not have established that distinction.

| Measurement | Full catalog and literals | Dynamics, no literals |
| --- | ---: | ---: |
| Fresh server to completed indexing, median | 47.2 s | 26.9 s |
| First quantity hover after indexing, median | 555 ms | 555 ms |
| Subsequent quantity hovers, median | 0.33 ms | 0.30 ms |

This is about a 43% indexing-time reduction in this particular experiment.
It is not a prediction of the same percentage for the complete workspace or
every application. Both configurations use the already optimized library.

The probe launched the installed VS Code server, `rust-analyzer
0.3.3049-standalone`, directly over LSP. Two independent Cargo consumer projects
used the same small source file with unit macros, constructors, and dynamics
arithmetic. Each configuration was run twice with a fresh server process and
warm Cargo/filesystem caches. Cache priming and proc macros were enabled;
check-on-save was disabled and Cargo all-targets was false. The full consumer
enabled `std`, `all-units`, and `literals`; the reduced consumer enabled `std`
and `units-dynamics`. The server's quiescent notification after indexing defined
the ready time. The quantity hover resolved the expected length/time type in
both cases. Neither the VS Code interface nor the formatter extension was part
of this measurement.

The completion probe returned native floating-point methods but did not return
the expected unit-constructor methods in either configuration. Those timings
therefore do not establish a unit-completion improvement and are excluded from
the table. The subsequent probe found parsing failures in nested catalog macro
expansions containing negative exponents. Pre-generated textual constructors
restore the tested methods. See [the expansion diagnostics](performance-results/04-completion-diagnosis.json).

Raw startup and hover samples are in
[rust-analyzer-results.json](rust-analyzer-results.json). The local probe,
source projects, and progress logs are under `/tmp/whippyunits-ra-perf`.

**A workspace issue found and corrected**

The repository's `lsp-proxy` and formatter-WASM packages previously depended on
whippyunits with its default features. Consequently, workspace Cargo metadata
still enabled `all-units`, `literals`, and `serde` even when requesting
`--no-default-features --features std,units-dynamics`. The flags could reduce an
isolated `cargo rustc -p whippyunits` build while the editor's workspace graph
still included the complete catalog.

Both tooling dependencies now specify `default-features = false` and
`features = ["alloc"]`. Their pretty-printing functions require allocation and
the complete core metadata, but do not require generated constructors, suffix
macros, or quantity serialization. Their own LSP serialization dependencies
remain unchanged. This does not remove electrical or thermal unit names from
formatted output.

The earlier workspace metadata with the following feature selection resolved
only `alloc`, `std`, and `units-si-dynamics` on whippyunits, with neither
`all-units` nor `wide-exponents` accidentally enabled:

```text
whippyunits/std,whippyunits/units-si-dynamics,whippyalgebra/std,whippyalgebra/nalgebra,whippyalgebra/units-si-dynamics
```

The proxy's 40 library tests passed without default features, and the formatter
passed `cargo check` for `wasm32-unknown-unknown`. The active editor settings were
not changed.

For an editor session focused on dynamics including degrees and ticks in this
worktree, the corresponding rust-analyzer settings are:

```json
{
  "rust-analyzer.cargo.noDefaultFeatures": true,
  "rust-analyzer.cargo.features": [
    "whippyunits/std",
    "whippyunits/units-dynamics-conversions",
    "whippyalgebra/std",
    "whippyalgebra/nalgebra",
    "whippyalgebra/units-dynamics-conversions"
  ]
}
```

Add the relevant catalog features when editing tests/examples that use those
catalogs. Add `whippyunits/wide-exponents` if intermediate physical or scale
exponents exceed 32 in magnitude. For development of the complete library, the full configuration is
still useful; a separate consumer project gives a cleaner model of a downstream
dynamics application's reduced feature graph. Merely opening a member manifest
does not guarantee isolation from its Cargo workspace. The optimization branch
is in `/tmp/whippyunits-macro-compile-time`; opening the original checkout does
not automatically use it.

The [rust-analyzer configuration reference](https://rust-analyzer.github.io/book/configuration.html)
documents the feature settings, cache priming, and separate Cargo target
directory option. Limiting background Cargo checks and changing analysis
features are different controls. In particular, `check.workspace = false` does
not itself remove crates from the analysis graph, and turning off cache priming
can defer work to later requests rather than eliminate it.

**The remaining hover/completion investigation**

The four-axis prototype now has isolated compiler evidence; production editor
benefits and removing `Brand` remain unestablished. Profiling expensive editor
requests should distinguish analysis work from presentation work. Rust-analyzer provides LSP tracing and
`RA_PROFILE` instrumentation in its
[profiling documentation](https://rust-analyzer.github.io/book/contributing/#profiling).

The current extension hooks hover, inlay hints, and diagnostics; it does not
directly rewrite completion responses. A hover first awaits rust-analyzer and
then invokes the WASM formatter synchronously. Matrix diagram generation has
no size cap and formats every cell; diagnostic and inlay batches also format
items synchronously. These are concrete potential costs, not measured causes
of the reported latency.

The existing `whippyunits.enabled` setting applies live and can isolate that
presentation stage while preserving rust-analyzer's warmed caches. If the old
executable LSP proxy is configured instead, its serial response-forwarding path
is another possible source of queueing. No active proxy configuration was found
in the inspected workspace/user settings, so its involvement is not assumed.
