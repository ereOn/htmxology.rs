//! A route method.

use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::Ident;

/// A route info.
///
/// This is a subset of the standard HTTP methods, that are commonly used in web applications.
#[derive(Debug, Clone)]
pub enum RouteType {
    /// A simple HTTP route.
    Simple {
        /// The route method.
        method: http::Method,
    },

    /// A prefixed sub-route.
    SubRoute,
}

/// An extension trait for `http::Method`.
pub(crate) trait MethodExt {
    /// Convert the method to an identifier.
    fn to_ident(&self) -> TokenStream;
}

impl MethodExt for http::Method {
    fn to_ident(&self) -> TokenStream {
        let method = Ident::new(
            match self {
                &Self::GET => "GET",
                &Self::POST => "POST",
                &Self::PUT => "PUT",
                &Self::DELETE => "DELETE",
                &Self::HEAD => "HEAD",
                &Self::OPTIONS => "OPTIONS",
                &Self::CONNECT => "CONNECT",
                &Self::PATCH => "PATCH",
                &Self::TRACE => "TRACE",
                method => method.as_str(),
            },
            Span::call_site(),
        );

        quote! { #method }
    }
}

/// Append the query argument to the format string.
pub(crate) fn append_query_arg(statements: &mut Vec<TokenStream>, query_arg: Option<&Ident>) {
    if let Some(query_arg) = query_arg {
        statements.push(quote! {
            std::fmt::Write::write_char(f, '?')?;
            let qs = &serde_urlencoded::to_string(&#query_arg).map_err(|_| std::fmt::Error)?;
            f.write_str(&qs)?;
        });
    }
}

/// Serialize the statements to a block.
pub(crate) fn to_block(statements: Vec<TokenStream>) -> TokenStream {
    quote! { {
        #( #statements )*
    } }
}
