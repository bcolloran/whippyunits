//! WebAssembly bindings around the lsp-proxy formatting code.
//!
//! The VS Code extension in `editors/vscode` loads this module so it rewrites
//! types with exactly the same logic as the `lsp-proxy` binary and the
//! `whippyunits-pretty` CLI. Everything here is text in, text out; the
//! extension does the surgery on VS Code's hover/inlay-hint objects itself.

use wasm_bindgen::prelude::*;
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
}

/// Cheap check for whippyunits types, run before doing any real work on a text.
#[wasm_bindgen]
pub fn contains_quantity(text: &str) -> bool {
    quantity_detection::contains_quantity_types_fast(text)
}
