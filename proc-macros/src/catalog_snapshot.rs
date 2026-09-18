//! Developer-only export of the fixed default catalog. The normal build uses
//! the checked-in files, while custom declarators still use procedural macros.

use std::collections::BTreeMap;

use quote::ToTokens;
use syn::{GenericArgument, Item, PathArguments, ReturnType, TraitItem, Type};

fn exponent(expr: &syn::Expr) -> i16 {
    match expr {
        syn::Expr::Lit(syn::ExprLit {
            lit: syn::Lit::Int(n),
            ..
        }) => n.base10_parse().unwrap(),
        syn::Expr::Unary(syn::ExprUnary {
            op: syn::UnOp::Neg(_),
            expr,
            ..
        }) => -exponent(expr),
        syn::Expr::Block(block) if block.block.stmts.len() == 1 => match &block.block.stmts[0] {
            syn::Stmt::Expr(expr, None) => exponent(expr),
            other => panic!(
                "unsupported exponent statement: {}",
                other.to_token_stream()
            ),
        },
        other => panic!("unsupported exponent: {}", other.to_token_stream()),
    }
}

fn dimension(ty: &Type, aliases: &BTreeMap<String, Type>, depth: usize) -> Option<[i16; 8]> {
    assert!(depth < 32, "cyclic generated catalog alias");
    let Type::Path(path) = ty else { return None };
    let segment = path.path.segments.last().unwrap();
    if segment.ident == "Dimension" {
        let PathArguments::AngleBracketed(args) = &segment.arguments else {
            panic!("dimension arguments")
        };
        let result: Vec<_> = args
            .args
            .iter()
            .map(|arg| {
                let GenericArgument::Type(Type::Path(path)) = arg else {
                    panic!("dimension axis")
                };
                let PathArguments::AngleBracketed(args) =
                    &path.path.segments.last().unwrap().arguments
                else {
                    panic!("axis exponent")
                };
                let GenericArgument::Const(expr) = args.args.first().unwrap() else {
                    panic!("constant axis exponent")
                };
                exponent(expr)
            })
            .collect();
        return Some(result.try_into().unwrap());
    }
    if let Some(alias) = aliases.get(&segment.ident.to_string()) {
        return dimension(alias, aliases, depth + 1);
    }
    if let PathArguments::AngleBracketed(args) = &segment.arguments {
        for arg in &args.args {
            if let GenericArgument::Type(ty) = arg {
                if let Some(dim) = dimension(ty, aliases, depth + 1) {
                    return Some(dim);
                }
            }
        }
    }
    None
}

fn features(dim: [i16; 8]) -> Vec<&'static str> {
    let mut result = Vec::new();
    if [dim[0], dim[1], dim[2], dim[7]] != [0; 4] {
        result.push("units-dynamics");
    }
    for (index, feature) in [
        (3, "units-electrical"),
        (4, "units-thermal"),
        (5, "units-chemical"),
        (6, "units-photometric"),
    ] {
        if dim[index] != 0 {
            result.push(feature);
        }
    }
    result
}

