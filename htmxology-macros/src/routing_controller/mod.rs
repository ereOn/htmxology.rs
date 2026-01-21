//! Route derive macro.

use quote::{quote, quote_spanned};
use syn::{
    GenericArgument, Ident, Lifetime, LitStr, Token, Type, TypePath, TypeReference,
    parse::{Parse, ParseStream},
    parse_quote,
    punctuated::Punctuated,
    spanned::Spanned,
};

pub(super) const CONTROLLER: &str = "controller";
pub(super) const SUBCONTROLLER: &str = "subcontroller";
pub(super) const ROUTE: &str = "route";
pub(super) const PATH: &str = "path";
pub(super) const DOC: &str = "doc";
pub(super) const CONVERT_WITH: &str = "convert_with";
pub(super) const CONVERT_RESPONSE: &str = "convert_response";
pub(super) const PARAMS: &str = "params";
pub(super) const RESPONSE: &str = "response";
pub(super) const ARGS: &str = "args";
pub(super) const PRE_HANDLER: &str = "pre_handler";
pub(super) const EXTRA_DERIVES: &str = "extra_derives";

pub fn derive(input: &mut syn::DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    // Get the name of the root type.
    let root_ident = &input.ident;

    let mut as_subcontroller_impls = Vec::new();
    let mut route_variants = Vec::new();
    let mut handle_request_variants = Vec::new();

    let mut controller_spec: Option<ControllerSpec> = None;
    let mut default_subcontroller_route: Option<Ident> = None;

    // Let's iterate over the top-level `subcontroller` attributes.
    for attr in &input.attrs {
        if attr.path().is_ident(SUBCONTROLLER) {
            let spec: SubcontrollerSpec = attr.parse_args()?;

            // Check for multiple default subcontrollers (without `path`)
            if spec.path.is_none() {
                if let Some(ref existing) = default_subcontroller_route {
                    return Err(syn::Error::new_spanned(
                        &spec.route_variant,
                        format!(
                            "only one default subcontroller (without `path`) is allowed; \
                             `{}` is already defined as the default",
                            existing
                        ),
                    ));
                }
                default_subcontroller_route = Some(spec.route_variant.clone());
            }

            as_subcontroller_impls.push((spec.as_subcontroller_impl_fn)(root_ident));

            let route_variant = &spec.route_variant;
            let controller_type = &spec.controller_type;
            let convert_response_fn = &spec.convert_response_fn;

            let doc_attr = if let Some(doc) = &spec.doc {
                quote_spanned! { spec.controller_type.span() =>
                    #[doc = #doc]
                }
            } else {
                quote! {}
            };

            // Generate route variant - tuple variant if no params, struct variant if params
            route_variants.push(if spec.params.is_empty() {
                // No params - simple tuple variant
                match spec.path {
                    Some(path) => {
                        quote_spanned! { spec.route_variant.span() =>
                            #doc_attr
                            #[route(#path)]
                            #route_variant(#[subroute] <#controller_type as htmxology::Controller>::Route),
                        }
                    }
                    None => {
                        quote_spanned! { spec.route_variant.span() =>
                            #doc_attr
                            #[catch_all]
                            #route_variant(<#controller_type as htmxology::Controller>::Route),
                        }
                    }
                }
            } else {
                // Has params - struct variant with param fields
                let param_fields = spec.params.iter().map(|p| {
                    let name = &p.name;
                    let ty = &p.ty;
                    quote! { #name: #ty }
                });

                match spec.path {
                    Some(path) => {
                        quote_spanned! { spec.route_variant.span() =>
                            #doc_attr
                            #[route(#path)]
                            #route_variant {
                                #(#param_fields,)*
                                #[subroute]
                                subroute: <#controller_type as htmxology::Controller>::Route,
                            },
                        }
                    }
                    None => {
                        quote_spanned! { spec.route_variant.span() =>
                            #doc_attr
                            #[catch_all]
                            #route_variant {
                                #(#param_fields,)*
                                subroute: <#controller_type as htmxology::Controller>::Route,
                            },
                        }
                    }
                }
            });

            let controller_type = remove_lifetimes(controller_type);

            // Generate the conversion logic based on whether a custom function was specified
            let conversion_logic = if let Some(fn_expr) = convert_response_fn {
                // Custom function specified - pass all handle_request parameters
                // Note: parts_for_convert and args_for_convert will be cloned before handle_request is called
                // The first parameter is &self to allow access to controller state
                quote! {
                    #fn_expr(self, &htmx, &parts_for_convert, server_info, &args_for_convert, response)
                }
            } else {
                // Default: use Into trait
                quote! {
                    response.into()
                }
            };

            // Generate handle_request match arm
            handle_request_variants.push(if spec.params.is_empty() {
                // No params - simple tuple variant, pass parent args through
                if convert_response_fn.is_some() {
                    // If using custom convert_response, clone values for convert_response before moving
                    quote_spanned! { spec.route_variant.span() =>
                        Self::Route::#route_variant(route) => {
                            let parts_for_convert = parts.clone();
                            let args_for_convert = args.clone();
                            let response = htmxology::SubcontrollerExt::get_subcontroller::<#controller_type>(self)
                                .handle_request(route, htmx.clone(), parts, server_info, args)
                                .await;
                            #conversion_logic
                        }
                    }
                } else {
                    quote_spanned! { spec.route_variant.span() =>
                        Self::Route::#route_variant(route) => {
                            let response = htmxology::SubcontrollerExt::get_subcontroller::<#controller_type>(self)
                                .handle_request(route, htmx.clone(), parts, server_info, args)
                                .await;
                            #conversion_logic
                        }
                    }
                }
            } else {
                // Has params - struct variant, construct Args from parent args + params
                let param_names = spec.params.iter().map(|p| &p.name);
                let param_names_for_construction = spec.params.iter().map(|p| &p.name);

                if convert_response_fn.is_some() {
                    // If using custom convert_response, clone values for convert_response before moving
                    quote_spanned! { spec.route_variant.span() =>
                        Self::Route::#route_variant { #(#param_names,)* subroute } => {
                            let parts_for_convert = parts.clone();
                            let args_for_convert = args.clone();
                            // Construct Args from parent args and path parameters
                            // User must implement From<(ParentArgs, param1, param2, ...)> for ChildArgs
                            let sub_args = <#controller_type as htmxology::Controller>::Args::from((args, #(#param_names_for_construction,)*));
                            let response = htmxology::SubcontrollerExt::get_subcontroller::<#controller_type>(self)
                                .handle_request(subroute, htmx.clone(), parts, server_info, sub_args)
                                .await;
                            #conversion_logic
                        }
                    }
                } else {
                    quote_spanned! { spec.route_variant.span() =>
                        Self::Route::#route_variant { #(#param_names,)* subroute } => {
                            // Construct Args from parent args and path parameters
                            // User must implement From<(ParentArgs, param1, param2, ...)> for ChildArgs
                            let sub_args = <#controller_type as htmxology::Controller>::Args::from((args, #(#param_names_for_construction,)*));
                            let response = htmxology::SubcontrollerExt::get_subcontroller::<#controller_type>(self)
                                .handle_request(subroute, htmx.clone(), parts, server_info, sub_args)
                                .await;
                            #conversion_logic
                        }
                    }
                }
            });
        } else if attr.path().is_ident(CONTROLLER) {
            if controller_spec.is_some() {
                return Err(syn::Error::new_spanned(
                    attr,
                    "only one `controller` attribute can be specified",
                ));
            }

            controller_spec = Some(attr.parse_args()?);
        }
    }

    let controller_spec = match controller_spec {
        Some(spec) => spec,
        None => {
            return Err(syn::Error::new_spanned(
                root_ident,
                "expected `controller` attribute",
            ));
        }
    };

    let route_ident = controller_spec.route_ident;
    let response_type = controller_spec.response_type.unwrap_or_else(
        || parse_quote!(Result<axum::response::Response, axum::response::Response>),
    );
    let args_type = controller_spec
        .args_type
        .unwrap_or_else(|| parse_quote!(()));
    let pre_handler = controller_spec.pre_handler;
    let extra_derives = controller_spec.extra_derives;

    // Build the derive attribute with default and extra derives
    let derive_attr = if extra_derives.is_empty() {
        quote! { #[derive(Debug, Clone, htmxology::Route)] }
    } else {
        quote! { #[derive(Debug, Clone, htmxology::Route, #(#extra_derives),*)] }
    };

    let route_decl = quote_spanned! { route_ident.span() =>
        #derive_attr
        pub enum #route_ident {
            #(#route_variants)*
        }
    };

    // Generate pre-handler call if configured
    let (pre_handler_call, args_mutability) = if let Some(pre_handler_fn) = &pre_handler {
        (
            quote! {
                // Call pre-handler and return early if it provides a response
                if let Some(response) = #pre_handler_fn(self, &route, &htmx, &parts, server_info, &mut args).await {
                    return response;
                }
            },
            quote! { mut },
        )
    } else {
        (quote! {}, quote! {})
    };

    let controller_impl = quote_spanned! { root_ident.span() =>
        impl htmxology::Controller for #root_ident {
            type Route = #route_ident;
            type Args = #args_type;
            type Response = #response_type;

            async fn handle_request(
                &self,
                route: Self::Route,
                htmx: htmxology::htmx::Request,
                parts: http::request::Parts,
                server_info: &htmxology::ServerInfo,
                #args_mutability args: Self::Args,
            ) -> Self::Response {
                #pre_handler_call

                match route {
                    #(#handle_request_variants)*
                }
            }
        }
    };

    Ok(quote! {
        #(#as_subcontroller_impls)*
        #route_decl
        #controller_impl
    })
}

