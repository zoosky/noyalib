//! Declarative validation via [`garde`] (requires the `garde` feature).
//!
//! Provides [`Validated<T>`], a transparent wrapper that runs `T::validate`
//! after deserialisation. Combine with `#[derive(garde::Validate)]` and
//! `#[garde(...)]` field attributes to layer schema-level constraints on
//! YAML input without hand-written post-deserialise code.
//!
//! # Example
//!
//! ```rust
//! use noyalib::Validated;
//! use garde::Validate;
//! use serde::Deserialize;
//!
//! #[derive(Deserialize, Validate)]
//! struct Server {
//!     #[garde(length(min = 1, max = 255))]
//!     host: String,
//!     #[garde(range(min = 1024, max = 65535))]
//!     port: u16,
//! }
//!
//! let yaml = "host: db.local\nport: 5432\n";
//! let wrapped: Validated<Server> = noyalib::from_str(yaml).unwrap();
//! assert_eq!(wrapped.0.host, "db.local");
//! ```
//!
//! Invalid input produces a descriptive [`crate::Error`]:
//!
//! ```rust
//! use noyalib::Validated;
//! use garde::Validate;
//! use serde::Deserialize;
//!
//! #[derive(Debug, Deserialize, Validate)]
//! struct Server {
//!     #[garde(range(min = 1024, max = 65535))]
//!     port: u16,
//! }
//!
//! let yaml = "port: 80\n";
//! let err = noyalib::from_str::<Validated<Server>>(yaml).unwrap_err();
//! assert!(err.to_string().contains("port"));
//! ```
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

use crate::prelude::*;
use core::ops::{Deref, DerefMut};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// A newtype that validates its inner value on deserialisation.
///
/// `T` must implement both [`serde::Deserialize`] and
/// [`garde::Validate`] with a unit context. After deserialisation,
/// `validate(&())` is called and any validation errors are converted
/// into a deserialiser error using the reported field path and message.
///
/// The wrapper implements [`Deref`] / [`DerefMut`] so callers can access
/// the inner value ergonomically.
///
/// # Example
///
/// ```rust
/// # use noyalib::Validated;
/// # use garde::Validate;
/// # use serde::Deserialize;
/// #[derive(Deserialize, Validate)]
/// struct Config {
///     #[garde(length(min = 1))]
///     name: String,
/// }
///
/// let cfg: Validated<Config> = noyalib::from_str("name: app\n").unwrap();
/// assert_eq!(cfg.name, "app"); // Deref to &Config
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Validated<T>(pub T);

impl<T> Validated<T> {
    /// Consume the wrapper and return the inner value.
    #[inline]
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T> Deref for Validated<T> {
    type Target = T;
    #[inline]
    fn deref(&self) -> &T {
        &self.0
    }
}

impl<T> DerefMut for Validated<T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut T {
        &mut self.0
    }
}

impl<T> From<T> for Validated<T> {
    #[inline]
    fn from(v: T) -> Self {
        Validated(v)
    }
}

impl<'de, T> Deserialize<'de> for Validated<T>
where
    T: Deserialize<'de> + garde::Validate<Context = ()>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::Error;
        let inner = T::deserialize(deserializer)?;
        match inner.validate_with(&()) {
            Ok(()) => Ok(Validated(inner)),
            Err(report) => Err(D::Error::custom(format_report(&report))),
        }
    }
}

impl<T: Serialize> Serialize for Validated<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // Serialise transparently — validation is a deserialise-time concern.
        self.0.serialize(serializer)
    }
}

/// Format a garde report as a compact single-line message suitable for use
/// as a serde deserialiser error. Each field error is rendered as
/// `field.path: message`, joined by `; `.
fn format_report(report: &garde::Report) -> String {
    use core::fmt::Write as _;
    let mut out = String::with_capacity(64);
    out.push_str("validation failed: ");
    let mut first = true;
    for (path, error) in report.iter() {
        if !first {
            out.push_str("; ");
        }
        first = false;
        // garde::Path Display gives a dotted path; Error carries the message.
        let _ = write!(out, "{path}: {error}");
    }
    if first {
        // Empty report (shouldn't happen in practice); still return something.
        out.push_str("<no details>");
    }
    out
}
