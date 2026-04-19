//! Smart pointer anchor types for shared/DAG structures.
//!
//! These wrappers provide anchor semantics for `Rc` and `Arc` pointers,
//! allowing YAML serialization of shared data structures.

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

use std::fmt;
use std::ops::Deref;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

/// An `Rc` wrapper with YAML anchor semantics.
///
/// Serializes by delegating to the inner `T`. Deserializes by wrapping the
/// result in `Rc`.
#[derive(Clone)]
pub struct RcAnchor<T>(pub std::rc::Rc<T>);

impl<T: fmt::Debug> fmt::Debug for RcAnchor<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("RcAnchor").field(&self.0).finish()
    }
}

impl<T> Deref for RcAnchor<T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.0
    }
}

impl<T> From<T> for RcAnchor<T> {
    fn from(v: T) -> Self {
        Self(std::rc::Rc::new(v))
    }
}

impl<T> From<std::rc::Rc<T>> for RcAnchor<T> {
    fn from(v: std::rc::Rc<T>) -> Self {
        Self(v)
    }
}

impl<T> RcAnchor<T> {
    /// Unwrap into the inner `Rc`.
    pub fn into_inner(self) -> std::rc::Rc<T> {
        self.0
    }
}

impl<T: Serialize> Serialize for RcAnchor<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.0.serialize(serializer)
    }
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for RcAnchor<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        T::deserialize(deserializer).map(|v| RcAnchor(std::rc::Rc::new(v)))
    }
}

/// An `Arc` wrapper with YAML anchor semantics.
///
/// Serializes by delegating to the inner `T`. Deserializes by wrapping the
/// result in `Arc`.
#[derive(Clone)]
pub struct ArcAnchor<T>(pub Arc<T>);

impl<T: fmt::Debug> fmt::Debug for ArcAnchor<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("ArcAnchor").field(&self.0).finish()
    }
}

impl<T> Deref for ArcAnchor<T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.0
    }
}

impl<T> From<T> for ArcAnchor<T> {
    fn from(v: T) -> Self {
        Self(Arc::new(v))
    }
}

impl<T> From<Arc<T>> for ArcAnchor<T> {
    fn from(v: Arc<T>) -> Self {
        Self(v)
    }
}

impl<T> ArcAnchor<T> {
    /// Unwrap into the inner `Arc`.
    pub fn into_inner(self) -> Arc<T> {
        self.0
    }
}

impl<T: Serialize> Serialize for ArcAnchor<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.0.serialize(serializer)
    }
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for ArcAnchor<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        T::deserialize(deserializer).map(|v| ArcAnchor(Arc::new(v)))
    }
}

/// A weak `Rc` reference with YAML anchor semantics.
///
/// Serializes as `null` if the reference is dangling, otherwise serializes
/// the inner value. Deserialization from `null` produces a dangling weak ref.
#[derive(Clone)]
pub struct RcWeakAnchor<T>(pub std::rc::Weak<T>);

impl<T: fmt::Debug> fmt::Debug for RcWeakAnchor<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0.upgrade() {
            Some(v) => f.debug_tuple("RcWeakAnchor").field(&v).finish(),
            None => f.debug_tuple("RcWeakAnchor").field(&"(dangling)").finish(),
        }
    }
}

impl<T> RcWeakAnchor<T> {
    /// Create a dangling weak anchor.
    pub fn dangling() -> Self {
        Self(std::rc::Weak::new())
    }

    /// Unwrap into the inner `Weak`.
    pub fn into_inner(self) -> std::rc::Weak<T> {
        self.0
    }

    /// Attempt to upgrade to a strong `Rc`.
    pub fn upgrade(&self) -> Option<std::rc::Rc<T>> {
        self.0.upgrade()
    }
}

impl<T> From<std::rc::Weak<T>> for RcWeakAnchor<T> {
    fn from(v: std::rc::Weak<T>) -> Self {
        Self(v)
    }
}

impl<T: Serialize> Serialize for RcWeakAnchor<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self.0.upgrade() {
            Some(v) => v.serialize(serializer),
            None => serializer.serialize_none(),
        }
    }
}

impl<'de, T> Deserialize<'de> for RcWeakAnchor<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        // Always deserialize as a dangling weak — there's no registry to look up.
        // We consume the value to avoid errors.
        let _ = serde::de::IgnoredAny::deserialize(deserializer)?;
        Ok(RcWeakAnchor(std::rc::Weak::new()))
    }
}

/// A weak `Arc` reference with YAML anchor semantics.
///
/// Serializes as `null` if the reference is dangling, otherwise serializes
/// the inner value. Deserialization from `null` produces a dangling weak ref.
#[derive(Clone)]
pub struct ArcWeakAnchor<T>(pub std::sync::Weak<T>);

impl<T: fmt::Debug> fmt::Debug for ArcWeakAnchor<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0.upgrade() {
            Some(v) => f.debug_tuple("ArcWeakAnchor").field(&v).finish(),
            None => f.debug_tuple("ArcWeakAnchor").field(&"(dangling)").finish(),
        }
    }
}

impl<T> ArcWeakAnchor<T> {
    /// Create a dangling weak anchor.
    pub fn dangling() -> Self {
        Self(std::sync::Weak::new())
    }

    /// Unwrap into the inner `Weak`.
    pub fn into_inner(self) -> std::sync::Weak<T> {
        self.0
    }

    /// Attempt to upgrade to a strong `Arc`.
    pub fn upgrade(&self) -> Option<Arc<T>> {
        self.0.upgrade()
    }
}

impl<T> From<std::sync::Weak<T>> for ArcWeakAnchor<T> {
    fn from(v: std::sync::Weak<T>) -> Self {
        Self(v)
    }
}

impl<T: Serialize> Serialize for ArcWeakAnchor<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self.0.upgrade() {
            Some(v) => v.serialize(serializer),
            None => serializer.serialize_none(),
        }
    }
}

impl<'de, T> Deserialize<'de> for ArcWeakAnchor<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let _ = serde::de::IgnoredAny::deserialize(deserializer)?;
        Ok(ArcWeakAnchor(std::sync::Weak::new()))
    }
}
