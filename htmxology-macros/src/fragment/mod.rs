//! Fragment derive macro.

use quote::quote;
use syn::spanned::Spanned;

pub fn derive(input: &mut syn::DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let root_ident = &input.ident;
    let root_generics_params = &input.generics.params;
    let root_where_clause = &input.generics.where_clause;

    // Find the #[fragment(...)] attribute
    let fragment_attr = input
        .attrs
        .iter()
        .find(|attr| attr.path().is_ident("fragment"))
        .ok_or_else(|| {
            syn::Error::new(
                input.span(),
                "missing #[fragment(strategy = \"...\")] or #[fragment(with_fn = \"function_name\")] attribute",
            )
        })?;

    // Parse the nested meta to get strategy = "value" or with_fn = "function_name"
    let nested_meta: syn::MetaNameValue = fragment_attr.parse_args()?;

    let strategy_expr = if nested_meta.path.is_ident("strategy") {
        // Direct strategy specification: #[fragment(strategy = "innerHTML")]
        let strategy_lit = match &nested_meta.value {
            syn::Expr::Lit(expr_lit) => match &expr_lit.lit {
                syn::Lit::Str(lit_str) => lit_str,
                _ => {
                    return Err(syn::Error::new_spanned(
                        &nested_meta.value,
                        "strategy must be a string literal",
                    ));
                }
            },
            _ => {
                return Err(syn::Error::new_spanned(
                    &nested_meta.value,
                    "strategy must be a string literal",
                ));
            }
        };

        let strategy_value = strategy_lit.value();

        // Map the string to the appropriate InsertStrategy variant
        // Using exact HTMX strings as documented at https://htmx.org/attributes/hx-swap/
        match strategy_value.as_str() {
            "innerHTML" => quote! { htmxology::htmx::InsertStrategy::InnerHtml },
            "outerHTML" => quote! { htmxology::htmx::InsertStrategy::OuterHtml },
            "textContent" => quote! { htmxology::htmx::InsertStrategy::TextContent },
            "beforebegin" => quote! { htmxology::htmx::InsertStrategy::BeforeBegin },
            "afterbegin" => quote! { htmxology::htmx::InsertStrategy::AfterBegin },
            "beforeend" => quote! { htmxology::htmx::InsertStrategy::BeforeEnd },
            "afterend" => quote! { htmxology::htmx::InsertStrategy::AfterEnd },
            "delete" => quote! { htmxology::htmx::InsertStrategy::Delete },
            "none" => quote! { htmxology::htmx::InsertStrategy::None },
            other => {
                // Allow custom strategies
                quote! { htmxology::htmx::InsertStrategy::Custom(#other.to_string()) }
            }
        }
    } else if nested_meta.path.is_ident("with_fn") {
        // Function-based strategy: #[fragment(with_fn = "get_strategy")]
        let fn_ident = crate::utils::parse_with_fn_attribute(&nested_meta)?;
        quote! { Self::#fn_ident(self) }
    } else {
        return Err(syn::Error::new_spanned(
            &nested_meta.path,
            "expected 'strategy' or 'with_fn' attribute",
        ));
    };

    Ok(quote! {
        impl<#root_generics_params> htmxology::htmx::Fragment for #root_ident<#root_generics_params>
            #root_where_clause
        {
            fn insert_strategy(&self) -> htmxology::htmx::InsertStrategy {
                #strategy_expr
            }
        }
    })
}

#[cfg(test)]
mod snapshot_tests {
    use super::*;
    use crate::utils::testing::test_derive;
    use insta::assert_snapshot;

    fn test_fragment(input: &str) -> String {
        test_derive(input, derive)
    }

    #[test]
    fn outer_html_strategy() {
        let input = r#"
            #[fragment(strategy = "outerHTML")]
            struct MyElement;
        "#;
        assert_snapshot!(test_fragment(input));
    }

    #[test]
    fn inner_html_strategy() {
        let input = r#"
            #[fragment(strategy = "innerHTML")]
            struct Notification {
                message: String,
            }
        "#;
        assert_snapshot!(test_fragment(input));
    }

    #[test]
    fn before_end_strategy() {
        let input = r#"
            #[fragment(strategy = "beforeend")]
            struct ListItem<T> {
                value: T,
            }
        "#;
        assert_snapshot!(test_fragment(input));
    }

    #[test]
    fn custom_strategy() {
        let input = r#"
            #[fragment(strategy = "my-custom-strategy")]
            struct CustomElement;
        "#;
        assert_snapshot!(test_fragment(input));
    }
}
