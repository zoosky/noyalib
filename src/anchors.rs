//! Smart pointer anchor types for shared/DAG structures.
//!
//! These wrappers provide anchor semantics for `Rc` and `Arc` pointers,
//! allowing YAML serialization of shared data structures.

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

use crate::prelude::*;
use core::ops::Deref;

#[cfg(not(feature = "std"))]
use alloc::rc::{Rc, Weak as RcWeak};
#[cfg(not(feature = "std"))]
use alloc::sync::Weak as ArcWeak;
#[cfg(feature = "std")]
use std::rc::{Rc, Weak as RcWeak};
#[cfg(feature = "std")]
use std::sync::Weak as ArcWeak;

use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};

/// Thread-local identity tracking for automatic anchor/alias emission.
///
/// Activated by `to_string_tracking_shared` (and writer variants). When active,
/// `RcAnchor`/`ArcAnchor`/`*WeakAnchor` consult this state during serialization:
/// the first time a pointer is seen, they emit a YAML anchor; subsequent
/// sightings emit an alias.
///
/// Not re-entrant across threads. `ArcAnchor` serialization remains on the
/// serialising thread; the scope guard ensures state does not leak across calls.
#[cfg(feature = "std")]
pub(crate) mod shared_tracking {
    use core::cell::RefCell;
    use rustc_hash::FxHashMap;

    pub(crate) enum TrackOutcome {
        NotActive,
        New(u32),
        Existing(u32),
    }

    struct AnchorState {
        seen: FxHashMap<usize, u32>,
        next_id: u32,
    }

    impl AnchorState {
        fn new() -> Self {
            Self {
                seen: FxHashMap::default(),
                next_id: 1,
            }
        }
    }

    std::thread_local! {
        static STATE: RefCell<Option<AnchorState>> = const { RefCell::new(None) };
    }

    /// RAII guard that installs a fresh tracking state on construction and
    /// clears it on drop. Nested scopes are rejected (only the outermost scope
    /// is authoritative) — this prevents accidental state bleed when users
    /// compose serializers.
    pub(crate) struct AnchorScope {
        owns: bool,
    }

    impl AnchorScope {
        pub(crate) fn enter() -> Self {
            let owns = STATE.with(|s| {
                let mut borrow = s.borrow_mut();
                if borrow.is_none() {
                    *borrow = Some(AnchorState::new());
                    true
                } else {
                    false
                }
            });
            AnchorScope { owns }
        }
    }

    impl Drop for AnchorScope {
        fn drop(&mut self) {
            if self.owns {
                STATE.with(|s| {
                    *s.borrow_mut() = None;
                });
            }
        }
    }

    /// Record a pointer; return whether it is newly seen or already tracked.
    pub(crate) fn track(ptr: usize) -> TrackOutcome {
        STATE.with(|s| {
            let mut borrow = s.borrow_mut();
            match borrow.as_mut() {
                None => TrackOutcome::NotActive,
                Some(state) => {
                    if let Some(&id) = state.seen.get(&ptr) {
                        TrackOutcome::Existing(id)
                    } else {
                        let id = state.next_id;
                        state.next_id = state.next_id.saturating_add(1);
                        let _ = state.seen.insert(ptr, id);
                        TrackOutcome::New(id)
                    }
                }
            }
        })
    }

    /// Look up without inserting. Used by weak-ref serializers: emit an alias
    /// only if the target was already anchored by a strong reference.
    pub(crate) fn peek(ptr: usize) -> Option<u32> {
        STATE.with(|s| {
            s.borrow()
                .as_ref()
                .and_then(|state| state.seen.get(&ptr).copied())
        })
    }

    pub(crate) fn format_id(id: u32) -> String {
        format!("id{id:03}")
    }
}

/// An `Rc` wrapper with YAML anchor semantics.
///
/// Serializes by delegating to the inner `T`. Deserializes by wrapping the
/// result in `Rc`.
#[derive(Clone)]
pub struct RcAnchor<T>(pub Rc<T>);

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
        Self(Rc::new(v))
    }
}

impl<T> From<Rc<T>> for RcAnchor<T> {
    fn from(v: Rc<T>) -> Self {
        Self(v)
    }
}

impl<T> RcAnchor<T> {
    /// Unwrap into the inner `Rc`.
    pub fn into_inner(self) -> Rc<T> {
        self.0
    }
}

