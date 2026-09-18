/// Generate the custom_literal module with all unit macros using a custom module name
/// This uses only the canonical data from whippyunits-core
pub fn generate_custom_literal_module_with_name(module_name: &str) -> proc_macro2::TokenStream {
    // Use the single generic function in default mode (no lift trace)
    crate::utils::literal_macros::generate_literal_macros_module(
        module_name,
        false,
        None,
        false,
        syn::Ident::new(module_name, proc_macro2::Span::mixed_site()),
    )
}