struct SubcontrollerSpec {
    as_subcontroller_impl_fn: Box<dyn Fn(&Ident) -> proc_macro2::TokenStream>,
    controller_type: Type,
    route_variant: Ident,
    path: Option<String>,
    doc: Option<String>,
    params: Vec<ParamSpec>,
    convert_response_fn: Option<proc_macro2::TokenStream>,
}

/// A parameter specification for a subcontroller route.
#[derive(Clone)]
struct ParamSpec {
    name: Ident,
    ty: Type,
}

enum SubcontrollerArg {
    Route(Ident, Ident),
    Path(Ident, String),
    ConvertWith(proc_macro2::TokenStream),
    ConvertResponse(proc_macro2::TokenStream),
    Doc(Ident, String),
    Params(Vec<ParamSpec>),
}

impl Parse for SubcontrollerArg {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let key: Ident = input.parse()?;

        match key.to_string().as_str() {
            ROUTE => {
                input.parse::<Token![=]>()?;
                let ident: Ident = input.parse()?;

                Ok(Self::Route(key, ident))
            }
            PATH => {
                input.parse::<Token![=]>()?;
                let path: LitStr = input.parse()?;

                Ok(Self::Path(key, path.value()))
            }
            CONVERT_WITH => {
                input.parse::<Token![=]>()?;
                let fn_name: LitStr = input.parse()?;
                let fn_expr = fn_name.value().parse().map_err(|err| {
                    syn::Error::new_spanned(
                        fn_name,
                        format!("failed to parse function name: {err}"),
                    )
                })?;

                Ok(Self::ConvertWith(fn_expr))
            }
            CONVERT_RESPONSE => {
                input.parse::<Token![=]>()?;
                let fn_name: LitStr = input.parse()?;
                let fn_expr = fn_name.value().parse().map_err(|err| {
                    syn::Error::new_spanned(
                        fn_name,
                        format!("failed to parse function name: {err}"),
                    )
                })?;

                Ok(Self::ConvertResponse(fn_expr))
            }
            DOC => {
                input.parse::<Token![=]>()?;
                let desc: LitStr = input.parse()?;

                Ok(Self::Doc(key, desc.value()))
            }
            PARAMS => {
                // Parse params(name: Type, name2: Type2, ...)
                let content;
                syn::parenthesized!(content in input);

                let params: Punctuated<ParamSpec, Token![,]> =
                    content.parse_terminated(ParamSpec::parse, Token![,])?;

                Ok(Self::Params(params.into_iter().collect()))
            }
            _ => Err(syn::Error::new_spanned(
                key,
                "expected `route`, `path`, `convert_with`, `convert_response`, `doc`, or `params`",
            )),
        }
    }
}