impl<T: Serialize> Serialize for RcAnchor<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        #[cfg(feature = "std")]
        {
            let ptr = Rc::as_ptr(&self.0) as *const () as usize;
            match shared_tracking::track(ptr) {
                shared_tracking::TrackOutcome::NotActive => self.0.serialize(serializer),
                shared_tracking::TrackOutcome::New(id) => {
                    let id_str = shared_tracking::format_id(id);
                    serializer
                        .serialize_newtype_struct(crate::fmt::MAGIC_ANCHOR_DEF, &(id_str, &*self.0))
                }
                shared_tracking::TrackOutcome::Existing(id) => {
                    let id_str = shared_tracking::format_id(id);
                    serializer.serialize_newtype_struct(crate::fmt::MAGIC_ANCHOR_REF, &id_str)
                }
            }
        }
        #[cfg(not(feature = "std"))]
        {
            self.0.serialize(serializer)
        }
    }
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for RcAnchor<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        T::deserialize(deserializer).map(|v| RcAnchor(Rc::new(v)))
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
        #[cfg(feature = "std")]
        {
            let ptr = Arc::as_ptr(&self.0) as *const () as usize;
            match shared_tracking::track(ptr) {
                shared_tracking::TrackOutcome::NotActive => self.0.serialize(serializer),
                shared_tracking::TrackOutcome::New(id) => {
                    let id_str = shared_tracking::format_id(id);
                    serializer
                        .serialize_newtype_struct(crate::fmt::MAGIC_ANCHOR_DEF, &(id_str, &*self.0))
                }
                shared_tracking::TrackOutcome::Existing(id) => {
                    let id_str = shared_tracking::format_id(id);
                    serializer.serialize_newtype_struct(crate::fmt::MAGIC_ANCHOR_REF, &id_str)
                }
            }
        }
        #[cfg(not(feature = "std"))]
        {
            self.0.serialize(serializer)
        }
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
pub struct RcWeakAnchor<T>(pub RcWeak<T>);

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
        Self(RcWeak::new())
    }

    /// Unwrap into the inner `Weak`.
    pub fn into_inner(self) -> RcWeak<T> {
        self.0
    }

    /// Attempt to upgrade to a strong `Rc`.
    pub fn upgrade(&self) -> Option<Rc<T>> {
        self.0.upgrade()
    }
}

impl<T> From<RcWeak<T>> for RcWeakAnchor<T> {
    fn from(v: RcWeak<T>) -> Self {
        Self(v)
    }
}

impl<T: Serialize> Serialize for RcWeakAnchor<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self.0.upgrade() {
            Some(v) => {
                #[cfg(feature = "std")]
                {
                    // Weak refs never define a new anchor. If tracking is active
                    // and the target was already anchored by a strong reference,
                    // emit an alias; otherwise fall back to inline value.
                    let ptr = Rc::as_ptr(&v) as *const () as usize;
                    if let Some(id) = shared_tracking::peek(ptr) {
                        let id_str = shared_tracking::format_id(id);
                        return serializer
                            .serialize_newtype_struct(crate::fmt::MAGIC_ANCHOR_REF, &id_str);
                    }
                }
                v.serialize(serializer)
            }
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
        Ok(RcWeakAnchor(RcWeak::new()))
    }
}

/// A weak `Arc` reference with YAML anchor semantics.
///
/// Serializes as `null` if the reference is dangling, otherwise serializes
/// the inner value. Deserialization from `null` produces a dangling weak ref.
#[derive(Clone)]
pub struct ArcWeakAnchor<T>(pub ArcWeak<T>);

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
        Self(ArcWeak::new())
    }

    /// Unwrap into the inner `Weak`.
    pub fn into_inner(self) -> ArcWeak<T> {
        self.0
    }

    /// Attempt to upgrade to a strong `Arc`.
    pub fn upgrade(&self) -> Option<Arc<T>> {
        self.0.upgrade()
    }
}

impl<T> From<ArcWeak<T>> for ArcWeakAnchor<T> {
    fn from(v: ArcWeak<T>) -> Self {
        Self(v)
    }
}

