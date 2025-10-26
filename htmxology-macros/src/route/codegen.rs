//! Code generation helpers for the Route derive macro.
//!
//! This module contains reusable functions for generating code from `VariantConfig`.
//! By extracting these helpers, we eliminate duplication between Named and Unnamed
//! field handling and make the code easier to maintain.

use super::config::{FieldRole, FieldsConfig, VariantConfig};
use super::route_type::{MethodExt, RouteType, append_query_arg, to_block};
use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};

/// Generates a match pattern for a variant.
///
/// This creates the pattern used in match arms for Display and method() implementations.
///
/// # Pattern Modes
///
/// - `Simple`: Just the pattern (e.g., `Self::Home` or `Self::User { user_id }`)
/// - `WithWildcard`: Pattern with `..` for partial matching (e.g., `Self::User { .. }`)
///
/// # Examples
///
/// ```ignore
/// // Unit variant
/// Self::Home
///
/// // Named variant with fields
/// Self::User { user_id, page }
///
/// // Named variant with wildcard
/// Self::User { .. }
///
/// // Unnamed variant
/// Self::User(user_id)
///
/// // Unnamed variant with wildcard
/// Self::User(..)
/// ```
pub enum PatternMode {
    /// Full pattern with all field names.
    Simple,
    /// Pattern with `..` wildcard for partial matching.
    WithWildcard,
}

