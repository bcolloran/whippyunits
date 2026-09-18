//! `#[expand_units]`: rewrites `unit!`/`qty!`/`vqty!` calls in the type positions
//! of an item into the types they expand to, so that `#[derive]`s written below
//! the attribute see ordinary types rather than macro calls.
//!
//! The built-in derives walk the field types of a *generic* item looking for
//! projections (`T::Assoc`) that need extra `where` bounds, and refuse outright
//! when that walk meets an unexpanded macro ("`derive` cannot be used on items
//! with type macros"). Attributes expand top to bottom, so an attribute placed
//! above the derives gets to rewrite the item before they run.

use std::collections::HashSet;

use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::visit_mut::{self, VisitMut};
use syn::{Expr, Ident, Item, Path, Token, Type, TypeMacro};
use whippyunits_core::UnitExpr;

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

/// Replaces each supported type macro with its expansion. Every other type is
/// walked normally so that calls nested inside generic arguments
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
            // On a parse or evaluation failure the call is left untouched: the
            // standalone macro then expands it and reports the error at the
            // offending tokens, exactly as it would without this attribute.
            // (Any derives below still complain about the remaining type macro,
            // but the real diagnosis is there alongside.)
            let expanded = match name.as_deref() {
                Some("unit") => syn::parse2::<UnitMacroInput>(mac.tokens.clone())
                    .ok()
                    .and_then(|input| input.try_expand_unit().ok()),
                Some("qty") => syn::parse2::<UnitMacroInput>(mac.tokens.clone())
                    .ok()
                    .and_then(|input| input.try_expand_quantity().ok()),
                Some("vqty") => syn::parse2::<VqtyMacroInput>(mac.tokens.clone())
                    .ok()
                    .and_then(|input| input.try_expand().ok()),
                // Some other type macro: nothing to see through, leave it alone.
                _ => return,
            };
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

enum VectorUnits {
    Uniform(UnitExpr),
    Mixed(Vec<UnitExpr>),
}

/// Mirrors whippyalgebra's `vqty!` grammar without depending on that crate:
/// whippyalgebra already depends on whippyunits and its procedural macros.
struct VqtyMacroInput {
    count: Expr,
    units: VectorUnits,
    storage: Type,
}

impl Parse for VqtyMacroInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let count = input.parse()?;
        input.parse::<Token![,]>()?;
        let is_list = input.peek(syn::token::Bracket);
        let units = if is_list {
            let content;
            syn::bracketed!(content in input);
            let units = Punctuated::<UnitExpr, Token![,]>::parse_terminated(&content)?;
            if units.is_empty() {
                return Err(content.error("vqty!: expected at least one unit"));
            }
            VectorUnits::Mixed(units.into_iter().collect())
        } else {
            VectorUnits::Uniform(input.parse()?)
        };

        let storage = if input.peek(Token![,]) {
            input.parse::<Token![,]>()?;
            // Only the bracketed form allows a trailing comma without an
            // explicit storage type, matching the standalone macro.
            if is_list && input.is_empty() {
                syn::parse_quote!(T)
            } else {
                let storage = input.parse()?;
                if input.peek(Token![,]) {
                    input.parse::<Token![,]>()?;
                }
                storage
            }
        } else {
            syn::parse_quote!(T)
        };

        Ok(Self {
            count,
            units,
            storage,
        })
    }
}

impl VqtyMacroInput {
    fn try_expand(self) -> Result<TokenStream, TokenStream> {
        // `Itself` includes examples and integration tests, which require an
        // absolute crate name rather than `crate::`. The library provides a
        // self alias for this same path when expanding within its own code.
        let algebra = match proc_macro_crate::crate_name("whippyalgebra") {
            Ok(proc_macro_crate::FoundCrate::Name(name)) => {
                let ident = Ident::new(&name, Span::call_site());
                quote!(::#ident)
            }
            Ok(proc_macro_crate::FoundCrate::Itself) | Err(_) => quote!(::whippyalgebra),
        };
        let expand_unit = |unit_expr| {
            UnitMacroInput {
                unit_expr,
                storage_type: None,
                brand_type: None,
            }
            .try_expand_unit()
        };
        let Self {
            count,
            units,
            storage,
        } = self;
        match units {
            VectorUnits::Uniform(unit) => {
                let unit = expand_unit(unit)?;
                Ok(quote! {
                    #algebra::nalgebra::UniformUnitMatrix<
                        #unit,
                        #algebra::nalgebra::__backend::SVector<#storage, { #count }>,
                    >
                })
            }
            VectorUnits::Mixed(units) => {
                // Expand each list entry too: leaving a dims! or unit! call
                // nested in this type would still prevent generic derives.
                let mut dims = quote!(#algebra::dims::DNil);
                for unit in units.into_iter().rev() {
                    let unit = expand_unit(unit)?;
                    dims = quote!(#algebra::dims::DCons<#unit, #dims>);
                }
                Ok(quote! {
                    #algebra::nalgebra::UnitVector<
                        #dims,
                        #algebra::Dimensionless,
                        #algebra::nalgebra::__backend::SVector<
                            #storage,
                            {
                                #algebra::nalgebra::__vqty_count(
                                    #count,
                                    <#dims as #algebra::DimList>::LEN,
                                )
                            },
                        >,
                    >
                })
            }
        }
    }
}