impl Parse for ParamSpec {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let name: Ident = input.parse()?;
        input.parse::<Token![:]>()?;
        let ty: Type = input.parse()?;

        Ok(ParamSpec { name, ty })
    }
}

/// Controller specification parsed from #[controller(...)] attribute.
struct ControllerSpec {
    route_ident: Ident,
    response_type: Option<Type>,
    args_type: Option<Type>,
    pre_handler: Option<proc_macro2::TokenStream>,
    extra_derives: Vec<Ident>,
}

impl Parse for ControllerSpec {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        // First, parse the route identifier (required)
        let route_ident: Ident = input.parse()?;

        let mut response_type = None;
        let mut args_type = None;
        let mut pre_handler = None;
        let mut extra_derives = Vec::new();

        // Check if there's a comma followed by named arguments
        while input.peek(Token![,]) {
            input.parse::<Token![,]>()?;

            if input.is_empty() {
                break;
            }

            // Parse "key = value"
            let key: Ident = input.parse()?;
            input.parse::<Token![=]>()?;

            match key.to_string().as_str() {
                RESPONSE => {
                    if response_type.is_some() {
                        return Err(syn::Error::new_spanned(
                            &key,
                            "duplicate `response` parameter",
                        ));
                    }
                    response_type = Some(input.parse()?);
                }
                ARGS => {
                    if args_type.is_some() {
                        return Err(syn::Error::new_spanned(&key, "duplicate `args` parameter"));
                    }
                    args_type = Some(input.parse()?);
                }
                PRE_HANDLER => {
                    if pre_handler.is_some() {
                        return Err(syn::Error::new_spanned(
                            &key,
                            "duplicate `pre_handler` parameter",
                        ));
                    }
                    let fn_name: LitStr = input.parse()?;
                    pre_handler = Some(fn_name.value().parse().map_err(|err| {
                        syn::Error::new_spanned(
                            fn_name,
                            format!("failed to parse function name: {err}"),
                        )
                    })?);
                }
                EXTRA_DERIVES => {
                    if !extra_derives.is_empty() {
                        return Err(syn::Error::new_spanned(
                            &key,
                            "duplicate `extra_derives` parameter",
                        ));
                    }
                    // Parse parenthesized list: extra_derives = (PartialEq, Eq, Hash)
                    let content;
                    syn::parenthesized!(content in input);
                    let derives: Punctuated<Ident, Token![,]> =
                        content.parse_terminated(Ident::parse, Token![,])?;
                    extra_derives = derives.into_iter().collect();
                }
                _ => {
                    return Err(syn::Error::new_spanned(
                        &key,
                        format!(
                            "expected `{RESPONSE}`, `{ARGS}`, `{PRE_HANDLER}`, or `{EXTRA_DERIVES}`, found `{key}`"
                        ),
                    ));
                }
            }
        }

