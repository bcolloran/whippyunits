//! `#[expand_units]`: rewrites `unit!`/`qty!` calls in the type positions of an
//! item into the types they expand to, so that `#[derive]`s written below the
//! attribute see ordinary types rather than macro calls.
//!
//! The built-in derives walk the field types of a *generic* item looking for
//! projections (`T::Assoc`) that need extra `where` bounds, and refuse outright
//! when that walk meets an unexpanded macro ("`derive` cannot be used on items
//! with type macros"). Attributes expand top to bottom, so an attribute placed
//! above the derives gets to rewrite the item before they run.

use std::collections::HashSet;

use proc_macro2::TokenStream;
use quote::quote;
use syn::visit_mut::{self, VisitMut};
use syn::{Item, Path, Type, TypeMacro};

use crate::unit_macro::UnitMacroInput;

pub fn expand(attr: TokenStream, item: TokenStream) -> TokenStream {
    if !attr.is_empty() {
        return syn::Error::new_spanned(attr, "`#[expand_units]` takes no arguments")
            .to_compile_error();
    }
    let mut item: Item = match syn::parse2(item) {
        Ok(item) => item,
        Err(err) => return err.to_compile_error(),
    };
    let mut expander = Expander::default();
    expander.visit_item_mut(&mut item);

    // Name resolution never sees the calls that were rewritten, so an import
    // they went through (`use whippyunits::qty;`) would otherwise be reported
    // as unused. An anonymous re-import of each rewritten path keeps it marked
    // as used without bringing anything new into scope.
    let used = expander.expanded_paths;
    let keep_imports_used = if used.is_empty() {
        quote!()
    } else {
        quote! {
            const _: () = {
                #(#[allow(unused_imports)] use #used as _;)*
            };
        }
    };

    quote! {
        #item
        #keep_imports_used
    }
}

/// Replaces each `unit!`/`qty!` type macro with its expansion. Every other type
/// is walked normally so that calls nested inside generic arguments
/// (`Option<qty!(m)>`, `Quantity<unit!(m), T>`) are reached too.
#[derive(Default)]
struct Expander {
    /// The distinct macro paths that were rewritten, in order of first use.
    expanded_paths: Vec<Path>,
    seen: HashSet<String>,
}

impl VisitMut for Expander {
    fn visit_type_mut(&mut self, ty: &mut Type) {
        if let Type::Macro(TypeMacro { mac }) = ty {
            let name = mac
                .path
                .segments
                .last()
                .map(|segment| segment.ident.to_string());
            let expand: fn(UnitMacroInput) -> Result<TokenStream, TokenStream> =
                match name.as_deref() {
                    Some("unit") => UnitMacroInput::try_expand_unit,
                    Some("qty") => UnitMacroInput::try_expand_quantity,
                    // Some other type macro: nothing to see through, leave it alone.
                    _ => return,
                };
            // On a parse or evaluation failure the call is left untouched: the
            // standalone macro then expands it and reports the error at the
            // offending tokens, exactly as it would without this attribute.
            // (Any derives below still complain about the remaining type macro,
            // but the real diagnosis is there alongside.)
            let expanded = syn::parse2::<UnitMacroInput>(mac.tokens.clone())
                .ok()
                .and_then(|input| expand(input).ok());
            if let Some(expanded) = expanded {
                let path = mac.path.clone();
                if self.seen.insert(quote!(#path).to_string()) {
                    self.expanded_paths.push(path);
                }
                *ty = Type::Verbatim(expanded);
            }
            return;
        }
        visit_mut::visit_type_mut(self, ty);
    }
}
