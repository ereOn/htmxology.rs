//! Named derive macro.

use quote::quote;
use syn::spanned::Spanned;

pub fn derive(input: &mut syn::DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let root_ident = &input.ident;
    let root_generics_params = &input.generics.params;
    let root_param_idents = crate::utils::extract_generic_param_idents(&input.generics.params);
    let root_where_clause = &input.generics.where_clause;

    // Find the #[named(...)] attribute
    let named_attr = input
        .attrs
        .iter()
        .find(|attr| attr.path().is_ident("named"))
        .ok_or_else(|| {
            syn::Error::new(
                input.span(),
                "missing #[named(name = \"...\")] or #[named(with_fn = \"Full::path\")] attribute",
            )
        })?;

    // Parse as key=value attribute: #[named(name = "my-name")] or #[named(with_fn = "Foo::method")]
    let meta: syn::MetaNameValue = named_attr.parse_args()?;

    let name_impl = if meta.path.is_ident("name") {
        // Direct name specification: #[named(name = "my-name")]
        let name_lit = match &meta.value {
            syn::Expr::Lit(expr_lit) => match &expr_lit.lit {
                syn::Lit::Str(lit_str) => lit_str,
                _ => {
                    return Err(syn::Error::new_spanned(
                        &meta.value,
                        "name must be a string literal",
                    ));
                }
            },
            _ => {
                return Err(syn::Error::new_spanned(
                    &meta.value,
                    "name must be a string literal",
                ));
            }
        };

        let name_value = name_lit.value();
        crate::utils::validate_html_identifier(&name_value, name_lit.span(), "name")?;

        quote! {
            htmxology::htmx::HtmlName::from_static(#name_lit)
                .expect("name was validated at compile time")
        }
    } else if meta.path.is_ident("with_fn") {
        // Function-based name: #[named(with_fn = "Foo::get_name")]
        let fn_path = crate::utils::parse_with_fn_attribute_as_path(&meta)?;

        quote! {
            #fn_path(self)
        }
    } else {
        return Err(syn::Error::new_spanned(
            &meta.path,
            "expected 'name' or 'with_fn' attribute",
        ));
    };

    Ok(quote! {
        impl<#root_generics_params> htmxology::htmx::Named for #root_ident<#root_param_idents>
            #root_where_clause
        {
            fn name(&self) -> htmxology::htmx::HtmlName {
                #name_impl
            }
        }
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_validate_html_name_valid() {
        let span = proc_macro2::Span::call_site();
        assert!(crate::utils::validate_html_identifier("valid-name", span, "name").is_ok());
        assert!(crate::utils::validate_html_identifier("valid_name", span, "name").is_ok());
        assert!(crate::utils::validate_html_identifier("valid:name", span, "name").is_ok());
        assert!(crate::utils::validate_html_identifier("valid.name", span, "name").is_ok());
        assert!(crate::utils::validate_html_identifier("_valid", span, "name").is_ok());
        assert!(crate::utils::validate_html_identifier("1valid", span, "name").is_ok());
    }

    #[test]
    fn test_validate_html_name_invalid() {
        let span = proc_macro2::Span::call_site();
        assert!(crate::utils::validate_html_identifier("", span, "name").is_err());
        assert!(crate::utils::validate_html_identifier("-invalid", span, "name").is_err());
        assert!(crate::utils::validate_html_identifier(".invalid", span, "name").is_err());
        assert!(crate::utils::validate_html_identifier(":invalid", span, "name").is_err());
        assert!(crate::utils::validate_html_identifier("invalid name", span, "name").is_err());
        assert!(crate::utils::validate_html_identifier("invalid$name", span, "name").is_err());
        assert!(crate::utils::validate_html_identifier("invalid/name", span, "name").is_err());
    }
}

#[cfg(test)]
mod snapshot_tests {
    use super::*;
    use crate::utils::testing::test_derive;
    use insta::assert_snapshot;

    fn test_named(input: &str) -> String {
        test_derive(input, derive)
    }

    #[test]
    fn simple_struct() {
        let input = r#"
            #[named(name = "my-field")]
            struct MyField;
        "#;
        assert_snapshot!(test_named(input));
    }

    #[test]
    fn struct_with_fields() {
        let input = r#"
            #[named(name = "user-email")]
            struct EmailField {
                value: String,
            }
        "#;
        assert_snapshot!(test_named(input));
    }

    #[test]
    fn struct_with_generic() {
        let input = r#"
            #[named(name = "input-field")]
            struct InputField<T> {
                value: T,
            }
        "#;
        assert_snapshot!(test_named(input));
    }

    #[test]
    fn generic_with_default() {
        let input = r#"
            #[named(name = "foo-field")]
            struct FooField<T = Bar>;
        "#;
        assert_snapshot!(test_named(input));
    }

    #[test]
    fn generic_with_bounds_and_default() {
        let input = r#"
            #[named(name = "baz-field")]
            struct BazField<T: Display = String>;
        "#;
        assert_snapshot!(test_named(input));
    }

    #[test]
    fn multiple_generics_with_defaults() {
        let input = r#"
            #[named(name = "multi-field")]
            struct MultiField<T = Foo, U: Clone = Vec<u8>> {
                value: T,
                other: U,
            }
        "#;
        assert_snapshot!(test_named(input));
    }
}
