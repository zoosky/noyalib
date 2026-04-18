//! Source location tracking for deserialized values.
//!
//! `Spanned<T>` wraps a deserialized value and records the start/end
//! [`Location`](crate::Location) in the original YAML source.

use std::fmt;
use std::ops::Deref;

use serde::{Deserialize, Serialize};

use crate::error::Location;

/// Sentinel struct name used by the deserializer to detect `Spanned<T>`.
pub(crate) const SPANNED_TYPE_NAME: &str = "$__noyalib_private_Spanned";

/// Field names for the virtual Spanned struct.
pub(crate) const SPANNED_FIELD_START_LINE: &str = "$__noyalib_start_line";
pub(crate) const SPANNED_FIELD_START_COLUMN: &str = "$__noyalib_start_column";
pub(crate) const SPANNED_FIELD_START_INDEX: &str = "$__noyalib_start_index";
pub(crate) const SPANNED_FIELD_END_LINE: &str = "$__noyalib_end_line";
pub(crate) const SPANNED_FIELD_END_COLUMN: &str = "$__noyalib_end_column";
pub(crate) const SPANNED_FIELD_END_INDEX: &str = "$__noyalib_end_index";
pub(crate) const SPANNED_FIELD_VALUE: &str = "$__noyalib_value";

pub(crate) const SPANNED_FIELDS: &[&str] = &[
    SPANNED_FIELD_START_LINE,
    SPANNED_FIELD_START_COLUMN,
    SPANNED_FIELD_START_INDEX,
    SPANNED_FIELD_END_LINE,
    SPANNED_FIELD_END_COLUMN,
    SPANNED_FIELD_END_INDEX,
    SPANNED_FIELD_VALUE,
];

/// A value annotated with source span information.
///
/// During serialization, `Spanned<T>` serializes transparently as `T`.
/// During deserialization from a YAML string via [`from_str`](crate::from_str)
/// or [`from_str_with_config`](crate::from_str_with_config), real source
/// locations are populated. When deserializing via
/// [`from_value`](crate::from_value), locations default to zero.
///
/// # Example
///
/// ```rust
/// use noyalib::Spanned;
/// use serde::{Deserialize, Serialize};
///
/// #[derive(Serialize, Deserialize, Debug)]
/// struct Config {
///     port: Spanned<u16>,
/// }
/// ```
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Spanned<T> {
    /// The deserialized value.
    pub value: T,
    /// Start location in the source.
    pub start: Location,
    /// End location in the source.
    pub end: Location,
}

impl<T: fmt::Debug> fmt::Debug for Spanned<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Spanned")
            .field("value", &self.value)
            .field("start", &self.start)
            .field("end", &self.end)
            .finish()
    }
}

impl<T> Deref for Spanned<T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.value
    }
}

impl<T> Spanned<T> {
    /// Create a new spanned value with default (zero) locations.
    pub fn new(value: T) -> Self {
        Self {
            value,
            start: Location::default(),
            end: Location::default(),
        }
    }

    /// Unwrap into the inner value.
    pub fn into_inner(self) -> T {
        self.value
    }
}

impl<T> From<T> for Spanned<T> {
    fn from(value: T) -> Self {
        Self::new(value)
    }
}

impl<T: Serialize> Serialize for Spanned<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        // Transparent: just serialize the inner value
        self.value.serialize(serializer)
    }
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for Spanned<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_struct(
            SPANNED_TYPE_NAME,
            SPANNED_FIELDS,
            SpannedVisitor(std::marker::PhantomData),
        )
    }
}

struct SpannedVisitor<T>(std::marker::PhantomData<T>);

impl<'de, T: Deserialize<'de>> serde::de::Visitor<'de> for SpannedVisitor<T> {
    type Value = Spanned<T>;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a spanned value")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::MapAccess<'de>,
    {
        let mut start_line: Option<usize> = None;
        let mut start_column: Option<usize> = None;
        let mut start_index: Option<usize> = None;
        let mut end_line: Option<usize> = None;
        let mut end_column: Option<usize> = None;
        let mut end_index: Option<usize> = None;
        let mut value: Option<T> = None;

        while let Some(key) = map.next_key::<&str>()? {
            match key {
                SPANNED_FIELD_START_LINE => start_line = Some(map.next_value()?),
                SPANNED_FIELD_START_COLUMN => start_column = Some(map.next_value()?),
                SPANNED_FIELD_START_INDEX => start_index = Some(map.next_value()?),
                SPANNED_FIELD_END_LINE => end_line = Some(map.next_value()?),
                SPANNED_FIELD_END_COLUMN => end_column = Some(map.next_value()?),
                SPANNED_FIELD_END_INDEX => end_index = Some(map.next_value()?),
                SPANNED_FIELD_VALUE => value = Some(map.next_value()?),
                _ => {
                    // Unknown field — skip
                    let _ = map.next_value::<serde::de::IgnoredAny>()?;
                }
            }
        }

        let value = value.ok_or_else(|| serde::de::Error::missing_field(SPANNED_FIELD_VALUE))?;

        let start = Location::new(
            start_line.unwrap_or(0),
            start_column.unwrap_or(0),
            start_index.unwrap_or(0),
        );
        let end = Location::new(
            end_line.unwrap_or(0),
            end_column.unwrap_or(0),
            end_index.unwrap_or(0),
        );

        Ok(Spanned { value, start, end })
    }
}
