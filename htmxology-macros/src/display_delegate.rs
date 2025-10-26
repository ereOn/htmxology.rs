//! DisplayDelegate derive macro.

use quote::quote;
use syn::{Data, Error};

pub fn derive(input: &mut syn::DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let root_generics_params = &input.generics.params;
    let root_where_clause = &input.generics.where_clause;
    let root_ident = &input.ident;

    let data = match &input.data {
        Data::Struct(_) => {
            return Err(Error::new_spanned(
                root_ident,
                "can't derive DisplayDelegate for a struct",
            ));
        }
        Data::Enum(data_enum) => data_enum,
        Data::Union(_) => {
            return Err(Error::new_spanned(
                root_ident,
                "can't derive DisplayDelegate for a union",
            ));
        }
    };

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
    use insta::assert_snapshot;
    use quote::quote;

    fn test_display_delegate(input: &str) -> String {
        let mut input: syn::DeriveInput = syn::parse_str(input).expect("Failed to parse input");
        let output = derive(&mut input).expect("Derive failed");

        let wrapped = quote! {
            #[allow(unused)]
            mod __test {
                #output
            }
        };

        let syntax_tree: syn::File = syn::parse2(wrapped).expect("Failed to parse output");
        prettyplease::unparse(&syntax_tree)
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
