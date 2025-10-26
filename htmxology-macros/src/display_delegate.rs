//! DisplayDelegate derive macro.

use crate::utils::expect_enum;
use quote::quote;

pub fn derive(input: &mut syn::DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let root_generics_params = &input.generics.params;
    let root_where_clause = &input.generics.where_clause;
    let root_ident = &input.ident;

    let data = expect_enum(input, "DisplayDelegate")?;

    let cases = data
        .variants
        .iter()
        .map(|variant| {
            let variant_ident = &variant.ident;
            quote! {
                Self::#variant_ident(page) => write!(f, "{page}"),
            }
        })
        .collect::<Vec<_>>();

    Ok(quote! {
        impl<#root_generics_params> std::fmt::Display for #root_ident<#root_generics_params>
            #root_where_clause {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self {
                    #(#cases)*
                }
            }
        }
    })
}

#[cfg(test)]
mod snapshot_tests {
    use super::*;
    use crate::utils::testing::test_derive;
    use insta::assert_snapshot;

    fn test_display_delegate(input: &str) -> String {
        test_derive(input, derive)
    }

    #[test]
    fn simple_enum() {
        let input = r#"
            enum Page {
                Home(HomePage),
                About(AboutPage),
                Contact(ContactPage),
            }
        "#;
        assert_snapshot!(test_display_delegate(input));
    }

    #[test]
    fn enum_with_generic() {
        let input = r#"
            enum Result<T> {
                Success(T),
                Error(ErrorPage),
            }
        "#;
        assert_snapshot!(test_display_delegate(input));
    }

    #[test]
    fn enum_with_lifetime() {
        let input = r#"
            enum Cow<'a> {
                Borrowed(&'a str),
                Owned(String),
            }
        "#;
        assert_snapshot!(test_display_delegate(input));
    }

    #[test]
    fn enum_with_where_clause() {
        let input = r#"
            enum Wrapper<T>
            where
                T: std::fmt::Display,
            {
                Value(T),
            }
        "#;
        assert_snapshot!(test_display_delegate(input));
    }

    #[test]
    fn many_variants() {
        let input = r#"
            enum Page {
                Home(HomePage),
                About(AboutPage),
                Contact(ContactPage),
                Blog(BlogPage),
                Shop(ShopPage),
            }
        "#;
        assert_snapshot!(test_display_delegate(input));
    }
}
