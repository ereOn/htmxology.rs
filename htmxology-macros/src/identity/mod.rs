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
        validate_html_id(&id_value, id_lit.span())?;

        quote! {
            htmxology::htmx::HtmlId::from_static(#id_lit)
                .expect("ID was validated at compile time")
        }
    } else {
        // Try to parse as with_fn attribute: #[identity(with_fn = "function_name")]
        let meta: syn::MetaNameValue = identity_attr.parse_args()?;

        if !meta.path.is_ident("with_fn") {
            return Err(syn::Error::new_spanned(
                &meta.path,
                "expected 'with_fn' attribute",
            ));
        }

        let fn_name = match &meta.value {
            syn::Expr::Lit(expr_lit) => match &expr_lit.lit {
                syn::Lit::Str(lit_str) => lit_str,
                _ => {
                    return Err(syn::Error::new_spanned(
                        &meta.value,
                        "with_fn must be a string literal",
                    ));
                }
            },
            _ => {
                return Err(syn::Error::new_spanned(
                    &meta.value,
                    "with_fn must be a string literal",
                ));
            }
        };

        let fn_ident = syn::Ident::new(&fn_name.value(), fn_name.span());

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