        Ok(ControllerSpec {
            route_ident,
            response_type,
            args_type,
            pre_handler,
            extra_derives,
        })
    }
}

impl Parse for SubcontrollerSpec {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        // Parse the subcontroller type, with its possible lifetime parameter.
        let controller_type: Type = input.parse()?;

        // Name the first lifetime parameter, if any.
        let lifetime: Lifetime = parse_quote!('_component_spec_lifetime);
        let (controller_type_with_spec_lifetime, has_lifetime) =
            replace_first_lifetime(&controller_type, lifetime.clone());

        // We force the controller type to have a 'static lifetime for the impl.
        let controller_type = replace_first_lifetime(&controller_type, parse_quote!('static)).0;

        let mut route = None;
        let mut path = None;
        let mut doc = None;
        let mut params = Vec::new();
        let mut body_impl = quote! { self.into() };
        let mut convert_response_fn = None;

        if input.peek(Token![,]) {
            input.parse::<Token![,]>()?;

            let args = Punctuated::<SubcontrollerArg, Token![,]>::parse_terminated(input)?;

            for arg in args {
                match arg {
                    SubcontrollerArg::ConvertWith(fn_expr) => {
                        body_impl = quote! { #fn_expr(self) };
                    }
                    SubcontrollerArg::ConvertResponse(fn_expr) => {
                        convert_response_fn = Some(fn_expr);
                    }
                    SubcontrollerArg::Route(key, ident) => {
                        if route.is_some() {
                            return Err(syn::Error::new_spanned(
                                key,
                                "only one `route` can be specified",
                            ));
                        }

                        route = Some(ident)
                    }
                    SubcontrollerArg::Path(key, rpath) => {
                        if path.is_some() {
                            return Err(syn::Error::new_spanned(
                                key,
                                "at most one path` can be specified",
                            ));
                        }

                        path = Some(rpath);
                    }
                    SubcontrollerArg::Doc(key, desc) => {
                        if doc.is_some() {
                            return Err(syn::Error::new_spanned(
                                key,
                                "at most one `doc` can be specified",
                            ));
                        }

                        doc = Some(desc);
                    }
                    SubcontrollerArg::Params(param_specs) => {
                        if !params.is_empty() {
                            return Err(syn::Error::new_spanned(
                                &param_specs[0].name,
                                "only one `params` can be specified",
                            ));
                        }

                        params = param_specs;
                    }
                }
            }
        };

        let route = match route {
            Some(r) => r,
            None => {
                return Err(syn::Error::new_spanned(
                    controller_type,
                    "expected a `route` argument",
                ));
            }
        };

        // Build the conversion body for as_subcontroller
        // No params anymore, so just use body_impl
        let conversion_body = body_impl;

        let as_subcontroller_impl_fn: Box<dyn Fn(&Ident) -> proc_macro2::TokenStream> = {
            if has_lifetime {
                Box::new(move |root_ident: &Ident| {
                    quote! {
                        impl<#lifetime> htmxology::HasSubcontroller<#lifetime, #controller_type_with_spec_lifetime> for #root_ident {
                            fn as_subcontroller(&#lifetime self) -> #controller_type_with_spec_lifetime {
                                #conversion_body
                            }
                        }
                    }
                })
            } else {
                Box::new(move |root_ident: &Ident| {
                    quote! {
                        impl htmxology::HasSubcontroller<'_, #controller_type_with_spec_lifetime> for #root_ident {
                            fn as_subcontroller(&self) -> #controller_type_with_spec_lifetime {
                                #conversion_body
                            }
                        }
                    }
                })
            }
        };

        Ok(SubcontrollerSpec {
            as_subcontroller_impl_fn,
            controller_type,
            route_variant: route,
            path,
            doc,
            params,
            convert_response_fn,
        })
    }
}

