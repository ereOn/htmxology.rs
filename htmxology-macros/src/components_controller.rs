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
pub(super) const COMPONENT: &str = "component";
pub(super) const ROUTE: &str = "route";
pub(super) const PATH: &str = "path";
pub(super) const DOC: &str = "doc";
pub(super) const CONVERT_WITH: &str = "convert_with";

pub fn derive(input: &mut syn::DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    // Get the name of the root type.
    let root_ident = &input.ident;

    let mut as_component_impls = Vec::new();
    let mut route_variants = Vec::new();
    let mut handle_request_variants = Vec::new();

    let mut route_ident: Option<Ident> = None;

    // Let's iterate over the top-level `component` attributes.
    for attr in &input.attrs {
        if attr.path().is_ident(COMPONENT) {
            let spec: ComponentSpec = attr.parse_args()?;

            as_component_impls.push((spec.as_component_impl_fn)(root_ident));

            let route_variant = &spec.route_variant;
            let controller_type = &spec.controller_type;

            let doc_attr = if let Some(doc) = &spec.doc {
                quote_spanned! { spec.controller_type.span() =>
                    #[doc = #doc]
                }
            } else {
                quote! {}
            };

            route_variants.push(match spec.path {
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
            });

            let controller_type = remove_lifetimes(controller_type);

            handle_request_variants.push(quote_spanned! { spec.route_variant.span() =>
                Self::Route::#route_variant(route) => {
                    self.get_component::<#controller_type>()
                        .handle_request(route, htmx, parts, server_info)
                        .await
                }
            });
        } else if attr.path().is_ident(CONTROLLER) {
            if route_ident.is_some() {
                return Err(syn::Error::new_spanned(
                    attr,
                    "only one `controller` attribute can be specified",
                ));
            }

            route_ident = Some(attr.parse_args()?);
        }
    }

    let route_ident = match route_ident {
        Some(ident) => ident,
        None => {
            return Err(syn::Error::new_spanned(
                root_ident,
                "expected `controller` attribute",
            ));
        }
    };

    let route_decl = quote_spanned! { route_ident.span() =>
        #[derive(Debug, Clone, htmxology::Route)]
        pub enum #route_ident {
            #(#route_variants)*
        }
    };

    let controller_impl = quote_spanned! { root_ident.span() =>
        impl htmxology::Controller for #root_ident {
            type Route = #route_ident;

            async fn handle_request(
                &self,
                route: Self::Route,
                htmx: htmxology::htmx::Request,
                parts: http::request::Parts,
                server_info: &htmxology::ServerInfo,
            ) -> Result<axum::response::Response, axum::response::Response> {
                match route {
                    #(#handle_request_variants)*
                }
            }
        }
    };

    Ok(quote! {
        #(#as_component_impls)*
        #route_decl
        #controller_impl
    })
}

struct ComponentSpec {
    as_component_impl_fn: Box<dyn Fn(&Ident) -> proc_macro2::TokenStream>,
    controller_type: Type,
    route_variant: Ident,
    path: Option<String>,
    doc: Option<String>,
}

#[derive(Debug)]
enum ComponentArg {
    Route(Ident, Ident),
    Path(Ident, String),
    ConvertWith(proc_macro2::TokenStream),
    Doc(Ident, String),
}

impl Parse for ComponentArg {
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
            DOC => {
                input.parse::<Token![=]>()?;
                let desc: LitStr = input.parse()?;

                Ok(Self::Doc(key, desc.value()))
            }
            _ => Err(syn::Error::new_spanned(
                key,
                "expected `catch_all`, `route`, or `convert_with`",
            )),
        }
    }
}

impl Parse for ComponentSpec {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        // Parse the component type, with its possible lifetime parameter.
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
        let mut body_impl = quote! { self.into() };

