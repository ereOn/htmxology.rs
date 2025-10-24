//! Route derive macro.

use quote::quote;
use syn::{
    GenericArgument, Ident, Lifetime, Token, Type, TypePath, TypeReference,
    parse::{Parse, ParseStream},
    parse_quote,
};

pub(super) const COMPONENT: &str = "component";
pub(super) const CONVERT_WITH: &str = "convert_with";

pub(super) fn derive(input: &mut syn::DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    // Get the name of the root type.
    let root_ident = &input.ident;

    let mut as_component_impls = Vec::new();

    // Let's iterate over the top-level `component` attributes.
    for attr in input
        .attrs
        .iter()
        .filter(|attr| attr.path().is_ident(COMPONENT))
    {
        // `component` attribute found.
        //
        // It is supposed to be one of:
        // - `#[component(MyComponent)]`
        // - `#[component(MyComponent, convert_with = "fn_name")]`

        let spec: ComponentSpec = attr.parse_args()?;

        as_component_impls.push((spec.as_component_impl_fn)(root_ident));
    }

    Ok(quote! {
        #(#as_component_impls)*
    })
}

struct ComponentSpec {
    as_component_impl_fn: Box<dyn Fn(&Ident) -> proc_macro2::TokenStream>,
}

impl Parse for ComponentSpec {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        // Parse the component type, with its possible lifetime parameter.
        let component_type: Type = input.parse()?;

        let lifetime: Lifetime = parse_quote!('_component_spec_lifetime);
        let (component_type, has_lifetime) =
            replace_first_lifetime(&component_type, lifetime.clone());

        let as_component_impl_fn: Box<dyn Fn(&Ident) -> proc_macro2::TokenStream> = {
            let body_impl = if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;

                // Parse `convert_with = "fn_name"`
                let convert_with_ident: Ident = input.parse()?;
                if convert_with_ident != CONVERT_WITH {
                    return Err(syn::Error::new_spanned(
                        convert_with_ident,
                        format!("expected `{CONVERT_WITH}`"),
                    ));
                }

                input.parse::<Token![=]>()?;
                // The next token is the function name, as a string literal.
                let convert_with_fn: syn::LitStr = input.parse()?;

                // Parse the function name from the string literal.
                let input = convert_with_fn.value();
                let convert_with_fn: proc_macro2::TokenStream = input.parse().map_err(|err| {
                    syn::Error::new_spanned(
                        convert_with_fn,
                        format!("failed to parse function name: {err}"),
                    )
                })?;

                quote! { #convert_with_fn(self) }
            } else {
                quote! { self.into() }
            };

            if has_lifetime {
                Box::new(move |root_ident: &Ident| {
                    quote! {
                        impl<#lifetime> htmxology::AsComponent<#lifetime, #component_type> for #root_ident {
                            fn as_component_controller(&#lifetime self) -> #component_type {
                                #body_impl
                            }
                        }
                    }
                })
            } else {
                Box::new(move |root_ident: &Ident| {
                    quote! {
                        impl htmxology::AsComponent<'_, #component_type> for #root_ident {
                            fn as_component_controller(&self) -> #component_type {
                                #body_impl
                            }
                        }
                    }
                })
            }
        };

        Ok(ComponentSpec {
            as_component_impl_fn,
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
            *lifetime = Some(new_lifetime.clone());
            true
        }

        // Handle path types like Foo<'a, T>
        Type::Path(TypePath { path, .. }) => {
            for segment in &mut path.segments {
                if let syn::PathArguments::AngleBracketed(args) = &mut segment.arguments {
                    for arg in &mut args.args {
                        if let GenericArgument::Lifetime(lt) = arg {
                            *lt = new_lifetime;
                            return true;
                        }
                    }
                }
            }
            false
        }

        // Handle other type variants if needed
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_replace_first_lifetime() {
        let ty: Type = syn::parse_str("&'a str").unwrap();
        let new_lifetime: Lifetime = syn::parse_str("'b").unwrap();
        let (new_ty, replaced) = replace_first_lifetime(&ty, new_lifetime);
        assert_eq!(quote! { #new_ty }.to_string(), "& 'b str");
        assert!(replaced);
    }

    #[test]
    fn test_replace_first_lifetime_in_path() {
        let ty: Type = syn::parse_str("Option<&'a str>").unwrap();
        let new_lifetime: Lifetime = syn::parse_str("'b").unwrap();
        let (new_ty, replaced) = replace_first_lifetime(&ty, new_lifetime);
        assert_eq!(
            quote! { #new_ty }.to_string(),
            "Option < & '
b str >"
        );
        assert!(replaced);
    }
}
