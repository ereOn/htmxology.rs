//! Fragment derive macro.

use convert_case::{Case, Casing};
use quote::quote;
use syn::{punctuated::Punctuated, Expr, Token};

mod attributes {
    pub(super) const HTMX: &str = "htmx";
    pub(super) const TARGET: &str = "target";
}

struct HtmxInput {
    target: String,
}

pub(super) fn derive(input: &mut syn::DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let ident = &input.ident;

    let mut htmx = HtmxInput {
        target: format!(
            "#{}",
            ident
                .to_string()
                .from_case(Case::UpperCamel)
                .to_case(Case::Snake)
        ),
    };

    for attr in &input.attrs {
        if attr.path().is_ident(attributes::HTMX) {
            let exprs =
                attr.parse_args_with(Punctuated::<syn::Expr, Token![,]>::parse_terminated)?;

            for expr in exprs {
                parse_htmx_attribute(&mut htmx, expr)?;
            }
        }
    }

    let htmx_target = htmx.target;

    let insert = quote! {
        impl htmx_ssr::htmx::Fragment for #ident {
            fn htmx_target(&self) -> &'static str {
                #htmx_target
            }
        }
    };

    Ok(quote! {
        #insert
    })
}

fn parse_htmx_attribute(htmx: &mut HtmxInput, expr: Expr) -> syn::Result<()> {
    match expr {
        Expr::Assign(expr) => {
            let ident = match expr.left.as_ref() {
                Expr::Path(expr) => expr.path.require_ident()?,
                _ => {
                    return Err(syn::Error::new_spanned(expr.left, "expected path"));
                }
            }
            .to_string();

            match ident.as_str() {
                attributes::TARGET => {
                    let target = match expr.right.as_ref() {
                        Expr::Lit(value) => match &value.lit {
                            syn::Lit::Str(lit) => lit.value(),
                            _ => {
                                return Err(syn::Error::new_spanned(
                                    expr.right,
                                    "expected string literal",
                                ));
                            }
                        },
                        _ => {
                            return Err(syn::Error::new_spanned(
                                expr.right,
                                "expected string literal",
                            ));
                        }
                    };

                    htmx.target = target;
                }
                _ => {
                    return Err(syn::Error::new_spanned(
                        expr.left,
                        format!("unknown attribute `{}`", ident),
                    ));
                }
            };

            Ok(())
        }
        _ => Err(syn::Error::new_spanned(expr, "expected `<attr> = \"...\"`")),
    }
}
