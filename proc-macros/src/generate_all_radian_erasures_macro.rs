use proc_macro2::TokenStream;
use quote::quote;
use syn::Ident;
use syn::parse::{Parse, ParseStream};

/// Input for the generate_all_radian_erasures macro
/// Usage: generate_all_radian_erasures!(max_exponent)
pub struct AllRadianErasuresInput {
    pub max_exponent: syn::LitInt,
}

impl Parse for AllRadianErasuresInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let max_exponent = input.parse()?;
        Ok(AllRadianErasuresInput { max_exponent })
    }
}

impl AllRadianErasuresInput {
    pub fn expand(self) -> TokenStream {
        let max_exponent = self.max_exponent.base10_parse::<i32>().unwrap();

        // Generate radian to scalar conversions (with scale handling)
        let types_with_rescale = [
            ("f32", "rescale_f32"),
            ("f64", "rescale_f64"),
            ("i16", "rescale_i16"),
            ("i32", "rescale_i32"),
            ("i64", "rescale_i64"),
            ("i128", "rescale_i128"),
            ("isize", "rescale_isize"),
        ];

        // Only the marker is repeated per exponent. The conversion bodies are
        // generic over the exponent and retain the numeric-type allowlists below.
        let scalar_exponents: Vec<i32> =
            (-max_exponent..=max_exponent).filter(|&x| x != 0).collect();

        let mut scalar_expansions = Vec::new();

        // Generate same-type conversions
        for (type_name, rescale_fn) in types_with_rescale {
            let type_ident = syn::parse_str::<Ident>(type_name).unwrap();
            let rescale_ident = syn::parse_str::<Ident>(rescale_fn).unwrap();

            let expansion = quote! {
                define_from_for_radians_with_scale!(ERASE_ANGLE_EXPONENT, #type_ident, #rescale_ident);
            };

            scalar_expansions.push(expansion);
        }

        // Generate cross-type conversions for common type pairs
        let cross_type_pairs = [
            (("f32", "rescale_f32"), ("f64", "rescale_f64")),
            (("f64", "rescale_f64"), ("f32", "rescale_f32")),
            (("i32", "rescale_i32"), ("f32", "rescale_f32")),
            (("i32", "rescale_i32"), ("f64", "rescale_f64")),
            (("i64", "rescale_i64"), ("f32", "rescale_f32")),
            (("i64", "rescale_i64"), ("f64", "rescale_f64")),
            (("f32", "rescale_f32"), ("i32", "rescale_i32")),
            (("f64", "rescale_f64"), ("i32", "rescale_i32")),
            (("f32", "rescale_f32"), ("i64", "rescale_i64")),
            (("f64", "rescale_f64"), ("i64", "rescale_i64")),
        ];

        for ((source_type, source_rescale), (target_type, _)) in cross_type_pairs {
            let source_type_ident = syn::parse_str::<Ident>(source_type).unwrap();
            let target_type_ident = syn::parse_str::<Ident>(target_type).unwrap();
            let source_rescale_ident = syn::parse_str::<Ident>(source_rescale).unwrap();

            let expansion = quote! {
                define_from_for_radians_with_scale_cross_type!(ERASE_ANGLE_EXPONENT, #source_type_ident, #target_type_ident, #source_rescale_ident);
            };

            scalar_expansions.push(expansion);
        }

        // Generate radian to dimensionless quantity conversions
        let types = ["f32", "f64", "i16", "i32", "i64", "i128", "isize"];

        let exponent_markers = scalar_exponents.iter().map(|exponent| {
            let exponent = syn::LitInt::new(&exponent.to_string(), proc_macro2::Span::call_site());
            quote! { impl NonzeroRadianExponent for _A<#exponent> {} }
        });
        let type_list = types
            .iter()
            .map(|type_name| syn::parse_str::<Ident>(type_name).unwrap())
            .collect::<Vec<_>>();

        quote! {
            // Private so downstream crates cannot extend the range. Excluding
            // zero keeps scalar erasure separate from dimensionless conversions
            // and compound erasure separate from the blanket From<T> for T.
            trait NonzeroRadianExponent {}
            #(#exponent_markers)*
            #(#scalar_expansions)*
            define_from_for_radians!(ERASE_ANGLE_EXPONENT, #(#type_list),*);
        }
    }
}