        if input.peek(Token![,]) {
            input.parse::<Token![,]>()?;

            let args = Punctuated::<ComponentArg, Token![,]>::parse_terminated(input)?;

            for arg in args {
                match arg {
                    ComponentArg::ConvertWith(fn_expr) => {
                        body_impl = quote! { #fn_expr(self) };
                    }
                    ComponentArg::Route(key, ident) => {
                        if route.is_some() {
                            return Err(syn::Error::new_spanned(
                                key,
                                "only one `route` can be specified",
                            ));
                        }

                        route = Some(ident)
                    }
                    ComponentArg::Path(key, rpath) => {
                        if path.is_some() {
                            return Err(syn::Error::new_spanned(
                                key,
                                "at most one path` can be specified",
                            ));
                        }

                        path = Some(rpath);
                    }
                    ComponentArg::Doc(key, desc) => {
                        if doc.is_some() {
                            return Err(syn::Error::new_spanned(
                                key,
                                "at most one `doc` can be specified",
                            ));
                        }

                        doc = Some(desc);
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

        let as_component_impl_fn: Box<dyn Fn(&Ident) -> proc_macro2::TokenStream> = {
            if has_lifetime {
                Box::new(move |root_ident: &Ident| {
                    quote! {
                        impl<#lifetime> htmxology::AsComponent<#lifetime, #controller_type_with_spec_lifetime> for #root_ident {
                            fn as_component_controller(&#lifetime self) -> #controller_type_with_spec_lifetime {
                                #body_impl
                            }
                        }
                    }
                })
            } else {
                Box::new(move |root_ident: &Ident| {
                    quote! {
                        impl htmxology::AsComponent<'_, #controller_type_with_spec_lifetime> for #root_ident {
                            fn as_component_controller(&self) -> #controller_type_with_spec_lifetime {
                                #body_impl
                            }
                        }
                    }
                })
            }
        };

        Ok(ComponentSpec {
            as_component_impl_fn,
            controller_type,
            route_variant: route,
            path,
            doc,
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

    fn test_components_controller(input: &str) -> String {
        test_derive(input, derive)
    }

    #[test]
    fn single_component_with_path() {
        let input = r#"
            #[controller(AppRoute)]
            #[component(BlogController, route = Blog, path = "blog/")]
            struct AppController {
                blog: BlogController,
            }
        "#;
        assert_snapshot!(test_components_controller(input));
    }

    #[test]
    fn single_component_catch_all() {
        let input = r#"
            #[controller(AppRoute)]
            #[component(NotFoundController, route = NotFound)]
            struct AppController {
                not_found: NotFoundController,
            }
        "#;
        assert_snapshot!(test_components_controller(input));
    }

    #[test]
    fn multiple_components() {
        let input = r#"
            #[controller(AppRoute)]
            #[component(HomeController, route = Home, path = "")]
            #[component(BlogController, route = Blog, path = "blog/")]
            #[component(ShopController, route = Shop, path = "shop/")]
            struct AppController {
                home: HomeController,
                blog: BlogController,
                shop: ShopController,
            }
        "#;
        assert_snapshot!(test_components_controller(input));
    }

    #[test]
    fn component_with_lifetime() {
        let input = r#"
            #[controller(AppRoute)]
            #[component(&'a DataController, route = Data, path = "data/")]
            struct AppController<'a> {
                data: &'a DataController,
            }
        "#;
        assert_snapshot!(test_components_controller(input));
    }

    #[test]
    fn component_with_convert() {
        let input = r#"
            #[controller(AppRoute)]
            #[component(BlogController, route = Blog, path = "blog/", convert_with = "Self::get_blog")]
            struct AppController {
                state: AppState,
            }
        "#;
        assert_snapshot!(test_components_controller(input));
    }

    #[test]
    fn component_with_doc() {
        let input = r#"
            #[controller(AppRoute)]
            #[component(BlogController, route = Blog, path = "blog/", doc = "Blog section")]
            struct AppController {
                blog: BlogController,
            }
        "#;
        assert_snapshot!(test_components_controller(input));
    }

    #[test]
    fn complex_app() {
        let input = r#"
            #[controller(AppRoute)]
            #[component(HomeController, route = Home, path = "", doc = "Home page")]
            #[component(AuthController, route = Auth, path = "auth/", doc = "Authentication")]
            #[component(ApiController, route = Api, path = "api/", doc = "API endpoints")]
            #[component(NotFoundController, route = NotFound, doc = "404 handler")]
            struct AppController {
                home: HomeController,
                auth: AuthController,
                api: ApiController,
                not_found: NotFoundController,
            }
        "#;
        assert_snapshot!(test_components_controller(input));
    }
}
