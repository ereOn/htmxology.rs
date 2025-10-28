//! Named derive macro.

use quote::quote;
use syn::spanned::Spanned;

pub fn derive(input: &mut syn::DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let root_ident = &input.ident;
    let root_generics_params = &input.generics.params;
    let root_where_clause = &input.generics.where_clause;

    // Find the #[named(...)] attribute
    let named_attr = input
        .attrs
        .iter()
        .find(|attr| attr.path().is_ident("named"))
        .ok_or_else(|| {
            syn::Error::new(
                input.span(),
                "missing #[named(\"name\")] or #[named(with_fn = \"function_name\")] attribute",
            )
        })?;

    // Try to parse as a simple string literal first
    let name_impl = if let Ok(name_lit) = named_attr.parse_args::<syn::LitStr>() {
        // Direct name specification: #[named("my-name")]
        let name_value = name_lit.value();
        crate::utils::validate_html_identifier(&name_value, name_lit.span(), "name")?;

        quote! {
            htmxology::htmx::HtmlName::from_static(#name_lit)
                .expect("name was validated at compile time")
        }
    } else {
        // Try to parse as with_fn attribute: #[named(with_fn = "function_name")]
        let meta: syn::MetaNameValue = named_attr.parse_args()?;
        let fn_ident = crate::utils::parse_with_fn_attribute(&meta)?;

        quote! {
            Self::#fn_ident(self)
        }
    };

    Ok(quote! {
        impl<#root_generics_params> htmxology::htmx::Named for #root_ident<#root_generics_params>
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
            #[named("my-field")]
            struct MyField;
        "#;
        assert_snapshot!(test_named(input));
    }

    #[test]
    fn struct_with_fields() {
        let input = r#"
            #[named("user-email")]
            struct EmailField {
                value: String,
            }
        "#;
        assert_snapshot!(test_named(input));
    }

    #[test]
    fn struct_with_generic() {
        let input = r#"
            #[named("input-field")]
            struct InputField<T> {
                value: T,
            }
        "#;
        assert_snapshot!(test_named(input));
    }
}
