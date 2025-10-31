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
pub(super) const ARGS_FACTORY: &str = "args_factory";

pub fn derive(input: &mut syn::DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    // Get the name of the root type.
    let root_ident = &input.ident;

    let mut as_subcontroller_impls = Vec::new();
    let mut route_variants = Vec::new();
    let mut handle_request_variants = Vec::new();

    let mut controller_spec: Option<ControllerSpec> = None;

    // Let's iterate over the top-level `subcontroller` attributes.
    for attr in &input.attrs {
        if attr.path().is_ident(SUBCONTROLLER) {
            let spec: SubcontrollerSpec = attr.parse_args()?;

            as_subcontroller_impls.push((spec.as_subcontroller_impl_fn)(root_ident));

            let route_variant = &spec.route_variant;
            let controller_type = &spec.controller_type;

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

            // Generate handle_request match arm
            handle_request_variants.push(if spec.params.is_empty() {
                // No params - simple tuple variant, pass parent args through
                quote_spanned! { spec.route_variant.span() =>
                    Self::Route::#route_variant(route) => {
                        let response = htmxology::SubcontrollerExt::get_subcontroller::<#controller_type>(self)
                            .handle_request(route, htmx.clone(), parts, server_info, args)
                            .await;
                        <Self as htmxology::HasSubcontroller<'_, #controller_type>>::convert_response(&htmx, response)
                    }
                }
            } else {
                // Has params - struct variant, construct Args from parent args + params
                let param_names = spec.params.iter().map(|p| &p.name);
                let param_names_for_construction = spec.params.iter().map(|p| &p.name);

                quote_spanned! { spec.route_variant.span() =>
                    Self::Route::#route_variant { #(#param_names,)* subroute } => {
                        // Construct Args from parent args and path parameters
                        // User must implement From<(ParentArgs, param1, param2, ...)> for ChildArgs
                        let sub_args = <#controller_type as htmxology::Controller>::Args::from((args, #(#param_names_for_construction,)*));
                        let response = htmxology::SubcontrollerExt::get_subcontroller::<#controller_type>(self)
                            .handle_request(subroute, htmx.clone(), parts, server_info, sub_args)
                            .await;
                        <Self as htmxology::HasSubcontroller<'_, #controller_type>>::convert_response(&htmx, response)
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
    let args_factory = controller_spec.args_factory;

    let route_decl = quote_spanned! { route_ident.span() =>
        #[derive(Debug, Clone, htmxology::Route)]
        pub enum #route_ident {
            #(#route_variants)*
        }
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
                args: Self::Args,
            ) -> Self::Response {
                match route {
                    #(#handle_request_variants)*
                }
            }
        }
    };

    // Generate From<Controller> for ControllerRouter implementation
    // Always pass a factory - either the custom one or default
    let factory_fn = if let Some(factory) = &args_factory {
        factory.clone()
    } else {
        quote! { |_| async { <#args_type>::default() } }
    };

    let controller_router_impl = quote_spanned! { root_ident.span() =>
        impl From<#root_ident> for htmxology::ControllerRouter {
            fn from(controller: #root_ident) -> Self {
                htmxology::ControllerRouter::new(controller, #factory_fn)
            }
        }
    };

    Ok(quote! {
        #(#as_subcontroller_impls)*
        #route_decl
        #controller_impl
        #controller_router_impl
    })
}

struct SubcontrollerSpec {
    as_subcontroller_impl_fn: Box<dyn Fn(&Ident) -> proc_macro2::TokenStream>,
    controller_type: Type,
    route_variant: Ident,
    path: Option<String>,
    doc: Option<String>,
    params: Vec<ParamSpec>,
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
    args_factory: Option<proc_macro2::TokenStream>,
}

impl Parse for ControllerSpec {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        // First, parse the route identifier (required)
        let route_ident: Ident = input.parse()?;

        let mut response_type = None;
        let mut args_type = None;
        let mut args_factory = None;

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
                ARGS_FACTORY => {
                    if args_factory.is_some() {
                        return Err(syn::Error::new_spanned(
                            &key,
                            "duplicate `args_factory` parameter",
                        ));
                    }
                    let fn_name: LitStr = input.parse()?;
                    args_factory = Some(fn_name.value().parse().map_err(|err| {
                        syn::Error::new_spanned(
                            fn_name,
                            format!("failed to parse function name: {err}"),
                        )
                    })?);
                }
                _ => {
                    return Err(syn::Error::new_spanned(
                        &key,
                        format!(
                            "expected `{RESPONSE}`, `{ARGS}`, or `{ARGS_FACTORY}`, found `{key}`"
                        ),
                    ));
                }
            }
        }

        Ok(ControllerSpec {
            route_ident,
            response_type,
            args_type,
            args_factory,
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

        // Generate convert_response body based on whether a custom function was specified
        let convert_response_body = if let Some(ref fn_expr) = convert_response_fn {
            // Custom function specified
            quote! {
                #fn_expr(htmx, response)
            }
        } else {
            // Default: assume ParentResponse: From<SubControllerResponse> and use Into
            quote! {
                response.into()
            }
        };

        let as_subcontroller_impl_fn: Box<dyn Fn(&Ident) -> proc_macro2::TokenStream> = {
            let convert_response_body_clone = convert_response_body.clone();
            if has_lifetime {
                Box::new(move |root_ident: &Ident| {
                    quote! {
                        impl<#lifetime> htmxology::HasSubcontroller<#lifetime, #controller_type_with_spec_lifetime> for #root_ident {
                            fn as_subcontroller(&#lifetime self) -> #controller_type_with_spec_lifetime {
                                #conversion_body
                            }

                            fn convert_response(
                                htmx: &htmxology::htmx::Request,
                                response: <#controller_type_with_spec_lifetime as htmxology::Controller>::Response
                            ) -> <Self as htmxology::Controller>::Response {
                                #convert_response_body_clone
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

                            fn convert_response(
                                htmx: &htmxology::htmx::Request,
                                response: <#controller_type_with_spec_lifetime as htmxology::Controller>::Response
                            ) -> <Self as htmxology::Controller>::Response {
                                #convert_response_body
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
}