impl<T: Serialize> Serialize for ArcWeakAnchor<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self.0.upgrade() {
            Some(v) => {
                #[cfg(feature = "std")]
                {
                    let ptr = Arc::as_ptr(&v) as *const () as usize;
                    if let Some(id) = shared_tracking::peek(ptr) {
                        let id_str = shared_tracking::format_id(id);
                        return serializer
                            .serialize_newtype_struct(crate::fmt::MAGIC_ANCHOR_REF, &id_str);
                    }
                }
                v.serialize(serializer)
            }
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
        Ok(ArcWeakAnchor(ArcWeak::new()))
    }
}

// ── Anchor Registries ──────────────────────────────────────────────────

/// Registry for shared `Rc` anchor references during deserialization.
///
/// When the same YAML anchor is referenced multiple times, all aliases
/// point to the same heap allocation via `Rc::clone`. This enables
/// true shared-memory DAG structures rather than duplicated subtrees.
///
/// # Example
///
/// ```rust
/// use noyalib::AnchorRegistry;
/// use std::rc::Rc;
///
/// let mut reg = AnchorRegistry::<String>::new();
/// let rc = reg.register("shared".into(), "hello".into());
/// let alias = reg.resolve("shared").unwrap();
/// assert!(Rc::ptr_eq(&rc, &alias));
/// ```
pub struct AnchorRegistry<T> {
    anchors: FxHashMap<String, Rc<T>>,
}

impl<T: fmt::Debug> fmt::Debug for AnchorRegistry<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AnchorRegistry")
            .field("len", &self.anchors.len())
            .finish()
    }
}

impl<T> Default for AnchorRegistry<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> AnchorRegistry<T> {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self {
            anchors: FxHashMap::default(),
        }
    }

    /// Register a value under the given anchor name.
    ///
    /// Returns the `Rc` wrapping the value. If an anchor with the
    /// same name already existed, the old entry is replaced.
    pub fn register(&mut self, name: String, value: T) -> Rc<T> {
        let rc = Rc::new(value);
        let _ = self.anchors.insert(name, Rc::clone(&rc));
        rc
    }

    /// Resolve an anchor by name, returning a cloned `Rc` if present.
    pub fn resolve(&self, name: &str) -> Option<Rc<T>> {
        self.anchors.get(name).cloned()
    }

    /// Returns the number of registered anchors.
    pub fn len(&self) -> usize {
        self.anchors.len()
    }

    /// Returns `true` if no anchors are registered.
    pub fn is_empty(&self) -> bool {
        self.anchors.is_empty()
    }

    /// Remove all entries from the registry.
    pub fn clear(&mut self) {
        self.anchors.clear();
    }
}

/// Registry for shared `Arc` anchor references during deserialization.
///
/// Thread-safe counterpart to [`AnchorRegistry`]. All aliases for the
/// same anchor share one `Arc` allocation, enabling cross-thread DAGs.
///
/// # Example
///
/// ```rust
/// use noyalib::ArcAnchorRegistry;
/// use std::sync::Arc;
///
/// let mut reg = ArcAnchorRegistry::<String>::new();
/// let arc = reg.register("shared".into(), "hello".into());
/// let alias = reg.resolve("shared").unwrap();
/// assert!(Arc::ptr_eq(&arc, &alias));
/// ```
pub struct ArcAnchorRegistry<T> {
    anchors: FxHashMap<String, Arc<T>>,
}

impl<T: fmt::Debug> fmt::Debug for ArcAnchorRegistry<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ArcAnchorRegistry")
            .field("len", &self.anchors.len())
            .finish()
    }
}

impl<T> Default for ArcAnchorRegistry<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> ArcAnchorRegistry<T> {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self {
            anchors: FxHashMap::default(),
        }
    }

    /// Register a value under the given anchor name.
    ///
    /// Returns the `Arc` wrapping the value.
    pub fn register(&mut self, name: String, value: T) -> Arc<T> {
        let arc = Arc::new(value);
        let _ = self.anchors.insert(name, Arc::clone(&arc));
        arc
    }

    /// Resolve an anchor by name, returning a cloned `Arc` if present.
    pub fn resolve(&self, name: &str) -> Option<Arc<T>> {
        self.anchors.get(name).cloned()
    }

    /// Returns the number of registered anchors.
    pub fn len(&self) -> usize {
        self.anchors.len()
    }

    /// Returns `true` if no anchors are registered.
    pub fn is_empty(&self) -> bool {
        self.anchors.is_empty()
    }

    /// Remove all entries from the registry.
    pub fn clear(&mut self) {
        self.anchors.clear();
    }
}
