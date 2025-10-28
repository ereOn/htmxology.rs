//! Identity derive macro.

use quote::quote;
use syn::spanned::Spanned;

/// Validate an HTML ID at compile time.
///
/// Returns an error if the ID is invalid according to HTML5 rules.
fn validate_html_id(id: &str, span: proc_macro2::Span) -> syn::Result<()> {
    if id.is_empty() {
        return Err(syn::Error::new(span, "HTML ID cannot be empty"));
    }

    let mut chars = id.chars();
    let first_char = chars.next().unwrap();

    if !first_char.is_ascii_alphanumeric() && first_char != '_' {
        return Err(syn::Error::new(
            span,
            format!("HTML ID must start with a letter, digit, or underscore, found '{first_char}'"),
        ));
    }

    for c in chars {
        if !(c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == ':' || c == '.') {
            return Err(syn::Error::new(
                span,
                format!(
                    "HTML ID contains invalid character '{c}'. Only letters, digits, hyphens, underscores, colons, and periods are allowed"
                ),
            ));
        }
    }

    Ok(())
}

pub fn derive(input: &mut syn::DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let root_ident = &input.ident;
    let root_generics_params = &input.generics.params;
    let root_where_clause = &input.generics.where_clause;

    // Find the #[identity("...")] attribute
    let identity_attr = input
        .attrs
        .iter()
        .find(|attr| attr.path().is_ident("identity"))
        .ok_or_else(|| syn::Error::new(input.span(), "missing #[identity(\"id\")] attribute"))?;

    // Parse the attribute to extract the ID string
    let id: syn::LitStr = identity_attr.parse_args()?;
    let id_value = id.value();

    // Validate the ID at compile time
    validate_html_id(&id_value, id.span())?;

    Ok(quote! {
        impl<#root_generics_params> htmxology::htmx::Identity for #root_ident<#root_generics_params>
            #root_where_clause
        {
            fn id(&self) -> htmxology::htmx::HtmlId {
                // This is safe because we validated the ID at compile time
                htmxology::htmx::HtmlId::from_static(#id)
                    .expect("ID was validated at compile time")
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_html_id_valid() {
        let span = proc_macro2::Span::call_site();
        assert!(validate_html_id("valid-id", span).is_ok());
        assert!(validate_html_id("valid_id", span).is_ok());
        assert!(validate_html_id("valid:id", span).is_ok());
        assert!(validate_html_id("valid.id", span).is_ok());
        assert!(validate_html_id("_valid", span).is_ok());
        assert!(validate_html_id("1valid", span).is_ok());
    }

    #[test]
    fn test_validate_html_id_invalid() {
        let span = proc_macro2::Span::call_site();
        assert!(validate_html_id("", span).is_err());
        assert!(validate_html_id("-invalid", span).is_err());
        assert!(validate_html_id(".invalid", span).is_err());
        assert!(validate_html_id(":invalid", span).is_err());
        assert!(validate_html_id("invalid id", span).is_err());
        assert!(validate_html_id("invalid$id", span).is_err());
        assert!(validate_html_id("invalid/id", span).is_err());
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
