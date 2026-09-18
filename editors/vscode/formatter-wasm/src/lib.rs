//! WebAssembly bindings around the lsp-proxy formatting code.
//!
//! The VS Code extension in `editors/vscode` loads this module so it rewrites
//! types with exactly the same logic as the `lsp-proxy` binary and the
//! `whippyunits-pretty` CLI. Everything here is text in, text out; the
//! extension does the surgery on VS Code's hover/inlay-hint objects itself.

use wasm_bindgen::prelude::*;
use whippyunits::print::prettyprint::pretty_print_quantity_type;
use whippyunits_core::UnitExpr;
use whippyunits_lsp_proxy::{
    hover_processor::HoverProcessor,
    inlay_hint_processor::InlayHintProcessor,
    quantity_detection,
    unit_formatter::{DisplayConfig, UnitFormatter},
};

/// A configured formatter. Create a new one whenever the display settings change.
#[wasm_bindgen]
pub struct Formatter {
    hover: HoverProcessor,
    inlay: InlayHintProcessor,
    text: UnitFormatter,
    text_config: DisplayConfig,
    verbose: bool,
}

#[wasm_bindgen]
impl Formatter {
    #[wasm_bindgen(constructor)]
    pub fn new(verbose: bool, include_raw: bool) -> Formatter {
        let hover_config = DisplayConfig {
            verbose,
            unicode: true,
            include_raw,
        };
        // Inlay hints are always compact, matching `LspProxy::with_config`
        let inlay_config = DisplayConfig {
            verbose: false,
            unicode: true,
            include_raw: false,
        };
        // Diagnostics never get a "Raw:" section appended
        let text_config = DisplayConfig {
            verbose,
            unicode: true,
            include_raw: false,
        };
        Formatter {
            hover: HoverProcessor::new(hover_config),
            inlay: InlayHintProcessor::with_config(inlay_config),
            text: UnitFormatter::new(),
            text_config,
            verbose,
        }
    }

    /// Rewrite the markdown of one hover entry.
    pub fn format_hover(&self, markdown: &str) -> String {
        self.hover.improve_hover_text(markdown)
    }

    /// Rewrite whippyunits types inside plain text, e.g. a diagnostic message.
    pub fn format_text(&self, text: &str) -> String {
        self.text.format_types(text, &self.text_config)
    }

    /// Pretty-print inlay-hint type text (the label with any leading `: ` removed):
    /// every `Quantity<Unit<…>>` and bare `Unit<Scale…>` is rewritten in place.
    pub fn format_inlay_type(&self, type_text: &str) -> String {
        self.inlay.format_inlay_type(type_text)
    }

    /// The `: qty!(...)` annotation for a pretty-printed inlay label such as
    /// `Quantity<mm, i32>`, used as the hint's double-click text edit.
    pub fn unit_macro_annotation(&self, pretty_type: &str) -> String {
        self.inlay.unit_macro_annotation(pretty_type)
    }

    /// The label the formatter prints for a unit expression as written in a
    /// `qty!`/`vqty!` invocation (`m / s` → `(m·s⁻¹)`, `kg * m / s ^ 2` → `N`),
    /// parsed with the same parser the macros use. `undefined` if it doesn't parse
    /// or doesn't resolve.
    ///
    /// rust-analyzer's hover on a struct field shows the *declared* type, in which
    /// negative exponents appear only as `{const}`; the source line still has the
    /// unit expression, so the extension can recover the label from it.
    pub fn unit_label(&self, unit_expr: &str) -> Option<String> {
        let unit: UnitExpr = syn::parse_str(unit_expr).ok()?;
        let result = unit.evaluate();
        // Render through the same path as an inline `Quantity<LABEL, f64>` so the
        // label matches what the formatter prints, then strip the wrapper.
        let pretty = pretty_print_quantity_type(
            result.dimension_exponents,
            result.scale_exponents,
            "f64",
            self.verbose,
            false,
            None,
        );
        let label = pretty.strip_prefix("Quantity<")?.strip_suffix(", f64>")?;
        (label != "?").then(|| label.to_string())
    }
}

/// Cheap check for whippyunits types, run before doing any real work on a text.
#[wasm_bindgen]
pub fn contains_quantity(text: &str) -> bool {
    quantity_detection::contains_quantity_types_fast(text)
}
