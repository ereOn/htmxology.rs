//! DisplayDelegate derive macro.

use quote::quote;
use syn::{Data, Error};

pub(super) fn derive(input: &mut syn::DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
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
