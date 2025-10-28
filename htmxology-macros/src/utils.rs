//! Shared utilities for derive macros.

use syn::{Data, Error};

/// Extracts the DataEnum from a DeriveInput, returning an error if it's not an enum.
///
/// This eliminates the boilerplate of checking for structs and unions that appears
/// in all derive macros.
///
/// # Example
///
/// ```ignore
/// pub fn derive(input: &mut syn::DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
///     let data = expect_enum(input, "Route")?;
///     // ... work with data
/// }
/// ```
pub fn expect_enum<'a>(
    input: &'a syn::DeriveInput,
    derive_name: &str,
) -> syn::Result<&'a syn::DataEnum> {
    match &input.data {
        Data::Struct(_) => Err(Error::new_spanned(
            &input.ident,
            format!("can't derive {derive_name} for a struct"),
        )),
        Data::Enum(data_enum) => Ok(data_enum),
        Data::Union(_) => Err(Error::new_spanned(
            &input.ident,
            format!("can't derive {derive_name} for a union"),
        )),
    }
}

/// Validate an HTML identifier (ID or name) at compile time.
///
/// Returns an error if the identifier is invalid according to HTML5 rules:
/// - Must not be empty
/// - Must start with a letter, digit, or underscore
/// - May contain letters, digits, hyphens, underscores, colons, and periods
///
/// # Arguments
///
/// * `identifier` - The string to validate
/// * `span` - The span for error reporting
/// * `kind` - The kind of identifier ("ID" or "name") for error messages
pub fn validate_html_identifier(
    identifier: &str,
    span: proc_macro2::Span,
    kind: &str,
) -> syn::Result<()> {
    if identifier.is_empty() {
        return Err(syn::Error::new(
            span,
            format!("HTML {kind} cannot be empty"),
        ));
    }

    let mut chars = identifier.chars();
    let first_char = chars.next().unwrap();

    if !first_char.is_ascii_alphanumeric() && first_char != '_' {
        return Err(syn::Error::new(
            span,
            format!(
                "HTML {kind} must start with a letter, digit, or underscore, found '{first_char}'"
            ),
        ));
    }

    for c in chars {
        if !(c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == ':' || c == '.') {
            return Err(syn::Error::new(
                span,
                format!(
                    "HTML {kind} contains invalid character '{c}'. Only letters, digits, hyphens, underscores, colons, and periods are allowed"
                ),
            ));
        }
    }

    Ok(())
}

/// Parse a `with_fn = "Full::path::to::function"` attribute and extract the function path.
///
/// This handles the common pattern of parsing a MetaNameValue where:
/// - The path must be "with_fn"
/// - The value must be a string literal containing a valid Rust path
///
/// Returns a Path that can be used in generated code (e.g., "Foo::method" or "Self::method").
pub fn parse_with_fn_attribute_as_path(meta: &syn::MetaNameValue) -> syn::Result<syn::Path> {
    if !meta.path.is_ident("with_fn") {
        return Err(syn::Error::new_spanned(
            &meta.path,
            "expected 'with_fn' attribute",
        ));
    }

    let fn_path_str = match &meta.value {
        syn::Expr::Lit(expr_lit) => match &expr_lit.lit {
            syn::Lit::Str(lit_str) => lit_str,
            _ => {
                return Err(syn::Error::new_spanned(
                    &meta.value,
                    "with_fn must be a string literal",
                ));
            }
        },
        _ => {
            return Err(syn::Error::new_spanned(
                &meta.value,
                "with_fn must be a string literal",
            ));
        }
    };

    // Parse the string as a Path (e.g., "Foo::method" or "Self::method")
    fn_path_str.parse::<syn::Path>().map_err(|_| {
        syn::Error::new_spanned(
            fn_path_str,
            "with_fn value must be a valid Rust path (e.g., 'Foo::method' or 'Self::method')",
        )
    })
}

/// Extracts only the identifiers from generic parameters (without bounds or defaults).
///
/// This is needed because `syn::Generics::params` includes bounds and default values,
/// which are only valid in impl signatures, not in type instantiations.
///
/// # Example
///
/// ```ignore
/// // For: <T: Display = Bar, U>
/// // Returns tokens for: T, U
/// ```
pub fn extract_generic_param_idents(
    params: &syn::punctuated::Punctuated<syn::GenericParam, syn::token::Comma>,
) -> proc_macro2::TokenStream {
    use quote::quote;

    let idents = params.iter().map(|param| match param {
        syn::GenericParam::Type(type_param) => {
            let ident = &type_param.ident;
            quote! { #ident }
        }
        syn::GenericParam::Lifetime(lifetime_param) => {
            let lifetime = &lifetime_param.lifetime;
            quote! { #lifetime }
        }
        syn::GenericParam::Const(const_param) => {
            let ident = &const_param.ident;
            quote! { #ident }
        }
    });

    quote! { #(#idents),* }
}

#[cfg(test)]
pub mod testing {
    //! Test utilities for snapshot testing derive macros.

    use quote::quote;

    /// Helper function for snapshot testing derive macros.
    ///
    /// This wraps the generated code in a test module and formats it with prettyplease,
    /// which is the standard pattern used across all derive macro tests.
    ///
    /// # Example
    ///
    /// ```ignore
    /// #[test]
    /// fn my_test() {
    ///     let input = r#"
    ///         enum MyEnum {
    ///             Variant,
    ///         }
    ///     "#;
    ///     let output = test_derive(input, my_derive_function);
    ///     assert_snapshot!(output);
    /// }
    /// ```
    pub fn test_derive<F>(input: &str, derive_fn: F) -> String
    where
        F: FnOnce(&mut syn::DeriveInput) -> syn::Result<proc_macro2::TokenStream>,
    {
        let mut input: syn::DeriveInput = syn::parse_str(input).expect("Failed to parse input");
        let output = derive_fn(&mut input).expect("Derive failed");

        // Wrap in a module to make it valid Rust
        let wrapped = quote! {
            #[allow(unused)]
            mod __test {
                #output
            }
        };

        let syntax_tree: syn::File = syn::parse2(wrapped).expect("Failed to parse output");
        prettyplease::unparse(&syntax_tree)
    }
}
