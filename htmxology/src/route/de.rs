//! Implement single path argument deserialization.

use serde::{
    de::{self, DeserializeSeed, EnumAccess, VariantAccess, Visitor},
    forward_to_deserialize_any, Deserializer,
};
use std::{any::type_name, borrow::Cow};

/// An error that occurred during path argument deserialization.
#[derive(Debug, thiserror::Error)]
pub(crate) enum PathArgumentDeserializationError {
    /// Failed to percent-decode the value.
    #[error("failed to percent-decode value: {0}")]
    PercentDecodeError(#[from] std::str::Utf8Error),

    /// Percent-decoding is required for the value.
    #[error("cannot parse without allocation as percent-decoding is required for value: {value}")]
    PercentDecodingRequired { value: String },

    /// The type is not supported.
    #[error("unsupported type: {0}")]
    UnsupportedType(&'static str),

    /// The type is not expected.
    #[error("unexpected type: {0}")]
    UnexpectedType(&'static str),

    /// The argument could not be parsed.
    #[error("failed to parse value `{value}` as {expected_type}")]
    ParseError {
        /// The value that could not be parsed.
        value: String,

        /// The expected type.
        expected_type: &'static str,
    },

    /// An unknown error occurred.
    #[error("an unknown error occurred: {msg}")]
    Unknown { msg: String },
}

impl serde::de::Error for PathArgumentDeserializationError {
    fn custom<T: std::fmt::Display>(msg: T) -> Self {
        PathArgumentDeserializationError::Unknown {
            msg: msg.to_string(),
        }
    }
}

macro_rules! impl_unsupported_type {
    ($trait_fn:ident) => {
        fn $trait_fn<V>(self, _: V) -> Result<V::Value, Self::Error>
        where
            V: Visitor<'de>,
        {
            Err(PathArgumentDeserializationError::UnsupportedType(
                type_name::<V::Value>(),
            ))
        }
    };
}

macro_rules! impl_simple {
    ($trait_fn:ident, $visit_fn:ident, $ty:literal) => {
        fn $trait_fn<V>(self, visitor: V) -> Result<V::Value, Self::Error>
        where
            V: Visitor<'de>,
        {
            let value = percent_encoding::percent_decode(self.url_encoded_value.as_bytes())
                .decode_utf8()?;

            let value =
                value
                    .parse()
                    .map_err(|_| PathArgumentDeserializationError::ParseError {
                        value: value.to_string(),
                        expected_type: $ty,
                    })?;

            visitor.$visit_fn(value)
        }
    };
}

pub(crate) struct PathArgumentDeserializer<'de> {
    url_encoded_value: &'de str,
}

impl<'de> PathArgumentDeserializer<'de> {
    #[inline]
    pub(crate) fn new(url_encoded_value: &'de str) -> Self {
        PathArgumentDeserializer { url_encoded_value }
    }
}

impl<'de> Deserializer<'de> for PathArgumentDeserializer<'de> {
    type Error = PathArgumentDeserializationError;

    impl_unsupported_type!(deserialize_bytes);
    impl_unsupported_type!(deserialize_option);
    impl_unsupported_type!(deserialize_identifier);
    impl_unsupported_type!(deserialize_ignored_any);
    impl_unsupported_type!(deserialize_seq);
    impl_unsupported_type!(deserialize_map);

    impl_simple!(deserialize_bool, visit_bool, "bool");
    impl_simple!(deserialize_i8, visit_i8, "i8");
    impl_simple!(deserialize_i16, visit_i16, "i16");
    impl_simple!(deserialize_i32, visit_i32, "i32");
    impl_simple!(deserialize_i64, visit_i64, "i64");
    impl_simple!(deserialize_i128, visit_i128, "i128");
    impl_simple!(deserialize_u8, visit_u8, "u8");
    impl_simple!(deserialize_u16, visit_u16, "u16");
    impl_simple!(deserialize_u32, visit_u32, "u32");
    impl_simple!(deserialize_u64, visit_u64, "u64");
    impl_simple!(deserialize_u128, visit_u128, "u128");
    impl_simple!(deserialize_f32, visit_f32, "f32");
    impl_simple!(deserialize_f64, visit_f64, "f64");
    impl_simple!(deserialize_string, visit_string, "String");
    impl_simple!(deserialize_byte_buf, visit_string, "String");
    impl_simple!(deserialize_char, visit_char, "char");

    fn deserialize_any<V>(self, v: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_string(v)
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match percent_encoding::percent_decode(self.url_encoded_value.as_bytes()).decode_utf8()? {
            Cow::Borrowed(value) => visitor.visit_borrowed_str(value),
            Cow::Owned(value) => {
                Err(PathArgumentDeserializationError::PercentDecodingRequired { value })
            }
        }
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_unit()
    }

    fn deserialize_unit_struct<V>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_unit()
    }

    fn deserialize_newtype_struct<V>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_tuple<V>(self, _len: usize, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Err(PathArgumentDeserializationError::UnsupportedType(
            type_name::<V::Value>(),
        ))
    }

    fn deserialize_tuple_struct<V>(
        self,
        _name: &'static str,
        _len: usize,
        _visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Err(PathArgumentDeserializationError::UnsupportedType(
            type_name::<V::Value>(),
        ))
    }

