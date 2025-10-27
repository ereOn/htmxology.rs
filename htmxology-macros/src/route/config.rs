//! Configuration structures for route parsing.
//!
//! This module contains intermediate data structures that represent the parsed
//! configuration of route variants. These structures separate parsing logic from
//! code generation, making the codebase easier to understand and maintain.

use super::route_type::RouteType;
use super::route_url::RouteUrl;
use super::{attributes, parse_route_info};
use quote::format_ident;
use syn::{Error, Field, Fields, Ident, Type, Variant};

/// Configuration for a single route variant.
///
/// This structure contains all the information needed to generate code for a route variant,
/// parsed from the enum variant's attributes and fields.
#[derive(Clone)]
pub struct VariantConfig {
    /// The identifier of the variant (e.g., `Home` in `enum Route { Home }`).
    pub ident: Ident,

    /// The parsed route URL (e.g., `users/{user_id}/posts`).
    pub route_url: RouteUrl,

    /// The type of route (Simple, SubRoute, or CatchAll).
    pub route_type: RouteType,

    /// The fields configuration for this variant.
    pub fields: FieldsConfig,
}

/// Configuration for the fields of a variant.
///
/// Represents the three possible field configurations in Rust enums:
/// - Unit: No fields (e.g., `Home`)
/// - Named: Named fields (e.g., `User { id: u32 }`)
/// - Unnamed: Tuple fields (e.g., `User(u32)`)
#[derive(Clone)]
pub enum FieldsConfig {
    /// No fields.
    Unit,

    /// Named fields (struct-like variant).
    Named(Vec<FieldConfig>),

    /// Unnamed fields (tuple-like variant).
    Unnamed(Vec<FieldConfig>),
}

impl FieldsConfig {
    /// Creates a `FieldsConfig` from syn Fields.
    pub fn from_fields(fields: &Fields, route_type: &RouteType) -> syn::Result<Self> {
        match fields {
            Fields::Unit => Ok(Self::Unit),
            Fields::Named(named_fields) => {
                if named_fields.named.is_empty() {
                    Ok(Self::Named(Vec::new()))
                } else {
                    let configs = named_fields
                        .named
                        .iter()
                        .map(|field| FieldConfig::from_named_field(field, route_type))
                        .collect::<syn::Result<Vec<_>>>()?;
                    Self::validate_fields(&configs)?;
                    Ok(Self::Named(configs))
                }
            }
            Fields::Unnamed(unnamed_fields) => {
                if unnamed_fields.unnamed.is_empty() {
                    Ok(Self::Unnamed(Vec::new()))
                } else {
                    let configs = unnamed_fields
                        .unnamed
                        .iter()
                        .enumerate()
                        .map(|(i, field)| FieldConfig::from_unnamed_field(field, i, route_type))
                        .collect::<syn::Result<Vec<_>>>()?;
                    Self::validate_fields(&configs)?;
                    Ok(Self::Unnamed(configs))
                }
            }
        }
    }

    /// Validates field configurations to ensure consistency.
    fn validate_fields(fields: &[FieldConfig]) -> syn::Result<()> {
        let mut query_count = 0;
        let mut body_count = 0;
        let mut subroute_count = 0;

        for field in fields {
            match field.role {
                FieldRole::Query => {
                    query_count += 1;
                    if query_count > 1 {
                        return Err(Error::new_spanned(
                            &field.ident,
                            "only one field can be a query parameter",
                        ));
                    }
                }
                FieldRole::Body => {
                    body_count += 1;
                    if body_count > 1 {
                        return Err(Error::new_spanned(
                            &field.ident,
                            "only one field can be a body parameter",
                        ));
                    }
                }
                FieldRole::Subroute => {
                    subroute_count += 1;
                    if subroute_count > 1 {
                        return Err(Error::new_spanned(
                            &field.ident,
                            "only one field can be a subroute",
                        ));
                    }
                }
                FieldRole::PathParam { .. } => {}
                FieldRole::CatchAll => {
                    // CatchAll fields are always valid, no validation needed
                }
            }
        }

        Ok(())
    }

    /// Returns an iterator over the fields.
    pub fn iter(&self) -> impl Iterator<Item = &FieldConfig> {
        match self {
            Self::Unit => [].iter(),
            Self::Named(fields) | Self::Unnamed(fields) => fields.iter(),
        }
    }

    /// Returns true if this is a named variant.
    pub fn is_named(&self) -> bool {
        matches!(self, Self::Named(_))
    }
}

/// Configuration for a single field in a variant.
///
/// Each field has an identifier (which may be generated for unnamed fields),
/// a type, and a role that determines how it's used in routing.
#[derive(Clone)]
pub struct FieldConfig {
    /// The identifier for this field.
    /// For named fields, this is the field name.
    /// For unnamed fields, this is generated (e.g., `arg0`, `arg1`).
    pub ident: Ident,

    /// The type of the field.
    pub ty: Type,

    /// The role of this field in routing.
    pub role: FieldRole,
}

