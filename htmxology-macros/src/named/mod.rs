//! Named derive macro.

use quote::quote;
use syn::spanned::Spanned;

/// Validate an HTML name at compile time.
///
/// Returns an error if the name is invalid according to HTML5 rules.
fn validate_html_name(name: &str, span: proc_macro2::Span) -> syn::Result<()> {
    if name.is_empty() {
        return Err(syn::Error::new(span, "HTML name cannot be empty"));
    }

    let mut chars = name.chars();
    let first_char = chars.next().unwrap();

    if !first_char.is_ascii_alphanumeric() && first_char != '_' {
        return Err(syn::Error::new(
            span,
            format!(
                "HTML name must start with a letter, digit, or underscore, found '{first_char}'"
            ),
        ));
    }

    for c in chars {
        if !(c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == ':' || c == '.') {
            return Err(syn::Error::new(
                span,
                format!(
                    "HTML name contains invalid character '{c}'. Only letters, digits, hyphens, underscores, colons, and periods are allowed"
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
        validate_html_name(&name_value, name_lit.span())?;

        quote! {
            htmxology::htmx::HtmlName::from_static(#name_lit)
                .expect("name was validated at compile time")
        }
    } else {
        // Try to parse as with_fn attribute: #[named(with_fn = "function_name")]
        let meta: syn::MetaNameValue = named_attr.parse_args()?;

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
    use super::*;

    #[test]
    fn test_validate_html_name_valid() {
        let span = proc_macro2::Span::call_site();
        assert!(validate_html_name("valid-name", span).is_ok());
        assert!(validate_html_name("valid_name", span).is_ok());
        assert!(validate_html_name("valid:name", span).is_ok());
        assert!(validate_html_name("valid.name", span).is_ok());
        assert!(validate_html_name("_valid", span).is_ok());
        assert!(validate_html_name("1valid", span).is_ok());
    }

    #[test]
    fn test_validate_html_name_invalid() {
        let span = proc_macro2::Span::call_site();
        assert!(validate_html_name("", span).is_err());
        assert!(validate_html_name("-invalid", span).is_err());
        assert!(validate_html_name(".invalid", span).is_err());
        assert!(validate_html_name(":invalid", span).is_err());
        assert!(validate_html_name("invalid name", span).is_err());
        assert!(validate_html_name("invalid$name", span).is_err());
        assert!(validate_html_name("invalid/name", span).is_err());
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