    fn deserialize_struct<V>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        _visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Err(PathArgumentDeserializationError::UnsupportedType(
            type_name::<V::Value>(),
        ))
    }

    fn deserialize_enum<V>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_enum(EnumDeserializer {
            value: self.url_encoded_value,
        })
    }
}

struct KeyDeserializer<'de> {
    key: &'de str,
}

macro_rules! parse_key {
    ($trait_fn:ident) => {
        fn $trait_fn<V>(self, visitor: V) -> Result<V::Value, Self::Error>
        where
            V: Visitor<'de>,
        {
            match percent_encoding::percent_decode(self.key.as_bytes()).decode_utf8()? {
                Cow::Borrowed(value) => visitor.visit_borrowed_str(value),
                Cow::Owned(value) => visitor.visit_string(value),
            }
        }
    };
}

impl<'de> Deserializer<'de> for KeyDeserializer<'de> {
    type Error = PathArgumentDeserializationError;

    parse_key!(deserialize_identifier);
    parse_key!(deserialize_str);
    parse_key!(deserialize_string);

    fn deserialize_any<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Err(PathArgumentDeserializationError::UnexpectedType(
            type_name::<V::Value>(),
        ))
    }

    forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char bytes
        byte_buf option unit unit_struct seq tuple
        tuple_struct map newtype_struct struct enum ignored_any
    }
}

struct EnumDeserializer<'de> {
    value: &'de str,
}

impl<'de> EnumAccess<'de> for EnumDeserializer<'de> {
    type Error = PathArgumentDeserializationError;
    type Variant = UnitVariant;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant), Self::Error>
    where
        V: de::DeserializeSeed<'de>,
    {
        Ok((
            seed.deserialize(KeyDeserializer { key: self.value })?,
            UnitVariant,
        ))
    }
}

struct UnitVariant;

impl<'de> VariantAccess<'de> for UnitVariant {
    type Error = PathArgumentDeserializationError;

    fn unit_variant(self) -> Result<(), Self::Error> {
        Ok(())
    }

    fn newtype_variant_seed<T>(self, _seed: T) -> Result<T::Value, Self::Error>
    where
        T: DeserializeSeed<'de>,
    {
        Err(PathArgumentDeserializationError::UnsupportedType(
            "newtype enum variant",
        ))
    }

    fn tuple_variant<V>(self, _len: usize, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Err(PathArgumentDeserializationError::UnsupportedType(
            "tuple enum variant",
        ))
    }

    fn struct_variant<V>(
        self,
        _fields: &'static [&'static str],
        _visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Err(PathArgumentDeserializationError::UnsupportedType(
            "struct enum variant",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[derive(Debug, Deserialize, Eq, PartialEq)]
    enum MyEnum {
        Apple,
        Banana,
        Carrot,
    }

    macro_rules! assert_parse_eq {
        ($ty:ty, $value_str:literal, $value:expr) => {{
            let deserializer = PathArgumentDeserializer::new($value_str);
            assert_eq!(<$ty>::deserialize(deserializer).unwrap(), $value);
        }};
    }

    macro_rules! assert_parse_error {
        ($ty:ty, $value_str:literal, $err:pat $(if $guard:expr)?) => {{
            let deserializer = PathArgumentDeserializer::new($value_str);
            let err = <$ty>::deserialize(deserializer).expect_err("expected error");

            match err {
                $err $(if $guard)? => {}
                err => panic!("unexpected error: {err}"),
            }
        }};
    }

    #[test]
    fn test_parse_single_value() {
        assert_parse_eq!(i8, "-42", -42);
        assert_parse_eq!(i16, "-42", -42);
        assert_parse_eq!(i32, "-42", -42);
        assert_parse_eq!(i64, "-42", -42);
        assert_parse_eq!(i128, "42", 42);
        assert_parse_eq!(u8, "42", 42);
        assert_parse_eq!(u16, "42", 42);
        assert_parse_eq!(u32, "42", 42);
        assert_parse_eq!(u64, "42", 42);
        assert_parse_eq!(u128, "42", 42);
        assert_parse_eq!(f32, "42", 42.0);
        assert_parse_eq!(f64, "42", 42.0);
        assert_parse_eq!(bool, "true", true);
        assert_parse_eq!(bool, "false", false);
        assert_parse_eq!(String, "foo", "foo");
        assert_parse_eq!(String, "alpha%20beta", "alpha beta");
        assert_parse_eq!(&str, "foo", "foo");
        assert_parse_eq!(char, "X", 'X');
        assert_parse_eq!(MyEnum, "Apple", MyEnum::Apple);
        assert_parse_eq!(MyEnum, "Banana", MyEnum::Banana);
        assert_parse_eq!(MyEnum, "Carrot", MyEnum::Carrot);

        assert_parse_error!(
            &str,
            "alpha%beta",
            PathArgumentDeserializationError::PercentDecodeError(_)
        );
        assert_parse_error!(
            &str,
            "alpha%20beta",
            PathArgumentDeserializationError::PercentDecodingRequired { value } if value == "alpha beta"
        );
        assert_parse_error!(
            u8,
            "300",
            PathArgumentDeserializationError::ParseError{value, expected_type} if value == "300" && expected_type == "u8"
        );
        assert_parse_error!(
            u16,
            "foo",
            PathArgumentDeserializationError::ParseError{value, expected_type} if value == "foo" && expected_type == "u16"
        );
    }
}
