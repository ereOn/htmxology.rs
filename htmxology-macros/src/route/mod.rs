//! Route derive macro.

use std::collections::BTreeMap;

use quote::{format_ident, quote, quote_spanned};
use syn::{
    punctuated::Punctuated, spanned::Spanned, Data, Error, Expr, Fields, Ident, Token, Variant,
};

mod route_type;
mod route_url;

use route_type::{append_query_arg, to_block, MethodExt, RouteType};
use route_url::{ParseError, RouteUrl};

mod attributes {
    pub(super) const ROUTE: &str = "route";
    pub(super) const METHOD: &str = "method";
    pub(super) const SUBROUTE: &str = "subroute";
    pub(super) const QUERY: &str = "query";
    pub(super) const BODY: &str = "body";
}

pub(super) fn derive(input: &mut syn::DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let root_ident = &input.ident;

    let data = match &input.data {
        Data::Struct(_) => {
            return Err(Error::new_spanned(
                root_ident,
                "can't derive Route for a struct",
            ));
        }
        Data::Enum(data_enum) => data_enum,
        Data::Union(_) => {
            return Err(Error::new_spanned(
                root_ident,
                "can't derive Route for a union",
            ));
        }
    };

    let mut to_urls = Vec::with_capacity(data.variants.len());
    let mut to_methods = Vec::with_capacity(data.variants.len());

    let mut simple_routes = BTreeMap::new();
    let mut sub_routes = BTreeMap::new();

    for variant in &data.variants {
        let ident = &variant.ident;

        let (url, route_type) = parse_route_info(variant)?;

        match route_type {
            RouteType::Simple { method } => {
                let handler = match &variant.fields {
                    // Enum::Unit - no query or body parameters.
                    Fields::Unit => {
                        let url = to_block(url.to_unparameterized_string(variant)?);

                        to_urls.push(quote! {
                            Self::#ident => #url
                        });

                        let method_ident = method.to_ident();
                        to_methods.push(
                            quote_spanned! { variant.span() => Self::#ident => http::Method::#method_ident },
                        );

                        quote_spanned! { variant.span() => #root_ident::#ident }
                    }
                    // Enum::Named{} - no query or body parameters.
                    Fields::Named(fields) if fields.named.is_empty() => {
                        let url = to_block(url.to_unparameterized_string(variant)?);

                        to_urls.push(quote! {
                            Self::#ident{} => #url
                        });

                        let method_ident = method.to_ident();
                        to_methods.push(
                            quote_spanned! { variant.span() => Self::#ident{} => http::Method::#method_ident },
                        );

                        quote_spanned! { variant.span() => #root_ident::#ident{} }
                    }
                    // Enum::Unnamed() - no query or body parameters.
                    Fields::Unnamed(fields) if fields.unnamed.is_empty() => {
                        let url = to_block(url.to_unparameterized_string(variant)?);

                        to_urls.push(quote! {
                            Self::#ident() => #url
                        });

                        let method_ident = method.to_ident();
                        to_methods.push(
                            quote_spanned! { variant.span() => Self::#ident() => http::Method::#method_ident },
                        );

                        quote_spanned! { variant.span() => #root_ident::#ident() }
                    }
                    // Enum::Named{...}
                    Fields::Named(fields) => {
                        // Will contain all the arguments.
                        let mut args = Vec::with_capacity(fields.named.len());
                        let mut args_defs = Vec::with_capacity(fields.named.len());

                        // All the arguments used for URL formatting.
                        let mut url_args = Vec::with_capacity(fields.named.len());

                        // All the path arguments.
                        let mut path_args = Vec::with_capacity(fields.named.len());
                        let mut path_args_names = BTreeMap::new();

                        // If there is a query argument, this will be set to its ident.
                        let mut query_arg = None;

                        // If there is a body argument, this will be set to its ident.
                        let mut body_arg = None;

                        for field in &fields.named {
                            let field_ident = field
                                .ident
                                .as_ref()
                                .expect("field of named variant has no ident");
                            let field_ty = &field.ty;

                            args.push(quote_spanned! { field_ident.span() =>
                                #field_ident
                            });
                            args_defs.push(quote_spanned! { field_ident.span() =>
                                #field_ident: #field_ty
                            });

                            let is_query = field
                                .attrs
                                .iter()
                                .any(|attr| attr.path().is_ident(attributes::QUERY));

                            let is_body = field
                                .attrs
                                .iter()
                                .any(|attr| attr.path().is_ident(attributes::BODY));

                            if is_body {
                                url_args.push(quote! { .. });
                            } else {
                                url_args.push(quote_spanned! { field_ident.span() =>
                                    #field_ident
                                });
                            }

                            match (is_query, is_body) {
                                (true, true) => {
                                    return Err(Error::new_spanned(
                                        field,
                                        "field cannot be both query and body parameter",
                                    ));
                                }
                                (true, false) => {
                                    if query_arg.is_some() {
                                        return Err(Error::new_spanned(
                                            field,
                                            "only one field can be a query parameter",
                                        ));
                                    }

                                    query_arg = Some(field_ident.clone());
                                }
                                (false, true) => {
                                    if body_arg.is_some() {
                                        return Err(Error::new_spanned(
                                            field,
                                            "only one field can be a body parameter",
                                        ));
                                    }

                                    body_arg = Some(field_ident.clone());
                                }
                                (false, false) => {
                                    path_args.push(quote_spanned! { field_ident.span() =>
                                        #field_ident
                                    });
                                    path_args_names
                                        .insert(field_ident.to_string(), field_ident.clone());
                                }
                            };
                        }

                        let url = {
                            let mut statements = if path_args_names.is_empty() {
                                url.to_unparameterized_string(variant)?
                            } else {
                                url.to_named_parameters_format(variant, path_args_names)?
                            };

                            append_query_arg(&mut statements, query_arg.as_ref());
                            to_block(statements)
                        };

                        to_urls.push(quote! {
                            Self::#ident{#(#url_args),*} => #url
                        });

                        let method_ident = method.to_ident();
                        to_methods.push(
                            quote_spanned! { variant.span() => Self::#ident{..} => http::Method::#method_ident },
                        );

                        let path_parse = if path_args.is_empty() {
                            quote!()
                        } else {
                            let mut args_parse = Vec::with_capacity(path_args.len());
                            for path_arg in &path_args {
                                args_parse.push(quote! {
                                    let #path_arg = htmxology::decode_path_argument(stringify!(#path_arg), &__captures[stringify!(#path_arg)])?;
                                });
                            }

                            quote! {
                                #(#args_parse)*
                            }
                        };

                        let query_parse = match query_arg {
                            None => quote!(),
                            Some(query_ident) => {
                                quote! {
                                    let (mut __parts, __body) = __req.into_parts();
                                    let axum::extract::Query(#query_ident) =
                                    axum::extract::Query::from_request_parts(&mut __parts, __state)
                                        .await
                                        .map_err(|err| err.into_response())?;
                                    let __req = http::Request::from_parts(__parts, __body);
                                }
                            }
                        };

                        let body_parse = match body_arg {
                            None => quote!(),
                            Some(body_ident) => {
                                quote! {
                                    let axum::extract::Form(#body_ident) =
                                    axum::extract::Form::from_request(__req, __state)
                                        .await
                                        .map_err(|err| err.into_response())?;
                                }
                            }
                        };

                        quote_spanned! { variant.span() => {
                            #path_parse
                            #query_parse
                            #body_parse

                            Self::#ident { #(#args),* }
                        }}
                    }
                    // Enum::Unnamed(...)
                    Fields::Unnamed(fields) => {
                        // Will contain all the arguments.
                        let mut args = Vec::with_capacity(fields.unnamed.len());
                        let mut args_defs = Vec::with_capacity(fields.unnamed.len());

                        // All the arguments used for URL formatting.
                        let mut url_args = Vec::with_capacity(fields.unnamed.len());

                        // All the path arguments.
                        let mut path_args = Vec::with_capacity(fields.unnamed.len());
                        let mut path_args_unnamed = Vec::with_capacity(fields.unnamed.len());

                        // If there is a query argument, this will be set to its ident.
                        let mut query_arg = None;

                        // If there is a body argument, this will be set to its ident.
                        let mut body_arg = None;

                        for (i, field) in fields.unnamed.iter().enumerate() {
                            let field_ident = Ident::new(&format!("arg{i}"), field.span());
                            let field_ty = &field.ty;

                            args.push(quote_spanned! { field_ident.span() =>
                                #field_ident
                            });
                            args_defs.push(quote_spanned! { field_ident.span() =>
                                #field_ident: #field_ty
                            });

                            let is_query = field
                                .attrs
                                .iter()
                                .any(|attr| attr.path().is_ident(attributes::QUERY));

                            let is_body = field
                                .attrs
                                .iter()
                                .any(|attr| attr.path().is_ident(attributes::BODY));

                            if is_body {
                                url_args.push(quote! { .. });
                            } else {
                                url_args.push(quote_spanned! { field_ident.span() =>
                                    #field_ident
                                });
                            }

                            match (is_query, is_body) {
                                (true, true) => {
                                    return Err(Error::new_spanned(
                                        field,
                                        "field cannot be both query and body parameter",
                                    ));
                                }
                                (true, false) => {
                                    if query_arg.is_some() {
                                        return Err(Error::new_spanned(
                                            field,
                                            "only one field can be a query parameter",
                                        ));
                                    }

                                    query_arg = Some(field_ident.clone());
                                }
                                (false, true) => {
                                    if body_arg.is_some() {
                                        return Err(Error::new_spanned(
                                            field,
                                            "only one field can be a body parameter",
                                        ));
                                    }

                                    body_arg = Some(field_ident.clone());
                                }
                                (false, false) => {
                                    path_args.push(quote_spanned! { field_ident.span() =>
                                        #field_ident
                                    });
                                    path_args_unnamed.push(field_ident.clone());
                                }
                            };
                        }

                        let url = {
                            let mut statements = if path_args.is_empty() {
                                url.to_unparameterized_string(variant)?
                            } else {
                                url.to_unnamed_parameters_format(variant, path_args_unnamed)?
                            };

                            append_query_arg(&mut statements, query_arg.as_ref());
                            to_block(statements)
                        };

                        to_urls.push(quote! {
                            Self::#ident(#(#url_args),*) => #url
                        });

                        let method_ident = method.to_ident();
                        to_methods.push(
                            quote_spanned! { variant.span() => Self::#ident(..) => http::Method::#method_ident },
                        );

                        let path_parse = if path_args.is_empty() {
                            quote!()
                        } else {
                            let mut args_parse = Vec::with_capacity(path_args.len());
                            for (i, path_arg) in path_args.iter().enumerate() {
                                let idx = i + 1;
                                args_parse.push(quote! {
                                    let #path_arg = htmxology::decode_path_argument(stringify!(#path_arg), &__captures[#idx])?;
                                });
                            }

                            quote! {
                                #(#args_parse)*
                            }
                        };

                        let query_parse = match query_arg {
                            None => quote!(),
                            Some(query_ident) => {
                                quote! {
                                    let (mut __parts, __body) = __req.into_parts();
                                    let axum::extract::Query(#query_ident) =
                                    axum::extract::Query::from_request_parts(&mut __parts, __state)
                                        .await
                                        .map_err(|err| err.into_response())?;
                                    let __req = http::Request::from_parts(__parts, __body);
                                }
                            }
                        };

                        let body_parse = match body_arg {
                            None => quote!(),
                            Some(body_ident) => {
                                quote! {
                                    let axum::extract::Form(#body_ident) =
                                    axum::extract::Form::from_request(__req, __state)
                                        .await
                                        .map_err(|err| err.into_response())?;
                                }
                            }
                        };

                        quote_spanned! { variant.span() => {
                            #path_parse
                            #query_parse
                            #body_parse

                            Self::#ident ( #(#args),* )
                        }}
                    }
                };

                simple_routes
                    .entry(url)
                    .or_insert_with(Vec::new)
                    .push((method, handler));
            }
            RouteType::SubRoute => {
                let router = match &variant.fields {
                    // Enum::Unit
                    Fields::Unit => {
                        return Err(Error::new_spanned(
                            variant,
                            "expected struct or tuple variant",
                        ))
                    }
                    // Enum::Named{...}
                    Fields::Named(fields) => {
                        // Will contain all the arguments.
                        let mut args = Vec::with_capacity(fields.named.len());
                        let mut args_defs = Vec::with_capacity(fields.named.len());

                        // All the path arguments.
                        let mut path_args = Vec::with_capacity(fields.named.len());
                        let mut path_args_names = BTreeMap::new();

                        let mut subroute_arg = None;

                        for field in &fields.named {
                            let field_ident = field
                                .ident
                                .as_ref()
                                .expect("field of named variant has no ident");
                            let field_ty = &field.ty;

                            args.push(quote_spanned! { field_ident.span() =>
                                #field_ident
                            });
                            args_defs.push(quote_spanned! { field_ident.span() =>
                                #field_ident: #field_ty
                            });

                            let is_subroute = field
                                .attrs
                                .iter()
                                .any(|attr| attr.path().is_ident(attributes::SUBROUTE));

                            if is_subroute {
                                if subroute_arg.is_some() {
                                    return Err(Error::new_spanned(
                                        field,
                                        "only one field can be a subroute",
                                    ));
                                }

                                subroute_arg = Some(field_ident.clone());
                            } else {
                                path_args.push(quote_spanned! { field_ident.span() =>
                                    #field_ident
                                });
                                path_args_names
                                    .insert(field_ident.to_string(), field_ident.clone());
                            }
                        }

                        let subroute_arg = subroute_arg.ok_or_else(|| {
                            Error::new_spanned(
                                variant,
                                "expected a field with `subroute` attribute",
                            )
                        })?;

                        let url = {
                            let mut statements = if path_args_names.is_empty() {
                                url.to_unparameterized_string(variant)?
                            } else {
                                url.to_named_parameters_format(variant, path_args_names)?
                            };

                            statements.push(quote! {
                                #subroute_arg.fmt(f)?;
                            });

                            to_block(statements)
                        };

                        to_methods.push(quote_spanned! { variant.span() => Self::#ident{#subroute_arg, ..} => #subroute_arg.method()});

                        to_urls.push(quote! {
                            Self::#ident{#(#args),*} => #url
                        });

                        let path_parse = if path_args.is_empty() {
                            quote!()
                        } else {
                            let mut args_parse = Vec::with_capacity(path_args.len());
                            for path_arg in &path_args {
                                args_parse.push(quote! {
                                    let #path_arg = htmxology::decode_path_argument(stringify!(#path_arg), &__captures[stringify!(#path_arg)])?;
                                });
                            }

                            quote! {
                                #(#args_parse)*
                            }
                        };

                        quote_spanned! { variant.span() => {
                            #path_parse

                            let __new_path = __captures["subroute"].to_owned();
                            let __req = htmxology::replace_request_path(__req, __new_path);

                            let #subroute_arg = axum::extract::FromRequest::from_request(__req, __state)
                                .await?;

                            return Ok(Self::#ident { #(#args),* });
                        }}
                    }
                    // Enum::Unnamed(...)
                    Fields::Unnamed(fields) => {
                        // Will contain all the arguments.
                        let mut args = Vec::with_capacity(fields.unnamed.len());
                        let mut args_defs = Vec::with_capacity(fields.unnamed.len());

                        // All the path arguments.
                        let mut path_args = Vec::with_capacity(fields.unnamed.len());
                        let mut path_args_unnamed = Vec::with_capacity(fields.unnamed.len());

                        let mut subroute_arg = None;

                        for (i, field) in fields.unnamed.iter().enumerate() {
                            let field_ident = format_ident!("arg{i}");
                            let field_ty = &field.ty;

                            args.push(quote_spanned! { field_ident.span() =>
                                #field_ident
                            });
                            args_defs.push(quote_spanned! { field_ident.span() =>
                                #field_ident: #field_ty
                            });

                            let is_subroute = field
                                .attrs
                                .iter()
                                .any(|attr| attr.path().is_ident(attributes::SUBROUTE));

                            if is_subroute {
                                if subroute_arg.is_some() {
                                    return Err(Error::new_spanned(
                                        field,
                                        "only one field can be a subroute",
                                    ));
                                }

                                subroute_arg = Some((i, field_ident.clone()));
                            } else {
                                path_args.push(quote_spanned! { field_ident.span() =>
                                    #field_ident
                                });
                                path_args_unnamed.push(field_ident.clone());
                            }
                        }

                        let (subroute_arg_idx, subroute_arg) = subroute_arg.ok_or_else(|| {
                            Error::new_spanned(
                                variant,
                                "expected a field with `subroute` attribute",
                            )
                        })?;

                        let url = {
                            let mut statements = if path_args.is_empty() {
                                url.to_unparameterized_string(variant)?
                            } else {
                                url.to_unnamed_parameters_format(variant, path_args_unnamed)?
                            };

                            statements.push(quote! {
                                #subroute_arg.fmt(f)?;
                            });

                            to_block(statements)
                        };

                        to_urls.push(quote! {
                            Self::#ident(#(#args),*) => #url
                        });

                        let margs = (0..args.len()).map(|i| {
                            if i == subroute_arg_idx {
                                quote! { #subroute_arg }
                            } else {
                                quote! { _ }
                            }
                        });

                        to_methods.push(quote_spanned! { variant.span() => Self::#ident(#(#margs),*) => #subroute_arg.method() });

                        let path_parse = if path_args.is_empty() {
                            quote!()
                        } else {
                            let mut args_parse = Vec::with_capacity(path_args.len());
                            for path_arg in &path_args {
                                args_parse.push(quote! {
                                    let #path_arg = htmxology::decode_path_argument(stringify!(#path_arg), &__captures[stringify!(#path_arg)])?;
                                });
                            }

                            quote! {
                                #(#args_parse)*
                            }
                        };

                        quote_spanned! { variant.span() => {
                            #path_parse

                            let __new_path = __captures["subroute"].to_owned();
                            let __req = htmxology::replace_request_path(__req, __new_path);

                            let #subroute_arg = axum::extract::FromRequest::from_request(__req, __state)
                                .await?;

                            return Ok(Self::#ident ( #(#args),* ));
                        }}
                    }
                };

                if sub_routes.insert(url, router).is_some() {
                    return Err(Error::new_spanned(variant, "duplicate subroute prefix"));
                }
            }
        }
    }

    let mut parsing = Vec::with_capacity(data.variants.len());

    // We add the subroutes first, so that they are matched before the simple routes.
    for (prefix, handler) in sub_routes.into_iter().rev() {
        let prefix = prefix.to_path_regex();

        parsing.push(quote! {{
            static RE: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| regex::Regex::new(#prefix).unwrap());

            if let Some(__captures) = RE.captures(&__req.uri().path()) {
                #handler
            }
        }});
    }

    // Then we add the simple routes, with more specific routes first.
    for (url, methods_and_handlers) in simple_routes.into_iter().rev() {
        let url = url.to_path_regex();
        let methods: Vec<_> = methods_and_handlers
            .into_iter()
            .map(|(method, handler)| {
                let method = method.to_ident();

                quote! {&http::Method::#method => Ok(#handler)}
            })
            .collect();

        parsing.push(quote! {{
            static RE: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| regex::Regex::new(#url).unwrap());

            if let Some(__captures) = RE.captures(&__req.uri().path()) {
                return match __req.method() {
                    #(#methods),*,
                    _ => Err(http::StatusCode::METHOD_NOT_ALLOWED.into_response()),
                };
            }
        }});
    }

    Ok(quote! {
        impl htmxology::Route for #root_ident {
            fn method(&self) -> http::Method {
                match self {
                    #(#to_methods),*
                }
            }
        }

        impl std::fmt::Display for #root_ident {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self {
                    #(#to_urls),*
                };

                Ok(())
            }
        }

        #[axum::async_trait]
        impl<S: Send + Sync> axum::extract::FromRequest<S> for #root_ident {
            type Rejection = axum::response::Response;

            async fn from_request(
                __req: axum::extract::Request,
                __state: &S,
            ) -> Result<Self, Self::Rejection> {
                use axum::extract::FromRequestParts;

                #(#parsing)*

                Err(http::StatusCode::NOT_FOUND.into_response())
            }
        }
    })
}

fn parse_route_info(variant: &Variant) -> syn::Result<(RouteUrl, RouteType)> {
    let mut result = None;

    for attr in &variant.attrs {
        if attr.path().is_ident(attributes::ROUTE) {
            if result.is_some() {
                return Err(Error::new_spanned(
                    attr,
                    format!("expected at most one `{}` attribute", attributes::ROUTE,),
                ));
            }

            let mut exprs = attr
                .parse_args_with(Punctuated::<syn::Expr, Token![,]>::parse_terminated)?
                .into_iter();

            let raw_url = exprs.next().ok_or_else(|| {
                Error::new_spanned(attr, "expected a route URL as the first argument")
            })?;

            let url = parse_route_url(raw_url)?;

            let route_type = if url.is_prefix() {
                RouteType::SubRoute
            } else {
                let method = match exprs.next() {
                    Some(raw_method) => parse_method(raw_method)?,
                    None => http::Method::GET,
                };

                RouteType::Simple { method }
            };

            if exprs.next().is_none() {
                result = Some((url, route_type));
            } else {
                return Err(Error::new_spanned(attr, "expected at most two arguments"));
            }
        }
    }

    result.ok_or_else(|| Error::new_spanned(variant, "expected `route` or `subroute` attribute"))
}

fn parse_raw_url(expr: &Expr) -> syn::Result<String> {
    match expr {
        Expr::Lit(expr) => match expr.lit {
            syn::Lit::Str(ref lit_str) => Ok(lit_str.value()),
            _ => Err(Error::new_spanned(
                expr.lit.clone(),
                "expected a string literal",
            )),
        },
        _ => Err(Error::new_spanned(expr, "expected a string literal")),
    }
}

fn parse_route_url(expr: Expr) -> syn::Result<RouteUrl> {
    let url = parse_raw_url(&expr)?;

    url.parse()
        .map_err(|err: ParseError| Error::new_spanned(expr, format!("{err}\n{}", err.detail(&url))))
}

fn parse_method(expr: Expr) -> syn::Result<http::Method> {
    match expr {
        Expr::Assign(expr) => {
            let left = match *expr.left {
                Expr::Path(expr) => expr.path.require_ident()?.to_string(),
                expr => {
                    return Err(Error::new_spanned(expr, "expected path"));
                }
            };

            match left.as_str() {
                attributes::METHOD => match *expr.right {
                    Expr::Lit(expr) => match expr.lit {
                        syn::Lit::Str(ref lit_str) => lit_str
                            .value()
                            .parse()
                            .map_err(|_| Error::new_spanned(expr, "invalid HTTP method")),
                        _ => Err(Error::new_spanned(expr, "expected string literal")),
                    },
                    expr => Err(Error::new_spanned(expr, "expected path")),
                },
                _ => Err(Error::new_spanned(
                    left,
                    format!("expected `{}`", attributes::METHOD),
                )),
            }
        }
        _ => Err(Error::new_spanned(
            expr,
            format!("expected `{} = \"<GET|POST|...>\"`", attributes::METHOD),
        )),
    }
}
