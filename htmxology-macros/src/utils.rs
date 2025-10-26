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
