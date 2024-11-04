//! Controller derive macro.

use quote::{quote, quote_spanned};
use syn::{punctuated::Punctuated, spanned::Spanned, Data, Error, Expr, Fields, Ident, Token};

mod attributes {
    pub(super) const URL: &str = "url";
    pub(super) const PATH: &str = "path";
    pub(super) const METHOD: &str = "method";
    pub(super) const QUERY: &str = "query";
}

struct RouteInfo {
    url: String,
    method: Ident,
}

pub(super) fn derive(input: &mut syn::DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let root_ident = &input.ident;

    let data = match &input.data {
        Data::Struct(_) => {
            return Err(Error::new_spanned(
                root_ident,
                "can't derive Controller for a struct",
            ));
        }
        Data::Enum(data_enum) => data_enum,
        Data::Union(_) => {
            return Err(Error::new_spanned(
                root_ident,
                "can't derive Controller for a union",
            ));
        }
    };

    let mut routes = Vec::with_capacity(data.variants.len());
    let mut declarations = Vec::with_capacity(data.variants.len());

    for variant in &data.variants {
        let ident = &variant.ident;

        let handler = match &variant.fields {
            // Enum::Unit
            Fields::Unit => {
                quote_spanned! { variant.span() =>
                    |
                        axum::extract::State(state): axum::extract::State<htmx_ssr::State<_>>,
                        htmx: htmx_ssr::htmx::Request,
                    | async move {
                        htmx_ssr::ViewMapper::render_view(#root_ident::#ident, state, htmx).await
                    }
                }
            }
            // Enum::Named{}
            Fields::Named(fields) if fields.named.is_empty() => {
                quote_spanned! { variant.span() =>
                    |
                        axum::extract::State(state): axum::extract::State<htmx_ssr::State<_>>,
                        htmx: htmx_ssr::htmx::Request,
                    | async move {
                        htmx_ssr::ViewMapper::render_view(#root_ident::#ident{}, state, htmx).await
                    }
                }
            }
            // Enum::Unnamed()
            Fields::Unnamed(fields) if fields.unnamed.is_empty() => {
                quote_spanned! { variant.span() =>
                    |
                        axum::extract::State(state): axum::extract::State<htmx_ssr::State<_>>,
                        htmx: htmx_ssr::htmx::Request,
                    | async move {
                        htmx_ssr::ViewMapper::render_view(#root_ident::#ident(), state, htmx).await
                    }
                }
            }
            // Enum::Named{...}
            Fields::Named(fields) => {
                let mut args_defs = Vec::with_capacity(fields.named.len());
                let mut path_args = Vec::with_capacity(fields.named.len());
                let mut query_args = Vec::with_capacity(fields.named.len());
                let mut query_args_defs = Vec::with_capacity(fields.named.len());

                for field in &fields.named {
                    let field_ident = field
                        .ident
                        .as_ref()
                        .expect("field of named variant has no ident");
                    let field_ty = &field.ty;

                    args_defs.push(quote_spanned! { field_ident.span() =>
                        #field_ident: #field_ty
                    });

                    match field
                        .attrs
                        .iter()
                        .find(|attr| attr.path().is_ident(attributes::QUERY))
                    {
                        Some(_) => {
                            query_args.push(quote_spanned! { field_ident.span() =>
                                #field_ident
                            });
                            query_args_defs.push(quote_spanned! { field_ident.span() =>
                                #[serde(default)]
                                #field_ident: #field_ty
                            });
                        }
                        None => {
                            path_args.push(quote_spanned! { field_ident.span() =>
                                #field_ident
                            });
                        }
                    }
                }

                let args = path_args
                    .clone()
                    .into_iter()
                    .chain(query_args.clone().into_iter())
                    .collect::<Vec<_>>();

                let params = Ident::new(&format!("{root_ident}{ident}Params"), ident.span());

                let path_parse = if path_args.is_empty() {
                    quote!()
                } else {
                    quote! {
                        let axum::extract::Path((#(#path_args),*)) = axum::extract::Path::from_request_parts(parts, state)
                            .await
                            .map_err(|err| err.into_response())?;
                    }
                };

                let query_parse = if query_args.is_empty() {
                    quote!()
                } else {
                    quote! {
                        let axum::extract::Query(query) =
                        axum::extract::Query::from_request_parts(parts, state)
                            .await
                            .map_err(|err| err.into_response())?;

                        #[derive(serde::Deserialize)]
                        struct Query {
                            #(#query_args_defs),*
                        }

                        let Query { #(#query_args),* } = query;
                    }
                };

                declarations.push(quote! {
                    struct #params {
                        #(#args_defs),*
                    }

                    #[axum::async_trait]
                    impl<S: Send + Sync> axum::extract::FromRequestParts<S> for #params {
                        type Rejection = axum::response::Response;

                        async fn from_request_parts(
                            parts: &mut http::request::Parts,
                            state: &S,
                        ) -> Result<Self, Self::Rejection> {
                            #path_parse
                            #query_parse

                            Ok(Self { #(#args),* })
                        }
                    }

                    impl From<#params> for #root_ident {
                        fn from(#params{ #(#args),* }: #params) -> Self {
                            Self::#ident { #(#args),* }
                        }
                    }
                });

                quote_spanned! { variant.span() =>
                    |
                        axum::extract::State(state): axum::extract::State<htmx_ssr::State<_>>,
                        htmx: htmx_ssr::htmx::Request,
                        params: #params,
                    | async move {
                        htmx_ssr::ViewMapper::render_view(#root_ident::from(params), state, htmx).await
                    }
                }
            }
            // Enum::Unnamed(...)
            Fields::Unnamed(fields) => {
                let params = fields.unnamed.iter().map(|field| {
                    let ty = &field.ty;
                    quote_spanned! {field.span() => #ty}
                });

                let params = quote_spanned! {fields.span() => (#(#params),*)};

                quote_spanned! { variant.span() =>
                    |
                        axum::extract::State(state): axum::extract::State<htmx_ssr::State<_>>,
                        htmx: htmx_ssr::htmx::Request,
                        params: #params,
                    | async move{
                        htmx_ssr::ViewMapper::render_view(#root_ident::from(params), state, htmx).await
                    }
                }
            }
        };

        for attr in &variant.attrs {
            if attr.path().is_ident(attributes::URL) {
                let exprs =
                    attr.parse_args_with(Punctuated::<syn::Expr, Token![,]>::parse_terminated)?;

                let RouteInfo { url, method } = parse_url_attribute(ident, exprs)?;

                routes.push(quote! {
                    .route(#url, axum::routing::#method(#handler))
                });
            }
        }
    }

    Ok(quote! {
        #(#declarations)*

        impl htmx_ssr::Controller for #root_ident {
            type Model = <#root_ident as htmx_ssr::ViewMapper>::Model;

            fn register_routes(
                router: axum::Router<htmx_ssr::State<Self::Model>>,
            ) -> axum::Router<htmx_ssr::State<Self::Model>> {
                router
                    #(#routes)*
            }
        }
    })
}

fn parse_url_attribute(
    ident: &Ident,
    exprs: impl IntoIterator<Item = Expr>,
) -> syn::Result<RouteInfo> {
    let mut exprs = exprs.into_iter();

    let first = match exprs.next() {
        Some(first) => first,
        None => {
            return Err(Error::new_spanned(ident, "expected at least one argument"));
        }
    };

    let url = match first {
        Expr::Lit(expr) => match expr.lit {
            syn::Lit::Str(lit_str) => lit_str.value(),
            _ => {
                return Err(Error::new_spanned(expr.lit, "expected a string literal"));
            }
        },
        Expr::Assign(expr) => {
            let left = match *expr.left {
                Expr::Path(expr) => expr.path.require_ident()?.to_string(),
                expr => {
                    return Err(Error::new_spanned(expr, "expected path"));
                }
            };

            if left != attributes::PATH {
                return Err(Error::new_spanned(left, "expected `path`"));
            }

            match *expr.right {
                Expr::Lit(expr) => match expr.lit {
                    syn::Lit::Str(lit_str) => lit_str.value(),
                    _ => {
                        return Err(Error::new_spanned(expr.lit, "expected a string literal"));
                    }
                },
                _ => {
                    return Err(Error::new_spanned(expr.right, "expected a string literal"));
                }
            }
        }
        _ => {
            return Err(Error::new_spanned(first, "expected a string literal"));
        }
    };

    let mut method = Ident::new("get", ident.span());

    for expr in exprs {
        match expr {
            Expr::Assign(expr) => {
                let left = match *expr.left {
                    Expr::Path(expr) => expr.path.require_ident()?.to_string(),
                    expr => {
                        return Err(Error::new_spanned(expr, "expected path"));
                    }
                };

                match left.as_str() {
                    attributes::METHOD => {
                        method = match *expr.right {
                            Expr::Path(expr) => expr.path.require_ident()?.clone(),
                            expr => {
                                return Err(Error::new_spanned(expr, "expected path"));
                            }
                        };
                    }
                    _ => {
                        return Err(Error::new_spanned(left, "expected `method`"));
                    }
                }
            }
            _ => {
                return Err(Error::new_spanned(expr, "expected `<foo> = <bar>`"));
            }
        }
    }

    Ok(RouteInfo { url, method })
}
