//! Identity derive macro.

use quote::quote;
use syn::spanned::Spanned;

pub fn derive(input: &mut syn::DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let root_ident = &input.ident;
    let root_generics_params = &input.generics.params;
    let root_where_clause = &input.generics.where_clause;

    // Find the #[identity(...)] attribute
    let identity_attr = input
        .attrs
        .iter()
        .find(|attr| attr.path().is_ident("identity"))
        .ok_or_else(|| {
            syn::Error::new(
                input.span(),
                "missing #[identity(\"id\")] or #[identity(with_fn = \"function_name\")] attribute",
            )
        })?;

    // Try to parse as a simple string literal first
    let id_impl = if let Ok(id_lit) = identity_attr.parse_args::<syn::LitStr>() {
        // Direct ID specification: #[identity("my-id")]
        let id_value = id_lit.value();
        crate::utils::validate_html_identifier(&id_value, id_lit.span(), "ID")?;

        quote! {
            htmxology::htmx::HtmlId::from_static(#id_lit)
                .expect("ID was validated at compile time")
        }
    } else {
        // Try to parse as with_fn attribute: #[identity(with_fn = "function_name")]
        let meta: syn::MetaNameValue = identity_attr.parse_args()?;
        let fn_ident = crate::utils::parse_with_fn_attribute(&meta)?;

        quote! {
            Self::#fn_ident(self)
        }
    };

    Ok(quote! {
        impl<#root_generics_params> htmxology::htmx::Identity for #root_ident<#root_generics_params>
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
            #[identity("my-element")]
            struct MyElement;
        "#;
        assert_snapshot!(test_identity(input));
    }

    #[test]
    fn struct_with_fields() {
        let input = r#"
            #[identity("notification")]
            struct Notification {
                message: String,
            }
        "#;
        assert_snapshot!(test_identity(input));
    }

    #[test]
    fn struct_with_generic() {
        let input = r#"
            #[identity("container")]
            struct Container<T> {
                value: T,
            }
        "#;
        assert_snapshot!(test_identity(input));
    }

    #[test]
    fn struct_with_lifetime() {
        let input = r#"
            #[identity("view")]
            struct View<'a> {
                data: &'a str,
            }
        "#;
        assert_snapshot!(test_identity(input));
    }
}