fn replace_first_lifetime(ty: &Type, new_lifetime: Lifetime) -> (Type, bool) {
    let mut ty = ty.clone();
    let replaced = replace_first_lifetime_mut(&mut ty, new_lifetime);

    (ty, replaced)
}

fn replace_first_lifetime_mut(ty: &mut Type, new_lifetime: Lifetime) -> bool {
    match ty {
        // Handle reference types like &'a T
        Type::Reference(TypeReference { lifetime, .. }) => {
            *lifetime = Some(new_lifetime);
            true
        }

        // Handle path types like Foo<'a, T>
        Type::Path(TypePath { path, .. }) => {
            for segment in &mut path.segments {
                if let syn::PathArguments::AngleBracketed(args) = &mut segment.arguments {
                    for arg in &mut args.args {
                        match arg {
                            GenericArgument::Lifetime(lt) => {
                                *lt = new_lifetime;
                                return true;
                            }
                            GenericArgument::Type(ty) => {
                                if replace_first_lifetime_mut(ty, new_lifetime.clone()) {
                                    return true;
                                }
                            }
                            _ => {}
                        };
                    }
                }
            }
            false
        }

        // Handle other type variants if needed
        _ => false,
    }
}

fn remove_lifetimes(ty: &Type) -> Type {
    let mut ty = ty.clone();
    remove_lifetimes_mut(&mut ty);
    ty
}