impl FieldConfig {
    /// Creates a `FieldConfig` from a named field.
    pub fn from_named_field(field: &Field, route_type: &RouteType) -> syn::Result<Self> {
        let ident = field.ident.clone().expect("named field should have ident");
        let ty = field.ty.clone();
        let role = Self::determine_role(field, Some(ident.to_string()), route_type)?;

        Ok(Self { ident, ty, role })
    }

    /// Creates a `FieldConfig` from an unnamed field.
    pub fn from_unnamed_field(
        field: &Field,
        index: usize,
        route_type: &RouteType,
    ) -> syn::Result<Self> {
        let ident = format_ident!("arg{}", index);
        let ty = field.ty.clone();
        let role = Self::determine_role(field, None, route_type)?;

        Ok(Self { ident, ty, role })
    }

    /// Determines the role of a field based on its attributes.
    fn determine_role(
        field: &Field,
        param_name: Option<String>,
        route_type: &RouteType,
    ) -> syn::Result<FieldRole> {
        let is_query = field
            .attrs
            .iter()
            .any(|attr| attr.path().is_ident(attributes::QUERY));

        let is_body = field
            .attrs
            .iter()
            .any(|attr| attr.path().is_ident(attributes::BODY));

        let is_subroute = field
            .attrs
            .iter()
            .any(|attr| attr.path().is_ident(attributes::SUBROUTE));

        // Validate that only one attribute is present
        let attr_count = [is_query, is_body, is_subroute]
            .iter()
            .filter(|&&b| b)
            .count();

        if attr_count > 1 {
            return Err(Error::new_spanned(
                field,
                "field cannot have multiple role attributes (query, body, subroute)",
            ));
        }

        // Determine the role
        if is_query {
            Ok(FieldRole::Query)
        } else if is_body {
            Ok(FieldRole::Body)
        } else if is_subroute {
            // Validate that subroute is only used with SubRoute route type
            if !matches!(route_type, RouteType::SubRoute) {
                return Err(Error::new_spanned(
                    field,
                    "subroute attribute can only be used on variants with trailing slash in route",
                ));
            }
            Ok(FieldRole::Subroute)
        } else if matches!(route_type, RouteType::CatchAll) {
            // For catch-all variants, the field is the catch-all route
            Ok(FieldRole::CatchAll)
        } else {
            // Default to path parameter
            Ok(FieldRole::PathParam { name: param_name })
        }
    }

    /// Returns true if this field is a path parameter.
    pub fn is_path_param(&self) -> bool {
        matches!(self.role, FieldRole::PathParam { .. })
    }

    /// Returns true if this field is a query parameter.
    pub fn is_query(&self) -> bool {
        matches!(self.role, FieldRole::Query)
    }

    /// Returns true if this field is a body parameter.
    pub fn is_body(&self) -> bool {
        matches!(self.role, FieldRole::Body)
    }

    /// Returns true if this field is a subroute.
    pub fn is_subroute(&self) -> bool {
        matches!(self.role, FieldRole::Subroute)
    }
}

/// The role of a field in routing.
///
/// Each field in a route variant serves a specific purpose:
/// - Path parameters are extracted from the URL path
/// - Query parameters are extracted from the query string
/// - Body parameters are extracted from the request body
/// - Subroutes delegate to another route type
/// - CatchAll handles any unmatched routes
#[derive(Debug, Clone)]
pub enum FieldRole {
    /// A path parameter extracted from the URL.
    ///
    /// For named fields, the `name` matches the field identifier and the URL parameter name.
    /// For unnamed fields, the `name` is None and the parameter is positional.
    PathParam {
        /// The name of the parameter in the URL (e.g., `user_id` in `/users/{user_id}`).
        /// None for positional parameters in unnamed variants.
        name: Option<String>,
    },

    /// A query parameter extracted from the query string (annotated with `#[query]`).
    Query,

    /// A body parameter extracted from the request body (annotated with `#[body]`).
    Body,

    /// A subroute that delegates to another route type (annotated with `#[subroute]`).
    Subroute,

    /// A catch-all field that handles unmatched routes (used in `#[catch_all]` variants).
    CatchAll,
}

impl VariantConfig {
    /// Creates a new `VariantConfig` from a syn Variant.
    ///
    /// This is the main entry point for parsing a variant. It extracts all the
    /// necessary information from the variant's attributes and fields.
    pub fn from_variant(variant: &Variant) -> syn::Result<Self> {
        let ident = variant.ident.clone();
        let (route_url, route_type) = parse_route_info(variant)?;
        let fields = FieldsConfig::from_fields(&variant.fields, &route_type)?;

        Ok(Self {
            ident,
            route_url,
            route_type,
            fields,
        })
    }

    /// Returns the query parameter field, if any.
    pub fn query_param(&self) -> Option<&FieldConfig> {
        self.fields.iter().find(|f| f.is_query())
    }

    /// Returns the body parameter field, if any.
    pub fn body_param(&self) -> Option<&FieldConfig> {
        self.fields.iter().find(|f| f.is_body())
    }

    /// Returns the subroute field, if any.
    pub fn subroute_param(&self) -> Option<&FieldConfig> {
        self.fields.iter().find(|f| f.is_subroute())
    }
}
