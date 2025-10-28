//! Identity derive macro.

use quote::quote;
use syn::spanned::Spanned;

pub fn derive(input: &mut syn::DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let root_ident = &input.ident;
    let root_generics_params = &input.generics.params;
    let root_param_idents = crate::utils::extract_generic_param_idents(&input.generics.params);
    let root_where_clause = &input.generics.where_clause;

    // Find the #[identity(...)] attribute
    let identity_attr = input
        .attrs
        .iter()
        .find(|attr| attr.path().is_ident("identity"))
        .ok_or_else(|| {
            syn::Error::new(
                input.span(),
                "missing #[identity(id = \"...\")] or #[identity(with_fn = \"Full::path\")] attribute",
            )
        })?;

    // Parse as key=value attribute: #[identity(id = "my-id")] or #[identity(with_fn = "Foo::method")]
    let meta: syn::MetaNameValue = identity_attr.parse_args()?;

    let id_impl = if meta.path.is_ident("id") {
        // Direct ID specification: #[identity(id = "my-id")]
        let id_lit = match &meta.value {
            syn::Expr::Lit(expr_lit) => match &expr_lit.lit {
                syn::Lit::Str(lit_str) => lit_str,
                _ => {
                    return Err(syn::Error::new_spanned(
                        &meta.value,
                        "id must be a string literal",
                    ));
                }
            },
            _ => {
                return Err(syn::Error::new_spanned(
                    &meta.value,
                    "id must be a string literal",
                ));
            }
        };

        let id_value = id_lit.value();
        crate::utils::validate_html_identifier(&id_value, id_lit.span(), "ID")?;

        quote! {
            htmxology::htmx::HtmlId::from_static(#id_lit)
                .expect("ID was validated at compile time")
        }
    } else if meta.path.is_ident("with_fn") {
        // Function-based ID: #[identity(with_fn = "Foo::get_id")]
        let fn_path = crate::utils::parse_with_fn_attribute_as_path(&meta)?;

        quote! {
            #fn_path(self)
        }
    } else {
        return Err(syn::Error::new_spanned(
            &meta.path,
            "expected 'id' or 'with_fn' attribute",
        ));
    };

    Ok(quote! {
        impl<#root_generics_params> htmxology::htmx::Identity for #root_ident<#root_param_idents>
            #root_where_clause
        {
            fn id(&self) -> htmxology::htmx::HtmlId {
                #id_impl
            }
        }
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_validate_html_id_valid() {
        let span = proc_macro2::Span::call_site();
        assert!(crate::utils::validate_html_identifier("valid-id", span, "ID").is_ok());
        assert!(crate::utils::validate_html_identifier("valid_id", span, "ID").is_ok());
        assert!(crate::utils::validate_html_identifier("valid:id", span, "ID").is_ok());
        assert!(crate::utils::validate_html_identifier("valid.id", span, "ID").is_ok());
        assert!(crate::utils::validate_html_identifier("_valid", span, "ID").is_ok());
        assert!(crate::utils::validate_html_identifier("1valid", span, "ID").is_ok());
    }

    #[test]
    fn test_validate_html_id_invalid() {
        let span = proc_macro2::Span::call_site();
        assert!(crate::utils::validate_html_identifier("", span, "ID").is_err());
        assert!(crate::utils::validate_html_identifier("-invalid", span, "ID").is_err());
        assert!(crate::utils::validate_html_identifier(".invalid", span, "ID").is_err());
        assert!(crate::utils::validate_html_identifier(":invalid", span, "ID").is_err());
        assert!(crate::utils::validate_html_identifier("invalid id", span, "ID").is_err());
        assert!(crate::utils::validate_html_identifier("invalid$id", span, "ID").is_err());
        assert!(crate::utils::validate_html_identifier("invalid/id", span, "ID").is_err());
    }
}

#[cfg(test)]
mod snapshot_tests {
    use super::*;
    use crate::utils::testing::test_derive;
    use insta::assert_snapshot;

    fn test_identity(input: &str) -> String {
        test_derive(input, derive)
    }

    #[test]
    fn simple_struct() {
        let input = r#"
            #[identity(id = "my-element")]
            struct MyElement;
        "#;
        assert_snapshot!(test_identity(input));
    }

    #[test]
    fn struct_with_fields() {
        let input = r#"
            #[identity(id = "notification")]
            struct Notification {
                message: String,
            }
        "#;
        assert_snapshot!(test_identity(input));
    }

    #[test]
    fn struct_with_generic() {
        let input = r#"
            #[identity(id = "container")]
            struct Container<T> {
                value: T,
            }
        "#;
        assert_snapshot!(test_identity(input));
    }

    #[test]
    fn struct_with_lifetime() {
        let input = r#"
            #[identity(id = "view")]
            struct View<'a> {
                data: &'a str,
            }
        "#;
        assert_snapshot!(test_identity(input));
    }

    #[test]
    fn generic_with_default() {
        let input = r#"
            #[identity(id = "foo")]
            struct Foo<T = Bar>;
        "#;
        assert_snapshot!(test_identity(input));
    }

    #[test]
    fn generic_with_bounds_and_default() {
        let input = r#"
            #[identity(id = "baz")]
            struct Baz<T: Display = String>;
        "#;
        assert_snapshot!(test_identity(input));
    }

    #[test]
    fn multiple_generics_with_defaults() {
        let input = r#"
            #[identity(id = "multi")]
            struct Multi<T = Foo, U: Clone = Vec<u8>> {
                value: T,
                other: U,
            }
        "#;
        assert_snapshot!(test_identity(input));
    }

    #[test]
    fn mixed_lifetime_and_generic_with_default() {
        let input = r#"
            #[identity(id = "mixed")]
            struct Mixed<'a, T = &'a str> {
                value: T,
            }
        "#;
        assert_snapshot!(test_identity(input));
    }
}