/// Invoked by tools/regenerate-default-catalogs.py for the full catalog and both
/// lean dynamics selections. It is ignored during
/// ordinary tests because exporting artifacts is an explicit developer action.
#[test]
#[ignore = "developer catalog export; use tools/regenerate-default-catalogs.py"]
fn export_default_catalog() {
    let output = std::path::PathBuf::from(
        std::env::var_os("WHIPPY_CATALOG_OUT").expect("WHIPPY_CATALOG_OUT"),
    );
    let mode = std::env::var("WHIPPY_CATALOG_MODE").unwrap();
    assert!(matches!(mode.as_str(), "full" | "coherent" | "conversions"));
    let lean = mode != "full";
    let generated = super::DefaultDeclaratorsInput.expand();
    let parsed: syn::File =
        syn::parse2(generated).expect("default catalog emits complete Rust items");
    let aliases: BTreeMap<_, _> = parsed
        .items
        .iter()
        .filter_map(|item| match item {
            Item::Type(item) => Some((item.ident.to_string(), (*item.ty).clone())),
            _ => None,
        })
        .collect();
    let trait_dimensions: BTreeMap<_, _> = parsed
        .items
        .iter()
        .filter_map(|item| match item {
            Item::Trait(item) => {
                let dim = item
                    .items
                    .iter()
                    .find_map(|item| match item {
                        TraitItem::Fn(method) => match &method.sig.output {
                            ReturnType::Type(_, ty) => dimension(ty, &aliases, 0),
                            _ => None,
                        },
                        _ => None,
                    })
                    .unwrap_or_else(|| panic!("no unit dimension in {}", item.ident));
                Some((item.ident.to_string(), dim))
            }
            _ => None,
        })
        .collect();

    let mut groups: BTreeMap<String, (Vec<&str>, proc_macro2::TokenStream)> = BTreeMap::new();
    for item in parsed.items {
        let dim = match &item {
            Item::Type(item) => dimension(&item.ty, &aliases, 0).expect("unit alias dimension"),
            Item::Trait(item) => trait_dimensions[&item.ident.to_string()],
            Item::Impl(item) => {
                let path = &item
                    .trait_
                    .as_ref()
                    .expect("extension trait implementation")
                    .1;
                trait_dimensions[&path.segments.last().unwrap().ident.to_string()]
            }
            Item::Macro(item)
                if item.mac.path.segments.last().unwrap().ident == "generate_literals_module" =>
            {
                continue;
            }
            other => panic!(
                "unexpected generated catalog item: {}",
                other.to_token_stream()
            ),
        };
        let required = features(dim);
        let name = if lean {
            if mode == "conversions" {
                "dynamics_conversions"
            } else {
                "si_dynamics"
            }
            .to_owned()
        } else if required.is_empty() {
            "dimensionless".to_owned()
        } else {
            required
                .iter()
                .map(|name| name.strip_prefix("units-").unwrap())
                .collect::<Vec<_>>()
                .join("_")
        };
        let group = groups
            .entry(name)
            .or_insert_with(|| (required, proc_macro2::TokenStream::new()));
        item.to_tokens(&mut group.1);
    }

    std::fs::create_dir_all(&output).unwrap();
    let header = "// @generated by tools/regenerate-default-catalogs.py; do not edit by hand.\n\n";
    let mut index = header.to_owned();
    for (name, (required, tokens)) in groups {
        std::fs::write(
            output.join(format!("{name}.rs")),
            format!("{header}{tokens}\n"),
        )
        .unwrap();
        if !lean {
            if !required.is_empty() {
                index.push_str(&format!(
                    "#[cfg(all({}))]\n",
                    required
                        .iter()
                        .map(|f| format!("feature = {f:?}"))
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
            }
            index.push_str(&format!("include!({:?});\n\n", format!("{name}.rs")));
        }
    }
    if !lean {
        index.push_str("#[cfg(all(feature = \"units-si-dynamics\", not(feature = \"units-dynamics\"), not(feature = \"units-dynamics-conversions\")))]\ninclude!(\"si_dynamics.rs\");\n\n");
        index.push_str("#[cfg(all(feature = \"units-dynamics-conversions\", not(feature = \"units-dynamics\")))]\ninclude!(\"dynamics_conversions.rs\");\n");
        std::fs::write(output.join("mod.rs"), index).unwrap();
    }
}

fn enabled_meta(meta: &syn::Meta) -> bool {
    match meta {
        syn::Meta::NameValue(value) if value.path.is_ident("feature") => {
            let syn::Expr::Lit(syn::ExprLit {
                lit: syn::Lit::Str(name),
                ..
            }) = &value.value
            else {
                panic!("feature string")
            };
            match name.value().as_str() {
                "units-dynamics" => cfg!(feature = "units-dynamics"),
                "units-si-dynamics" => cfg!(feature = "units-si-dynamics"),
                "units-dynamics-conversions" => cfg!(feature = "units-dynamics-conversions"),
                "units-electrical" => cfg!(feature = "units-electrical"),
                "units-thermal" => cfg!(feature = "units-thermal"),
                "units-chemical" => cfg!(feature = "units-chemical"),
                "units-photometric" => cfg!(feature = "units-photometric"),
                other => panic!("unknown catalog feature {other}"),
            }
        }
        syn::Meta::List(list) => {
            use syn::parse::Parser;
            let args = syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated
                .parse2(list.tokens.clone())
                .unwrap();
            if list.path.is_ident("all") {
                args.iter().all(enabled_meta)
            } else if list.path.is_ident("not") {
                assert_eq!(args.len(), 1);
                !enabled_meta(args.first().unwrap())
            } else {
                panic!("unsupported cfg predicate")
            }
        }
        _ => panic!("unsupported cfg attribute"),
    }
}

fn normalized_item(mut item: Item) -> String {
    // rustfmt inserts optional trailing commas into multiline generic argument
    // lists. Normalize those punctuation choices before comparing declarations.
    struct Normalize;
    impl syn::visit_mut::VisitMut for Normalize {
        fn visit_angle_bracketed_generic_arguments_mut(
            &mut self,
            args: &mut syn::AngleBracketedGenericArguments,
        ) {
            syn::visit_mut::visit_angle_bracketed_generic_arguments_mut(self, args);
            args.args.pop_punct();
        }
        fn visit_signature_mut(&mut self, signature: &mut syn::Signature) {
            syn::visit_mut::visit_signature_mut(self, signature);
            signature.inputs.pop_punct();
        }
        fn visit_expr_call_mut(&mut self, call: &mut syn::ExprCall) {
            syn::visit_mut::visit_expr_call_mut(self, call);
            call.args.pop_punct();
        }
    }
    syn::visit_mut::VisitMut::visit_item_mut(&mut Normalize, &mut item);
    item.to_token_stream().to_string()
}

#[test]
fn checked_in_catalog_matches_active_features() {
    let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    // A packaged proc-macro crate does not contain its sibling whippyunits
    // source tree. Snapshot consistency is a repository-level contract.
    if !manifest.join("../Cargo.toml").is_file() {
        return;
    }
    let directory = manifest.join("../src/generated/default_declarators");
    let index =
        syn::parse_file(&std::fs::read_to_string(directory.join("mod.rs")).unwrap()).unwrap();
    let mut actual = Vec::new();
    for item in index.items {
        let Item::Macro(item) = item else {
            panic!("generated index contains only includes")
        };
        assert!(item.mac.path.is_ident("include"));
        if item.attrs.iter().any(|attr| {
            attr.path().is_ident("cfg") && !enabled_meta(&attr.parse_args::<syn::Meta>().unwrap())
        }) {
            continue;
        }
        let name: syn::LitStr = syn::parse2(item.mac.tokens).unwrap();
        let parsed =
            syn::parse_file(&std::fs::read_to_string(directory.join(name.value())).unwrap())
                .unwrap();
        actual.extend(parsed.items.into_iter().map(normalized_item));
    }
    let generated: syn::File = syn::parse2(super::DefaultDeclaratorsInput.expand()).unwrap();
    let mut expected: Vec<_> = generated.items.into_iter().filter(|item| !matches!(item, Item::Macro(item) if item.mac.path.segments.last().unwrap().ident == "generate_literals_module")).map(normalized_item).collect();
    actual.sort();
    expected.sort();
    assert_eq!(actual.len(), expected.len(), "generated item count differs");
    for (actual, expected) in actual.iter().zip(&expected) {
        assert!(
            actual == expected,
            "checked-in catalog differs for this feature selection near {:?}; run tools/regenerate-default-catalogs.py",
            actual.chars().take(200).collect::<String>()
        );
    }
}
