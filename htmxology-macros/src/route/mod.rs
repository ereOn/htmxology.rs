//! Route derive macro.

use std::collections::BTreeMap;

use crate::utils::expect_enum;
use quote::quote;
use syn::{Error, Expr, Token, Variant, punctuated::Punctuated};

mod codegen;
mod config;
mod route_type;
mod route_url;

pub(crate) use config::{FieldsConfig, VariantConfig};
use route_type::{MethodExt, RouteType};
use route_url::{ParseError, RouteUrl};

mod attributes {
    pub(super) const ROUTE: &str = "route";
    pub(super) const CATCH_ALL: &str = "catch_all";
    pub(super) const METHOD: &str = "method";
    pub(super) const SUBROUTE: &str = "subroute";
    pub(super) const QUERY: &str = "query";
    pub(super) const BODY: &str = "body";
}

pub fn derive(input: &mut syn::DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let root_ident = &input.ident;
    let data = expect_enum(input, "Route")?;

    // Parse all variants into configurations
    let configs: Vec<VariantConfig> = data
        .variants
        .iter()
        .map(VariantConfig::from_variant)
        .collect::<syn::Result<Vec<_>>>()?;

    let mut to_urls = Vec::with_capacity(configs.len());
    let mut methods = Vec::with_capacity(configs.len());

    let mut simple_routes = BTreeMap::new();
    let mut sub_routes = BTreeMap::new();
    let mut get_only_routes = BTreeMap::new(); // For FromStr implementation
    let mut catch_all = quote! {
        Err(http::StatusCode::NOT_FOUND.into_response())
    };

    for config in &configs {
        // Generate Display and method() match arms
        let display_match = codegen::generate_display_match(config)?;
        let method_match = codegen::generate_method_match(config);

        to_urls.push(display_match);
        methods.push(method_match);

        // Generate routing logic based on route type
        match &config.route_type {
            RouteType::Simple { method } => {
                let handler = codegen::generate_request_parsing(config);
                simple_routes
                    .entry(config.route_url.clone())
                    .or_insert_with(Vec::new)
                    .push((method.clone(), handler));

                // Collect GET routes for FromStr
                if method == &http::Method::GET {
                    let from_str_handler = codegen::generate_from_str_parsing(config);
                    get_only_routes
                        .entry(config.route_url.clone())
                        .or_insert(from_str_handler);
                }
            }
            RouteType::SubRoute => {
                let handler = generate_subroute_handler(config)?;
                if sub_routes
                    .insert(config.route_url.clone(), handler)
                    .is_some()
                {
                    return Err(Error::new_spanned(
                        &config.ident,
                        "duplicate subroute prefix",
                    ));
                }
            }
            RouteType::CatchAll => {
                catch_all = generate_catch_all_handler(config)?;
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
        let local_methods: Vec<_> = methods_and_handlers
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
                    #(#local_methods),*,
                    _ => Err(http::StatusCode::METHOD_NOT_ALLOWED.into_response()),
                };
            }
        }});
    }

    // Generate FromStr parsing logic for GET-only routes
    let mut from_str_parsing = Vec::new();

    for (url, handler) in get_only_routes.into_iter().rev() {
        let url = url.to_path_regex();

        from_str_parsing.push(quote! {{
            static RE: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| regex::Regex::new(#url).unwrap());

            // Split path and query string before matching regex
            let (__path, __query_str): (&str, &str) = match __s.split_once('?') {
                Some((p, q)) => (p, q),
                None => (__s, ""),
            };

            if let Some(__captures) = RE.captures(__path) {
                return Ok(#handler);
            }
        }});
    }

    Ok(quote! {
        use axum::response::IntoResponse as _;

        impl htmxology::Route for #root_ident {
            fn method(&self) -> http::Method {
                match self {
                    #(#methods),*
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

        impl std::str::FromStr for #root_ident {
            type Err = htmxology::ParseError;

            fn from_str(__s: &str) -> Result<Self, Self::Err> {
                #(#from_str_parsing)*

                Err(htmxology::ParseError::NoMatchingRoute {
                    url: __s.to_string(),
                })
            }
        }

        impl<S: Send + Sync> axum::extract::FromRequest<S> for #root_ident {
            type Rejection = axum::response::Response;

            async fn from_request(
                __req: axum::extract::Request,
                __state: &S,
            ) -> Result<Self, Self::Rejection> {
                use axum::extract::FromRequestParts;

                #(#parsing)*

                #catch_all
            }
        }
    })
}

/// Generates the handler code for a subroute variant.
fn generate_subroute_handler(config: &VariantConfig) -> syn::Result<proc_macro2::TokenStream> {
    let subroute_field = config
        .subroute_param()
        .expect("SubRoute should have subroute field");

    let subroute_ident = &subroute_field.ident;
    let subroute_ty = &subroute_field.ty;
    let ident = &config.ident;

    // Generate path parameter parsing
    let path_params: Vec<_> = config.fields.iter().filter(|f| f.is_path_param()).collect();

    let path_parse = if path_params.is_empty() {
        quote!()
    } else if config.fields.is_named() {
        let parse_stmts: Vec<_> = path_params
            .iter()
            .map(|field| {
                let field_ident = &field.ident;
                quote! {
                    let #field_ident = htmxology::decode_path_argument(
                        stringify!(#field_ident),
                        &__captures[stringify!(#field_ident)]
                    )?;
                }
            })
            .collect();
        quote! { #(#parse_stmts)* }
    } else {
        let parse_stmts: Vec<_> = path_params
            .iter()
            .enumerate()
            .map(|(i, field)| {
                let field_ident = &field.ident;
                let idx = i + 1;
                quote! {
                    let #field_ident = htmxology::decode_path_argument(
                        stringify!(#field_ident),
                        &__captures[#idx]
                    )?;
                }
            })
            .collect();
        quote! { #(#parse_stmts)* }
    };

    // Generate variant construction
    let construction = match &config.fields {
        FieldsConfig::Named(fields) => {
            let field_idents: Vec<_> = fields.iter().map(|f| &f.ident).collect();
            quote! { Self::#ident { #(#field_idents),* } }
        }
        FieldsConfig::Unnamed(fields) => {
            let field_idents: Vec<_> = fields.iter().map(|f| &f.ident).collect();
            quote! { Self::#ident(#(#field_idents),*) }
        }
        _ => unreachable!("SubRoute must have fields"),
    };

    Ok(quote! {{
        #path_parse

        let __new_path = __captures["subroute"].to_owned();
        let __req = htmxology::replace_request_path(__req, __new_path);

        let #subroute_ident = #subroute_ty::from_request(__req, __state).await?;

        return Ok(#construction);
    }})
}

/// Generates the handler code for a catch-all variant.
fn generate_catch_all_handler(config: &VariantConfig) -> syn::Result<proc_macro2::TokenStream> {
    // Catch-all should have exactly one unnamed field
    if let FieldsConfig::Unnamed(fields) = &config.fields
        && fields.len() == 1
    {
        let field_ty = &fields[0].ty;
        let ident = &config.ident;

        return Ok(quote! {{
            <#field_ty as axum::extract::FromRequest<S>>::from_request(__req, __state)
                .await
                .map(Self::#ident)
        }});
    }

    Err(Error::new_spanned(
        &config.ident,
        "catch-all variant must have exactly one unnamed field",
    ))
}

fn parse_route_info(variant: &Variant) -> syn::Result<(RouteUrl, RouteType)> {
    let mut result = None;

    for attr in &variant.attrs {
        if attr.path().is_ident(attributes::ROUTE) {
            if result.is_some() {
                return Err(Error::new_spanned(
                    attr,
                    format!(
                        "expected exactly one `{}` or `{}` attribute",
                        attributes::ROUTE,
                        attributes::CATCH_ALL
                    ),
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
        } else if attr.path().is_ident(attributes::CATCH_ALL) {
            if result.is_some() {
                return Err(Error::new_spanned(
                    attr,
                    format!(
                        "expected exactly one `{}` or `{}` attribute",
                        attributes::ROUTE,
                        attributes::CATCH_ALL
                    ),
                ));
            }

            if !matches!(attr.meta, syn::Meta::Path(_)) {
                return Err(Error::new_spanned(
                    attr,
                    format!(
                        "`{}` attribute does not take any arguments",
                        attributes::CATCH_ALL
                    ),
                ));
            }

            result = Some((RouteUrl::default(), RouteType::CatchAll));
        }
    }

    result.ok_or_else(|| {
        Error::new_spanned(
            variant,
            format!(
                "expected one `{}` or `{}` attribute",
                attributes::ROUTE,
                attributes::CATCH_ALL
            ),
        )
    })
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

#[cfg(test)]
mod snapshot_tests {
    use super::*;
    use crate::utils::testing::test_derive;
    use insta::assert_snapshot;

    fn test_route_derive(input: &str) -> String {
        test_derive(input, derive)
    }

    #[test]
    fn unit_variant_get() {
        let input = r#"
            enum MyRoute {
                #[route("")]
                Home,
            }
        "#;
        assert_snapshot!(test_route_derive(input));
    }

    #[test]
    fn unit_variant_post() {
        let input = r#"
            enum MyRoute {
                #[route("submit", method = "POST")]
                Submit,
            }
        "#;
        assert_snapshot!(test_route_derive(input));
    }

    #[test]
    fn named_single_path_param() {
        let input = r#"
            enum MyRoute {
                #[route("users/{user_id}")]
                User { user_id: u32 },
            }
        "#;
        assert_snapshot!(test_route_derive(input));
    }

    #[test]
    fn named_multiple_path_params() {
        let input = r#"
            enum MyRoute {
                #[route("users/{user_id}/posts/{post_id}")]
                Post { user_id: u32, post_id: u32 },
            }
        "#;
        assert_snapshot!(test_route_derive(input));
    }

    #[test]
    fn unnamed_single_path_param() {
        let input = r#"
            enum MyRoute {
                #[route("users/{user_id}")]
                User(u32),
            }
        "#;
        assert_snapshot!(test_route_derive(input));
    }

    #[test]
    fn unnamed_multiple_path_params() {
        let input = r#"
            enum MyRoute {
                #[route("users/{user_id}/posts/{post_id}")]
                Post(u32, u32),
            }
        "#;
        assert_snapshot!(test_route_derive(input));
    }

    #[test]
    fn named_query_param() {
        let input = r#"
            enum MyRoute {
                #[route("search")]
                Search {
                    #[query]
                    q: String,
                },
            }
        "#;
        assert_snapshot!(test_route_derive(input));
    }

    #[test]
    fn named_path_and_query() {
        let input = r#"
            enum MyRoute {
                #[route("users/{user_id}/posts")]
                UserPosts {
                    user_id: u32,
                    #[query]
                    page: Option<u32>,
                },
            }
        "#;
        assert_snapshot!(test_route_derive(input));
    }

    #[test]
    fn unnamed_query_param() {
        let input = r#"
            enum MyRoute {
                #[route("search")]
                Search(#[query] String),
            }
        "#;
        assert_snapshot!(test_route_derive(input));
    }

    #[test]
    fn query_param_with_vec() {
        let input = r#"
            enum MyRoute {
                #[route("search")]
                Search {
                    #[query]
                    tags: Vec<String>,
                },
            }
        "#;
        assert_snapshot!(test_route_derive(input));
    }

    #[test]
    fn named_body_param() {
        let input = r#"
            enum MyRoute {
                #[route("submit", method = "POST")]
                Submit {
                    #[body("application/x-www-form-urlencoded")]
                    data: FormData,
                },
            }
        "#;
        assert_snapshot!(test_route_derive(input));
    }

    #[test]
    fn unnamed_body_param() {
        let input = r#"
            enum MyRoute {
                #[route("submit", method = "POST")]
                Submit(#[body("application/x-www-form-urlencoded")] FormData),
            }
        "#;
        assert_snapshot!(test_route_derive(input));
    }

    #[test]
    fn query_and_body_params() {
        let input = r#"
            enum MyRoute {
                #[route("users", method = "POST")]
                CreateUser {
                    #[query]
                    notify: bool,
                    #[body("application/x-www-form-urlencoded")]
                    user_data: UserForm,
                },
            }
        "#;
        assert_snapshot!(test_route_derive(input));
    }

    #[test]
    fn named_subroute() {
        let input = r#"
            enum MyRoute {
                #[route("api/")]
                Api {
                    #[subroute]
                    route: ApiRoute,
                },
            }
        "#;
        assert_snapshot!(test_route_derive(input));
    }

    #[test]
    fn named_subroute_with_path() {
        let input = r#"
            enum MyRoute {
                #[route("users/{user_id}/")]
                UserSubroutes {
                    user_id: u32,
                    #[subroute]
                    route: UserRoute,
                },
            }
        "#;
        assert_snapshot!(test_route_derive(input));
    }

    #[test]
    fn unnamed_subroute() {
        let input = r#"
            enum MyRoute {
                #[route("api/")]
                Api(#[subroute] ApiRoute),
            }
        "#;
        assert_snapshot!(test_route_derive(input));
    }

    #[test]
    fn catch_all() {
        let input = r#"
            enum MyRoute {
                #[route("")]
                Home,
                #[catch_all]
                NotFound(NotFoundRoute),
            }
        "#;
        assert_snapshot!(test_route_derive(input));
    }

    #[test]
    fn full_application() {
        let input = r#"
            enum AppRoute {
                #[route("")]
                Home,

                #[route("users/{user_id}")]
                UserProfile { user_id: u32 },

                #[route("search")]
                Search {
                    #[query]
                    q: String,
                },

                #[route("posts/{post_id}", method = "DELETE")]
                DeletePost { post_id: u32 },

                #[route("login", method = "POST")]
                Login {
                    #[body("application/x-www-form-urlencoded")]
                    credentials: LoginForm,
                },

                #[route("admin/")]
                Admin {
                    #[subroute]
                    route: AdminRoute,
                },

                #[catch_all]
                NotFound(NotFoundRoute),
            }
        "#;
        assert_snapshot!(test_route_derive(input));
    }
}
