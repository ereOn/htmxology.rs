//! Fragment derive macro.

use quote::quote;
use syn::spanned::Spanned;

pub fn derive(input: &mut syn::DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let root_ident = &input.ident;
    let root_generics_params = &input.generics.params;
    let root_where_clause = &input.generics.where_clause;

    // Find the #[fragment(strategy = "...")] attribute
    let fragment_attr = input
        .attrs
        .iter()
        .find(|attr| attr.path().is_ident("fragment"))
        .ok_or_else(|| {
            syn::Error::new(
                input.span(),
                "missing #[fragment(strategy = \"...\")] attribute",
            )
        })?;

    // Parse the nested meta to get strategy = "value"
    let nested_meta: syn::MetaNameValue = fragment_attr.parse_args()?;

    // Ensure the key is "strategy"
    if !nested_meta.path.is_ident("strategy") {
        return Err(syn::Error::new_spanned(
            &nested_meta.path,
            "expected 'strategy' attribute",
        ));
    }

    // Extract the string value
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
    let strategy_expr = match strategy_value.as_str() {
        "innerHTML" | "inner_html" => quote! { htmxology::htmx::InsertStrategy::InnerHtml },
        "outerHTML" | "outer_html" => quote! { htmxology::htmx::InsertStrategy::OuterHtml },
        "textContent" | "text_content" => quote! { htmxology::htmx::InsertStrategy::TextContent },
        "beforebegin" | "before_begin" => quote! { htmxology::htmx::InsertStrategy::BeforeBegin },
        "afterbegin" | "after_begin" => quote! { htmxology::htmx::InsertStrategy::AfterBegin },
        "beforeend" | "before_end" => quote! { htmxology::htmx::InsertStrategy::BeforeEnd },
        "afterend" | "after_end" => quote! { htmxology::htmx::InsertStrategy::AfterEnd },
        "delete" => quote! { htmxology::htmx::InsertStrategy::Delete },
        "none" => quote! { htmxology::htmx::InsertStrategy::None },
        other => {
            // Allow custom strategies
            quote! { htmxology::htmx::InsertStrategy::Custom(#other.to_string()) }
        }
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

    #[test]
    fn snake_case_strategy() {
        let input = r#"
            #[fragment(strategy = "outer_html")]
            struct SnakeCaseElement;
        "#;
        assert_snapshot!(test_fragment(input));
    }
}