pub fn generate_pattern(config: &VariantConfig, mode: PatternMode) -> TokenStream {
    let ident = &config.ident;

    match (&config.fields, mode) {
        // Unit variants
        (FieldsConfig::Unit, _) => quote! { Self::#ident },

        // Named variants
        (FieldsConfig::Named(fields), PatternMode::Simple) if fields.is_empty() => {
            quote! { Self::#ident {} }
        }
        (FieldsConfig::Named(fields), PatternMode::Simple) => {
            // Include all fields in the pattern, but use _ for body fields
            // since they don't appear in URLs
            let bindings: Vec<_> = fields
                .iter()
                .map(|f| {
                    let ident = &f.ident;
                    if f.is_body() {
                        quote! { #ident: _ }
                    } else {
                        quote! { #ident }
                    }
                })
                .collect();
            if bindings.is_empty() {
                quote! { Self::#ident {} }
            } else {
                quote! { Self::#ident { #(#bindings),* } }
            }
        }
        (FieldsConfig::Named(_), PatternMode::WithWildcard) => {
            quote! { Self::#ident { .. } }
        }

        // Unnamed variants
        (FieldsConfig::Unnamed(fields), PatternMode::Simple) if fields.is_empty() => {
            quote! { Self::#ident() }
        }
        (FieldsConfig::Unnamed(fields), PatternMode::Simple) => {
            // Include all fields in the pattern, but use _ for body fields
            // since they don't appear in URLs
            let bindings: Vec<_> = fields
                .iter()
                .map(|f| {
                    if f.is_body() {
                        quote! { _ }
                    } else {
                        let ident = &f.ident;
                        quote! { #ident }
                    }
                })
                .collect();
            if bindings.is_empty() {
                quote! { Self::#ident() }
            } else {
                quote! { Self::#ident(#(#bindings),*) }
            }
        }
        (FieldsConfig::Unnamed(_), PatternMode::WithWildcard) => {
            quote! { Self::#ident(..) }
        }
    }
}

/// Generates the URL formatting code for the Display implementation.
///
/// This creates the code that formats the route URL with path and query parameters.
///
/// # Example Output
///
/// ```ignore
/// {
///     std::fmt::Write::write_char(f, '/')?;
///     f.write_str("users")?;
///     std::fmt::Write::write_char(f, '/')?;
///     user_id.fmt(f)?;
///     let qs = &serde_html_form::to_string(&page).map_err(|_| std::fmt::Error)?;
///     if !qs.is_empty() {
///         std::fmt::Write::write_char(f, '?')?;
///         f.write_str(&qs)?;
///     }
/// }
/// ```
pub fn generate_url_format(config: &VariantConfig) -> syn::Result<TokenStream> {
    // Collect path parameters
    let path_params: Vec<_> = config.fields.iter().filter(|f| f.is_path_param()).collect();

    // Build parameter name map for named fields
    let param_names = if config.fields.is_named() {
        path_params
            .iter()
            .filter_map(|f| {
                if let FieldRole::PathParam { name } = &f.role {
                    name.as_ref().map(|n| (n.clone(), f.ident.clone()))
                } else {
                    None
                }
            })
            .collect()
    } else {
        // For unnamed fields, collect idents in order
        vec![]
    };

    // Generate URL formatting statements
    let mut statements = if path_params.is_empty() {
        config.route_url.to_unparameterized_string(&config.ident)?
    } else if config.fields.is_named() {
        config
            .route_url
            .to_named_parameters_format(&config.ident, param_names)?
    } else {
        let param_idents: Vec<_> = path_params.iter().map(|f| f.ident.clone()).collect();
        config
            .route_url
            .to_unnamed_parameters_format(&config.ident, param_idents)?
    };

    // Add query parameter formatting if present
    if let Some(query_field) = config.query_param() {
        append_query_arg(&mut statements, Some(&query_field.ident));
    }

    Ok(to_block(statements))
}

/// Generates the request parsing code for the FromRequest implementation.
///
/// This creates the code that extracts path, query, and body parameters from the request.
///
/// # Example Output
///
/// ```ignore
/// {
///     let user_id = htmxology::decode_path_argument(stringify!(user_id), &__captures[stringify!(user_id)])?;
///     let (mut __parts, __body) = __req.into_parts();
///     let axum::extract::Query(page) = axum::extract::Query::from_request_parts(&mut __parts, __state)
///         .await
///         .map_err(|err| err.into_response())?;
///     let __req = http::Request::from_parts(__parts, __body);
///     Self::User { user_id, page }
/// }
/// ```
pub fn generate_request_parsing(config: &VariantConfig) -> TokenStream {
    let path_parse = generate_path_parsing(config);
    let query_parse = generate_query_parsing(config);
    let body_parse = generate_body_parsing(config);
    let construction = generate_variant_construction(config);

    // Check if we need any parsing - if not, just return the construction directly
    let has_parsing = !path_parse.is_empty() || !query_parse.is_empty() || !body_parse.is_empty();

    if has_parsing {
        quote! {
            {
                #path_parse
                #query_parse
                #body_parse
                #construction
            }
        }
    } else {
        construction
    }
}

/// Generates path parameter parsing code.
fn generate_path_parsing(config: &VariantConfig) -> TokenStream {
    let path_params: Vec<_> = config.fields.iter().filter(|f| f.is_path_param()).collect();

    if path_params.is_empty() {
        return quote!();
    }

    if config.fields.is_named() {
        // Named fields: use parameter names
        let parse_stmts: Vec<_> = path_params
            .iter()
            .map(|field| {
                let ident = &field.ident;
                quote! {
                    let #ident = htmxology::decode_path_argument(
                        stringify!(#ident),
                        &__captures[stringify!(#ident)]
                    )?;
                }
            })
            .collect();

        quote! { #(#parse_stmts)* }
    } else {
        // Unnamed fields: use positional indices
        let parse_stmts: Vec<_> = path_params
            .iter()
            .enumerate()
            .map(|(i, field)| {
                let ident = &field.ident;
                let idx = i + 1; // Regex capture groups are 1-indexed
                quote! {
                    let #ident = htmxology::decode_path_argument(
                        stringify!(#ident),
                        &__captures[#idx]
                    )?;
                }
            })
            .collect();

        quote! { #(#parse_stmts)* }
    }
}

/// Generates query parameter parsing code.
fn generate_query_parsing(config: &VariantConfig) -> TokenStream {
    if let Some(query_field) = config.query_param() {
        let ident = &query_field.ident;
        quote! {
            let (mut __parts, __body) = __req.into_parts();
            let axum_extra::extract::Query(#ident) = axum_extra::extract::Query::from_request_parts(
                &mut __parts,
                __state
            )
            .await
            .map_err(|err| err.into_response())?;
            let __req = http::Request::from_parts(__parts, __body);
        }
    } else {
        quote!()
    }
}

/// Generates body parameter parsing code.
fn generate_body_parsing(config: &VariantConfig) -> TokenStream {
    if let Some(body_field) = config.body_param() {
        let ident = &body_field.ident;
        quote! {
            let axum_extra::extract::Form(#ident) = axum_extra::extract::Form::from_request(
                __req,
                __state
            )
            .await
            .map_err(|err| err.into_response())?;
        }
    } else {
        quote!()
    }
}

/// Generates the variant construction expression.
fn generate_variant_construction(config: &VariantConfig) -> TokenStream {
    let ident = &config.ident;

    // Collect all field idents - body fields are still part of the variant,
    // they're just parsed from the request body instead of the URL
    let field_idents: Vec<_> = config.fields.iter().map(|f| &f.ident).collect();

    match &config.fields {
        FieldsConfig::Unit => quote! { Self::#ident },
        FieldsConfig::Named(_) if field_idents.is_empty() => quote! { Self::#ident {} },
        FieldsConfig::Named(_) => quote! { Self::#ident { #(#field_idents),* } },
        FieldsConfig::Unnamed(_) if field_idents.is_empty() => quote! { Self::#ident() },
        FieldsConfig::Unnamed(_) => quote! { Self::#ident(#(#field_idents),*) },
    }
}

/// Generates the method match arm for a variant.
///
/// # Example Output
///
/// ```ignore
/// Self::User { .. } => http::Method::GET
/// ```
pub fn generate_method_match(config: &VariantConfig) -> TokenStream {
    let pattern = generate_pattern(config, PatternMode::WithWildcard);
    let span = config.ident.span();

    match &config.route_type {
        RouteType::Simple { method } => {
            let method_ident = method.to_ident();
            quote_spanned! { span => #pattern => http::Method::#method_ident }
        }
        RouteType::SubRoute => {
            // For subroutes, delegate to the subroute's method
            if let Some(subroute_field) = config.subroute_param() {
                let subroute_ident = &subroute_field.ident;
                let ident = &config.ident;
                match &config.fields {
                    FieldsConfig::Unit => {
                        // Unit variants can't have subroutes
                        unreachable!("Unit variants cannot have subroutes")
                    }
                    FieldsConfig::Named(_) => {
                        quote_spanned! { span => Self::#ident { #subroute_ident, .. } => #subroute_ident.method() }
                    }
                    FieldsConfig::Unnamed(fields) => {
                        // Generate pattern with subroute field in correct position
                        let subroute_idx = fields
                            .iter()
                            .position(|f| f.is_subroute())
                            .expect("subroute field should exist");

                        let pattern_args: Vec<_> = (0..fields.len())
                            .map(|i| {
                                if i == subroute_idx {
                                    quote! { #subroute_ident }
                                } else {
                                    quote! { _ }
                                }
                            })
                            .collect();

                        let ident = &config.ident;
                        quote_spanned! { span => Self::#ident(#(#pattern_args),*) => #subroute_ident.method() }
                    }
                }
            } else {
                // This shouldn't happen if validation worked
                quote_spanned! { span => #pattern => http::Method::GET }
            }
        }
        RouteType::CatchAll => {
            // For catch-all, extract the inner route and delegate
            let ident = &config.ident;
            quote_spanned! { span => Self::#ident(catch_all) => catch_all.method() }
        }
    }
}

/// Generates the Display match arm for a variant.
///
/// # Example Output
///
/// ```ignore
/// Self::User { user_id, page } => {
///     std::fmt::Write::write_char(f, '/')?;
///     f.write_str("users")?;
///     std::fmt::Write::write_char(f, '/')?;
///     user_id.fmt(f)?;
/// }
/// ```
pub fn generate_display_match(config: &VariantConfig) -> syn::Result<TokenStream> {
    let pattern = generate_pattern(config, PatternMode::Simple);
    let url_format = generate_url_format(config)?;
    let span = config.ident.span();

    // For subroutes and catch-all, handle delegation
    if matches!(config.route_type, RouteType::SubRoute)
        && let Some(subroute_field) = config.subroute_param()
    {
        let subroute_ident = &subroute_field.ident;

        // Generate URL format up to the subroute, then delegate
        let path_params: Vec<_> = config.fields.iter().filter(|f| f.is_path_param()).collect();

        let param_names = if config.fields.is_named() {
            path_params
                .iter()
                .filter_map(|f| {
                    if let FieldRole::PathParam { name } = &f.role {
                        name.as_ref().map(|n| (n.clone(), f.ident.clone()))
                    } else {
                        None
                    }
                })
                .collect()
        } else {
            vec![]
        };

        let mut statements = if path_params.is_empty() {
            config.route_url.to_unparameterized_string(&config.ident)?
        } else if config.fields.is_named() {
            config
                .route_url
                .to_named_parameters_format(&config.ident, param_names)?
        } else {
            let param_idents: Vec<_> = path_params.iter().map(|f| f.ident.clone()).collect();
            config
                .route_url
                .to_unnamed_parameters_format(&config.ident, param_idents)?
        };

        // Add subroute delegation
        statements.push(quote! { #subroute_ident.fmt(f)?; });

        let block = to_block(statements);
        return Ok(quote_spanned! { span => #pattern => #block });
    }

    if matches!(config.route_type, RouteType::CatchAll) {
        let ident = &config.ident;
        return Ok(quote_spanned! { span => Self::#ident(catch_all) => catch_all.fmt(f)? });
    }

    Ok(quote_spanned! { span => #pattern => #url_format })
}