fn remove_lifetimes_mut(ty: &mut Type) {
    match ty {
        Type::Reference(TypeReference { lifetime, .. }) => {
            *lifetime = None;
        }
        Type::Path(TypePath { path, .. }) => {
            for segment in &mut path.segments {
                if let syn::PathArguments::AngleBracketed(args) = &mut segment.arguments {
                    for arg in &mut args.args {
                        match arg {
                            GenericArgument::Lifetime(lifetime) => {
                                *lifetime = parse_quote!('_);
                            }
                            GenericArgument::Type(ty) => {
                                remove_lifetimes_mut(ty);
                            }
                            _ => {}
                        };
                    }
                }
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_replace_first_lifetime_reference() {
        let ty: Type = syn::parse_str("&'a str").unwrap();
        let new_lifetime: Lifetime = syn::parse_str("'b").unwrap();
        let (new_ty, replaced) = replace_first_lifetime(&ty, new_lifetime);
        assert_eq!(quote! { #new_ty }.to_string(), "& 'b str");
        assert!(replaced);
    }

    #[test]
    fn test_replace_first_lifetime_generic() {
        let ty: Type = syn::parse_str("Foo<'a>").unwrap();
        let new_lifetime: Lifetime = syn::parse_str("'b").unwrap();
        let (new_ty, replaced) = replace_first_lifetime(&ty, new_lifetime);
        assert_eq!(quote! { #new_ty }.to_string(), "Foo < 'b >");
        assert!(replaced);
    }

    #[test]
    fn test_replace_first_lifetime_nested_generic() {
        let ty: Type = syn::parse_str("Option<&'a str>").unwrap();
        let new_lifetime: Lifetime = syn::parse_str("'b").unwrap();
        let (new_ty, replaced) = replace_first_lifetime(&ty, new_lifetime);
        assert_eq!(quote! { #new_ty }.to_string(), "Option < & 'b str >");
        assert!(replaced);
    }

    #[test]
    fn test_replace_first_lifetime_no_lifetime() {
        let ty: Type = syn::parse_str("i32").unwrap();
        let new_lifetime: Lifetime = syn::parse_str("'b").unwrap();
        let (new_ty, replaced) = replace_first_lifetime(&ty, new_lifetime);
        assert_eq!(quote! { #new_ty }.to_string(), "i32");
        assert!(!replaced);
    }

    #[test]
    fn test_remove_lifetimes_from_reference() {
        let ty: Type = syn::parse_str("&'a str").unwrap();
        let new_ty = remove_lifetimes(&ty);
        assert_eq!(quote! { #new_ty }.to_string(), "& str");
    }

    #[test]
    fn test_remove_lifetimes_from_generic() {
        let ty: Type = syn::parse_str("Foo<'a>").unwrap();
        let new_ty = remove_lifetimes(&ty);
        assert_eq!(quote! { #new_ty }.to_string(), "Foo < '_ >");
    }

    #[test]
    fn test_remove_lifetimes_from_nested_generic() {
        let ty: Type = syn::parse_str("Option<&'a str>").unwrap();
        let new_ty = remove_lifetimes(&ty);
        assert_eq!(quote! { #new_ty }.to_string(), "Option < & str >");
    }

    #[test]
    fn test_remove_lifetimes_no_lifetime() {
        let ty: Type = syn::parse_str("i32").unwrap();
        let new_ty = remove_lifetimes(&ty);
        assert_eq!(quote! { #new_ty }.to_string(), "i32");
    }
}

#[cfg(test)]
mod snapshot_tests {
    use super::*;
    use crate::utils::testing::test_derive;
    use insta::assert_snapshot;

    fn test_routing_controller(input: &str) -> String {
        test_derive(input, derive)
    }

    #[test]
    fn single_subcontroller_with_path() {
        let input = r#"
            #[controller(AppRoute)]
            #[subcontroller(BlogController, route = Blog, path = "blog/")]
            struct AppController {
                blog: BlogController,
            }
        "#;
        assert_snapshot!(test_routing_controller(input));
    }

    #[test]
    fn single_subcontroller_catch_all() {
        let input = r#"
            #[controller(AppRoute)]
            #[subcontroller(NotFoundController, route = NotFound)]
            struct AppController {
                not_found: NotFoundController,
            }
        "#;
        assert_snapshot!(test_routing_controller(input));
    }

    #[test]
    fn multiple_subcontrollers() {
        let input = r#"
            #[controller(AppRoute)]
            #[subcontroller(HomeController, route = Home, path = "")]
            #[subcontroller(BlogController, route = Blog, path = "blog/")]
            #[subcontroller(ShopController, route = Shop, path = "shop/")]
            struct AppController {
                home: HomeController,
                blog: BlogController,
                shop: ShopController,
            }
        "#;
        assert_snapshot!(test_routing_controller(input));
    }

    #[test]
    fn subcontroller_with_lifetime() {
        let input = r#"
            #[controller(AppRoute)]
            #[subcontroller(&'a DataController, route = Data, path = "data/")]
            struct AppController<'a> {
                data: &'a DataController,
            }
        "#;
        assert_snapshot!(test_routing_controller(input));
    }

    #[test]
    fn subcontroller_with_convert() {
        let input = r#"
            #[controller(AppRoute)]
            #[subcontroller(BlogController, route = Blog, path = "blog/", convert_with = "Self::get_blog")]
            struct AppController {
                state: AppState,
            }
        "#;
        assert_snapshot!(test_routing_controller(input));
    }

    #[test]
    fn subcontroller_with_doc() {
        let input = r#"
            #[controller(AppRoute)]
            #[subcontroller(BlogController, route = Blog, path = "blog/", doc = "Blog section")]
            struct AppController {
                blog: BlogController,
            }
        "#;
        assert_snapshot!(test_routing_controller(input));
    }

    #[test]
    fn complex_app() {
        let input = r#"
            #[controller(AppRoute)]
            #[subcontroller(HomeController, route = Home, path = "", doc = "Home page")]
            #[subcontroller(AuthController, route = Auth, path = "auth/", doc = "Authentication")]
            #[subcontroller(ApiController, route = Api, path = "api/", doc = "API endpoints")]
            #[subcontroller(NotFoundController, route = NotFound, doc = "404 handler")]
            struct AppController {
                home: HomeController,
                auth: AuthController,
                api: ApiController,
                not_found: NotFoundController,
            }
        "#;
        assert_snapshot!(test_routing_controller(input));
    }

    // Note: Removed parameterized subcontroller tests since params() is no longer supported.
    // Path parameters should be handled at the Route level, not for subcontroller construction.

    #[test]
    fn custom_response_type() {
        let input = r#"
            #[controller(AppRoute, response = Result<MyResponse, MyError>)]
            #[subcontroller(BlogController, route = Blog, path = "blog/")]
            struct AppController {
                blog: BlogController,
            }
        "#;
        assert_snapshot!(test_routing_controller(input));
    }

    #[test]
    fn default_response_type() {
        let input = r#"
            #[controller(AppRoute)]
            #[subcontroller(HomeController, route = Home, path = "")]
            struct AppController {
                home: HomeController,
            }
        "#;
        assert_snapshot!(test_routing_controller(input));
    }

    #[test]
    fn with_pre_handler() {
        let input = r#"
            #[controller(AppRoute, args = Session, pre_handler = "Self::authenticate")]
            #[subcontroller(DashboardController, route = Dashboard, path = "dashboard/")]
            #[subcontroller(AdminController, route = Admin, path = "admin/")]
            struct AppController {
                dashboard: DashboardController,
                admin: AdminController,
            }
        "#;
        assert_snapshot!(test_routing_controller(input));
    }

    #[test]
    fn without_pre_handler() {
        let input = r#"
            #[controller(AppRoute, args = Session)]
            #[subcontroller(DashboardController, route = Dashboard, path = "dashboard/")]
            struct AppController {
                dashboard: DashboardController,
            }
        "#;
        assert_snapshot!(test_routing_controller(input));
    }

    #[test]
    fn with_extra_derives() {
        let input = r#"
            #[controller(AppRoute, extra_derives = (PartialEq, Eq, Hash))]
            #[subcontroller(HomeController, route = Home, path = "")]
            #[subcontroller(BlogController, route = Blog, path = "blog/")]
            struct AppController {
                home: HomeController,
                blog: BlogController,
            }
        "#;
        assert_snapshot!(test_routing_controller(input));
    }

    #[test]
    fn with_single_extra_derive() {
        let input = r#"
            #[controller(AppRoute, extra_derives = (PartialEq))]
            #[subcontroller(HomeController, route = Home, path = "")]
            struct AppController {
                home: HomeController,
            }
        "#;
        assert_snapshot!(test_routing_controller(input));
    }

    #[test]
    fn with_extra_derives_and_other_options() {
        let input = r#"
            #[controller(AppRoute, args = Session, response = Result<MyResponse, MyError>, extra_derives = (PartialEq, Eq))]
            #[subcontroller(DashboardController, route = Dashboard, path = "dashboard/")]
            struct AppController {
                dashboard: DashboardController,
            }
        "#;
        assert_snapshot!(test_routing_controller(input));
    }

    #[test]
    fn multiple_default_subcontrollers_error() {
        let input = r#"
            #[controller(AppRoute)]
            #[subcontroller(FallbackA, route = FallbackA)]
            #[subcontroller(FallbackB, route = FallbackB)]
            struct AppController {
                fallback_a: FallbackA,
                fallback_b: FallbackB,
            }
        "#;
        let mut parsed: syn::DeriveInput = syn::parse_str(input).expect("Failed to parse input");
        let result = derive(&mut parsed);
        assert!(
            result.is_err(),
            "Expected error for multiple default subcontrollers"
        );
        let error_message = result.unwrap_err().to_string();
        assert!(
            error_message.contains("only one default subcontroller"),
            "Expected error about multiple default subcontrollers, got: {}",
            error_message
        );
    }
}
