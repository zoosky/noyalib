//! YAML value types.

use std::borrow::Cow;
use std::cmp::Ordering;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::ops::{Index, IndexMut};
use std::str::FromStr;

use indexmap::map::{IntoIter, Iter, IterMut, Keys, Values, ValuesMut};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

/// A YAML mapping (dictionary/object).
///
/// This is an ordered map that preserves insertion order, wrapping
/// `IndexMap<String, Value>`. It provides a comprehensive API for working with
/// YAML mappings.
///
/// # Example
///
/// ```rust
/// use noyalib::{Mapping, Value};
///
/// let mut map = Mapping::new();
/// map.insert("name", Value::from("test"));
/// map.insert("value", Value::from(42));
///
/// assert_eq!(map.len(), 2);
/// assert_eq!(map.get("name").unwrap().as_str(), Some("test"));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Mapping(IndexMap<String, Value>);

impl Mapping {
    /// Creates an empty mapping.
    #[must_use]
    pub fn new() -> Self {
        Self(IndexMap::new())
    }

    /// Creates an empty mapping with the specified capacity.
    #[must_use]
    pub fn with_capacity(capacity: usize) -> Self {
        Self(IndexMap::with_capacity(capacity))
    }

    /// Returns the number of key-value pairs the mapping can hold without
    /// reallocating.
    #[must_use]
    pub fn capacity(&self) -> usize {
        self.0.capacity()
    }

    /// Reserves capacity for at least `additional` more key-value pairs.
    pub fn reserve(&mut self, additional: usize) {
        self.0.reserve(additional);
    }

    /// Shrinks the capacity of the mapping as much as possible.
    pub fn shrink_to_fit(&mut self) {
        self.0.shrink_to_fit();
    }

    /// Returns the number of key-value pairs in the mapping.
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns `true` if the mapping contains no key-value pairs.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Clears the mapping, removing all key-value pairs.
    pub fn clear(&mut self) {
        self.0.clear();
    }

    /// Inserts a key-value pair into the mapping.
    ///
    /// If the mapping already had this key present, the value is updated,
    /// and the old value is returned.
    pub fn insert(&mut self, key: impl Into<String>, value: Value) -> Option<Value> {
        self.0.insert(key.into(), value)
    }

    /// Returns `true` if the mapping contains the specified key.
    #[must_use]
    pub fn contains_key(&self, key: &str) -> bool {
        self.0.contains_key(key)
    }

    /// Returns a reference to the value corresponding to the key.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.0.get(key)
    }

    /// Returns a mutable reference to the value corresponding to the key.
    #[must_use]
    pub fn get_mut(&mut self, key: &str) -> Option<&mut Value> {
        self.0.get_mut(key)
    }

    /// Returns a reference to the key-value pair at the given index.
    #[must_use]
    pub fn get_index(&self, index: usize) -> Option<(&String, &Value)> {
        self.0.get_index(index)
    }

    /// Returns a mutable reference to the key-value pair at the given index.
    #[must_use]
    pub fn get_index_mut(&mut self, index: usize) -> Option<(&String, &mut Value)> {
        self.0.get_index_mut(index)
    }

    /// Removes a key from the mapping, returning the value if the key was
    /// present.
    ///
    /// This operation preserves the order of remaining elements.
    pub fn remove(&mut self, key: &str) -> Option<Value> {
        self.0.shift_remove(key)
    }

    /// Removes a key from the mapping, returning the key-value pair if present.
    ///
    /// This operation preserves the order of remaining elements.
    pub fn remove_entry(&mut self, key: &str) -> Option<(String, Value)> {
        self.0.shift_remove_entry(key)
    }

    /// Removes a key by swapping it with the last element.
    ///
    /// This is faster than `remove` but does not preserve order.
    pub fn swap_remove(&mut self, key: &str) -> Option<Value> {
        self.0.swap_remove(key)
    }

    /// Removes a key by shifting all elements after it.
    ///
    /// This preserves order but is slower than `swap_remove`.
    pub fn shift_remove(&mut self, key: &str) -> Option<Value> {
        self.0.shift_remove(key)
    }

    /// Gets the entry for the given key for in-place manipulation.
    pub fn entry(&mut self, key: impl Into<String>) -> indexmap::map::Entry<'_, String, Value> {
        self.0.entry(key.into())
    }

    /// Retains only the key-value pairs specified by the predicate.
    pub fn retain<F>(&mut self, f: F)
    where
        F: FnMut(&String, &mut Value) -> bool,
    {
        self.0.retain(f);
    }

    /// Returns an iterator over the key-value pairs.
    pub fn iter(&self) -> Iter<'_, String, Value> {
        self.0.iter()
    }

    /// Returns a mutable iterator over the key-value pairs.
    pub fn iter_mut(&mut self) -> IterMut<'_, String, Value> {
        self.0.iter_mut()
    }

    /// Returns an iterator over the keys.
    pub fn keys(&self) -> Keys<'_, String, Value> {
        self.0.keys()
    }

    /// Returns an iterator over the values.
    pub fn values(&self) -> Values<'_, String, Value> {
        self.0.values()
    }

    /// Returns a mutable iterator over the values.
    pub fn values_mut(&mut self) -> ValuesMut<'_, String, Value> {
        self.0.values_mut()
    }

    /// Returns the first key-value pair.
    #[must_use]
    pub fn first(&self) -> Option<(&String, &Value)> {
        self.0.first()
    }

    /// Returns a mutable reference to the first key-value pair.
    #[must_use]
    pub fn first_mut(&mut self) -> Option<(&String, &mut Value)> {
        self.0.first_mut()
    }

    /// Returns the last key-value pair.
    #[must_use]
    pub fn last(&self) -> Option<(&String, &Value)> {
        self.0.last()
    }

    /// Returns a mutable reference to the last key-value pair.
    #[must_use]
    pub fn last_mut(&mut self) -> Option<(&String, &mut Value)> {
        self.0.last_mut()
    }

    /// Removes and returns the first key-value pair.
    pub fn pop_first(&mut self) -> Option<(String, Value)> {
        self.0.shift_remove_index(0)
    }

    /// Removes and returns the last key-value pair.
    pub fn pop_last(&mut self) -> Option<(String, Value)> {
        self.0.pop()
    }

    /// Sorts the mapping by keys.
    pub fn sort_keys(&mut self) {
        self.0.sort_keys();
    }

    /// Reverses the order of key-value pairs.
    pub fn reverse(&mut self) {
        self.0.reverse();
    }

    /// Extends the mapping with the contents of an iterator.
    pub fn extend<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = (String, Value)>,
    {
        self.0.extend(iter);
    }

    /// Returns the inner `IndexMap`.
    #[must_use]
    pub fn into_inner(self) -> IndexMap<String, Value> {
        self.0
    }

    /// Creates a mapping from an `IndexMap`.
    #[must_use]
    pub fn from_inner(map: IndexMap<String, Value>) -> Self {
        Self(map)
    }
}

impl Index<&str> for Mapping {
    type Output = Value;

    /// Index into the mapping by key.
    ///
    /// # Panics
    ///
    /// Panics if the key is not present in the mapping.
    fn index(&self, key: &str) -> &Self::Output {
        self.0.get(key).expect("key not found in mapping")
    }
}

impl IndexMut<&str> for Mapping {
    /// Mutably index into the mapping by key.
    ///
    /// # Panics
    ///
    /// Panics if the key is not present in the mapping.
    fn index_mut(&mut self, key: &str) -> &mut Self::Output {
        self.0.get_mut(key).expect("key not found in mapping")
    }
}

impl IntoIterator for Mapping {
    type Item = (String, Value);
    type IntoIter = IntoIter<String, Value>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a> IntoIterator for &'a Mapping {
    type Item = (&'a String, &'a Value);
    type IntoIter = Iter<'a, String, Value>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl<'a> IntoIterator for &'a mut Mapping {
    type Item = (&'a String, &'a mut Value);
    type IntoIter = IterMut<'a, String, Value>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter_mut()
    }
}

impl FromIterator<(String, Value)> for Mapping {
    fn from_iter<I: IntoIterator<Item = (String, Value)>>(iter: I) -> Self {
        Self(IndexMap::from_iter(iter))
    }
}

impl<const N: usize> From<[(String, Value); N]> for Mapping {
    fn from(arr: [(String, Value); N]) -> Self {
        Self(IndexMap::from(arr))
    }
}

impl From<IndexMap<String, Value>> for Mapping {
    fn from(map: IndexMap<String, Value>) -> Self {
        Self(map)
    }
}

impl From<Mapping> for IndexMap<String, Value> {
    fn from(map: Mapping) -> Self {
        map.0
    }
}

impl From<Vec<(String, Value)>> for Mapping {
    fn from(v: Vec<(String, Value)>) -> Self {
        Self(v.into_iter().collect())
    }
}

impl Hash for Mapping {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.len().hash(state);
        for (k, v) in &self.0 {
            k.hash(state);
            v.hash(state);
        }
    }
}

impl PartialOrd for Mapping {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Mapping {
    fn cmp(&self, other: &Self) -> Ordering {
        self.len().cmp(&other.len()).then_with(|| {
            for ((ak, av), (bk, bv)) in self.iter().zip(other.iter()) {
                match ak.cmp(bk) {
                    Ordering::Equal => {}
                    ord => return ord,
                }
                match av.cmp(bv) {
                    Ordering::Equal => continue,
                    ord => return ord,
                }
            }
            Ordering::Equal
        })
    }
}

impl fmt::Display for Mapping {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{{")?;
        for (i, (k, v)) in self.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{k}: {v}")?;
        }
        write!(f, "}}")
    }
}

impl Serialize for Mapping {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(self.len()))?;
        for (k, v) in self {
            map.serialize_entry(k, v)?;
        }
        map.end()
    }
}

impl<'de> Deserialize<'de> for Mapping {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::{MapAccess, Visitor};

        struct MappingVisitor;

        impl<'de> Visitor<'de> for MappingVisitor {
            type Value = Mapping;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a YAML mapping")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Mapping, A::Error>
            where
                A: MapAccess<'de>,
            {
                let mut mapping = Mapping::with_capacity(map.size_hint().unwrap_or(0));
                while let Some((key, value)) = map.next_entry::<String, Value>()? {
                    let _ = mapping.insert(key, value);
                }
                Ok(mapping)
            }
        }

        deserializer.deserialize_map(MappingVisitor)
    }
}

/// A YAML mapping with `Value` keys.
///
/// Unlike [`Mapping`] which only supports `String` keys, `MappingAny` allows
/// any [`Value`] as a key. This is useful for representing YAML mappings where
/// keys might be numbers, booleans, or even nested structures.
///
/// # Example
///
/// ```rust
/// use noyalib::{MappingAny, Value};
///
/// let mut map = MappingAny::new();
/// map.insert(Value::from(1), Value::from("one"));
/// map.insert(Value::from("two"), Value::from(2));
/// map.insert(Value::Bool(true), Value::from("yes"));
///
/// assert_eq!(map.len(), 3);
/// assert_eq!(map.get(&Value::from(1)).unwrap().as_str(), Some("one"));
/// ```
///
/// # YAML Example
///
/// This type can represent YAML like:
///
/// ```yaml
/// 1: one
/// "two": 2
/// true: yes
/// [1, 2]: nested key
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MappingAny(IndexMap<Value, Value>);

impl MappingAny {
    /// Creates an empty mapping.
    #[must_use]
    pub fn new() -> Self {
        Self(IndexMap::new())
    }

    /// Creates an empty mapping with the specified capacity.
    #[must_use]
    pub fn with_capacity(capacity: usize) -> Self {
        Self(IndexMap::with_capacity(capacity))
    }

    /// Returns the number of key-value pairs the mapping can hold without
    /// reallocating.
    #[must_use]
    pub fn capacity(&self) -> usize {
        self.0.capacity()
    }

    /// Reserves capacity for at least `additional` more key-value pairs.
    pub fn reserve(&mut self, additional: usize) {
        self.0.reserve(additional);
    }

    /// Shrinks the capacity of the mapping as much as possible.
    pub fn shrink_to_fit(&mut self) {
        self.0.shrink_to_fit();
    }

    /// Returns the number of key-value pairs in the mapping.
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns `true` if the mapping contains no key-value pairs.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Clears the mapping, removing all key-value pairs.
    pub fn clear(&mut self) {
        self.0.clear();
    }

    /// Inserts a key-value pair into the mapping.
    ///
    /// If the mapping already had this key present, the value is updated,
    /// and the old value is returned.
    pub fn insert(&mut self, key: Value, value: Value) -> Option<Value> {
        self.0.insert(key, value)
    }

    /// Returns `true` if the mapping contains the specified key.
    #[must_use]
    pub fn contains_key(&self, key: &Value) -> bool {
        self.0.contains_key(key)
    }

    /// Returns a reference to the value corresponding to the key.
    #[must_use]
    pub fn get(&self, key: &Value) -> Option<&Value> {
        self.0.get(key)
    }

    /// Returns a mutable reference to the value corresponding to the key.
    #[must_use]
    pub fn get_mut(&mut self, key: &Value) -> Option<&mut Value> {
        self.0.get_mut(key)
    }

    /// Returns a reference to the key-value pair at the given index.
    #[must_use]
    pub fn get_index(&self, index: usize) -> Option<(&Value, &Value)> {
        self.0.get_index(index)
    }

    /// Returns a mutable reference to the key-value pair at the given index.
    #[must_use]
    pub fn get_index_mut(&mut self, index: usize) -> Option<(&Value, &mut Value)> {
        self.0.get_index_mut(index)
    }

    /// Removes a key from the mapping, returning the value if the key was
    /// present.
    ///
    /// This operation preserves the order of remaining elements.
    pub fn remove(&mut self, key: &Value) -> Option<Value> {
        self.0.shift_remove(key)
    }

    /// Removes a key from the mapping, returning the key-value pair if present.
    ///
    /// This operation preserves the order of remaining elements.
    pub fn remove_entry(&mut self, key: &Value) -> Option<(Value, Value)> {
        self.0.shift_remove_entry(key)
    }

    /// Removes a key by swapping it with the last element.
    ///
    /// This is faster than `remove` but does not preserve order.
    pub fn swap_remove(&mut self, key: &Value) -> Option<Value> {
        self.0.swap_remove(key)
    }

    /// Removes a key by shifting all elements after it.
    ///
    /// This preserves order but is slower than `swap_remove`.
    pub fn shift_remove(&mut self, key: &Value) -> Option<Value> {
        self.0.shift_remove(key)
    }

    /// Gets the entry for the given key for in-place manipulation.
    pub fn entry(&mut self, key: Value) -> indexmap::map::Entry<'_, Value, Value> {
        self.0.entry(key)
    }

    /// Retains only the key-value pairs specified by the predicate.
    pub fn retain<F>(&mut self, f: F)
    where
        F: FnMut(&Value, &mut Value) -> bool,
    {
        self.0.retain(f);
    }

    /// Returns an iterator over the key-value pairs.
    pub fn iter(&self) -> Iter<'_, Value, Value> {
        self.0.iter()
    }

    /// Returns a mutable iterator over the key-value pairs.
    pub fn iter_mut(&mut self) -> IterMut<'_, Value, Value> {
        self.0.iter_mut()
    }

    /// Returns an iterator over the keys.
    pub fn keys(&self) -> Keys<'_, Value, Value> {
        self.0.keys()
    }

    /// Returns an iterator over the values.
    pub fn values(&self) -> Values<'_, Value, Value> {
        self.0.values()
    }

    /// Returns a mutable iterator over the values.
    pub fn values_mut(&mut self) -> ValuesMut<'_, Value, Value> {
        self.0.values_mut()
    }

    /// Returns the first key-value pair.
    #[must_use]
    pub fn first(&self) -> Option<(&Value, &Value)> {
        self.0.first()
    }

    /// Returns a mutable reference to the first key-value pair.
    #[must_use]
    pub fn first_mut(&mut self) -> Option<(&Value, &mut Value)> {
        self.0.first_mut()
    }

    /// Returns the last key-value pair.
    #[must_use]
    pub fn last(&self) -> Option<(&Value, &Value)> {
        self.0.last()
    }

    /// Returns a mutable reference to the last key-value pair.
    #[must_use]
    pub fn last_mut(&mut self) -> Option<(&Value, &mut Value)> {
        self.0.last_mut()
    }

    /// Removes and returns the first key-value pair.
    pub fn pop_first(&mut self) -> Option<(Value, Value)> {
        self.0.shift_remove_index(0)
    }

    /// Removes and returns the last key-value pair.
    pub fn pop_last(&mut self) -> Option<(Value, Value)> {
        self.0.pop()
    }

    /// Sorts the mapping by keys.
    pub fn sort_keys(&mut self) {
        self.0.sort_keys();
    }

    /// Reverses the order of key-value pairs.
    pub fn reverse(&mut self) {
        self.0.reverse();
    }

    /// Extends the mapping with the contents of an iterator.
    pub fn extend<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = (Value, Value)>,
    {
        self.0.extend(iter);
    }

    /// Returns the inner `IndexMap`.
    #[must_use]
    pub fn into_inner(self) -> IndexMap<Value, Value> {
        self.0
    }

    /// Creates a mapping from an `IndexMap`.
    #[must_use]
    pub fn from_inner(map: IndexMap<Value, Value>) -> Self {
        Self(map)
    }

    /// Converts this `MappingAny` to a `Mapping` if all keys are strings.
    ///
    /// Returns `None` if any key is not a string value.
    ///
    /// # Example
    ///
    /// ```rust
    /// use noyalib::{Mapping, MappingAny, Value};
    ///
    /// let mut map = MappingAny::new();
    /// map.insert(Value::from("key1"), Value::from(1));
    /// map.insert(Value::from("key2"), Value::from(2));
    ///
    /// let mapping = map.into_mapping().unwrap();
    /// assert_eq!(mapping.len(), 2);
    /// ```
    #[must_use]
    pub fn into_mapping(self) -> Option<Mapping> {
        let mut mapping = Mapping::with_capacity(self.len());
        for (k, v) in self.0 {
            if let Value::String(s) = k {
                let _ = mapping.insert(s, v);
            } else {
                return None;
            }
        }
        Some(mapping)
    }
}

impl Index<&Value> for MappingAny {
    type Output = Value;

    /// Index into the mapping by key.
    ///
    /// # Panics
    ///
    /// Panics if the key is not present in the mapping.
    fn index(&self, key: &Value) -> &Self::Output {
        self.0.get(key).expect("key not found in mapping")
    }
}

impl IndexMut<&Value> for MappingAny {
    /// Mutably index into the mapping by key.
    ///
    /// # Panics
    ///
    /// Panics if the key is not present in the mapping.
    fn index_mut(&mut self, key: &Value) -> &mut Self::Output {
        self.0.get_mut(key).expect("key not found in mapping")
    }
}

impl IntoIterator for MappingAny {
    type Item = (Value, Value);
    type IntoIter = IntoIter<Value, Value>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a> IntoIterator for &'a MappingAny {
    type Item = (&'a Value, &'a Value);
    type IntoIter = Iter<'a, Value, Value>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl<'a> IntoIterator for &'a mut MappingAny {
    type Item = (&'a Value, &'a mut Value);
    type IntoIter = IterMut<'a, Value, Value>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter_mut()
    }
}

impl FromIterator<(Value, Value)> for MappingAny {
    fn from_iter<I: IntoIterator<Item = (Value, Value)>>(iter: I) -> Self {
        Self(IndexMap::from_iter(iter))
    }
}

impl<const N: usize> From<[(Value, Value); N]> for MappingAny {
    fn from(arr: [(Value, Value); N]) -> Self {
        Self(IndexMap::from(arr))
    }
}

impl From<IndexMap<Value, Value>> for MappingAny {
    fn from(map: IndexMap<Value, Value>) -> Self {
        Self(map)
    }
}

impl From<MappingAny> for IndexMap<Value, Value> {
    fn from(map: MappingAny) -> Self {
        map.0
    }
}

impl From<Mapping> for MappingAny {
    /// Converts a `Mapping` (with `String` keys) into a `MappingAny`.
    fn from(map: Mapping) -> Self {
        let mut any = MappingAny::with_capacity(map.len());
        for (k, v) in map {
            let _ = any.insert(Value::String(k), v);
        }
        any
    }
}

impl Hash for MappingAny {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.len().hash(state);
        for (k, v) in &self.0 {
            k.hash(state);
            v.hash(state);
        }
    }
}

impl PartialOrd for MappingAny {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for MappingAny {
    fn cmp(&self, other: &Self) -> Ordering {
        self.len().cmp(&other.len()).then_with(|| {
            for ((ak, av), (bk, bv)) in self.iter().zip(other.iter()) {
                match ak.cmp(bk) {
                    Ordering::Equal => {}
                    ord => return ord,
                }
                match av.cmp(bv) {
                    Ordering::Equal => continue,
                    ord => return ord,
                }
            }
            Ordering::Equal
        })
    }
}

impl fmt::Display for MappingAny {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{{")?;
        for (i, (k, v)) in self.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{k}: {v}")?;
        }
        write!(f, "}}")
    }
}

impl Serialize for MappingAny {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(self.len()))?;
        for (k, v) in self {
            map.serialize_entry(k, v)?;
        }
        map.end()
    }
}

impl<'de> Deserialize<'de> for MappingAny {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::{MapAccess, Visitor};

        struct MappingAnyVisitor;

        impl<'de> Visitor<'de> for MappingAnyVisitor {
            type Value = MappingAny;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a YAML mapping")
            }

            fn visit_map<A>(self, mut map: A) -> Result<MappingAny, A::Error>
            where
                A: MapAccess<'de>,
            {
                let mut mapping = MappingAny::with_capacity(map.size_hint().unwrap_or(0));
                while let Some((key, value)) = map.next_entry::<Value, Value>()? {
                    let _ = mapping.insert(key, value);
                }
                Ok(mapping)
            }
        }

        deserializer.deserialize_map(MappingAnyVisitor)
    }
}

/// A YAML sequence (array/list).
pub type Sequence = Vec<Value>;

/// Represents a YAML number.
#[derive(Debug, Clone, Copy)]
pub enum Number {
    /// A signed integer.
    Integer(i64),
    /// A floating-point number.
    Float(f64),
}

impl Number {
    /// Returns the number as an i64 if it is an integer.
    #[must_use]
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Number::Integer(n) => Some(*n),
            Number::Float(_) => None,
        }
    }

    /// Returns the number as a u64 if it is a non-negative integer in range.
    #[must_use]
    pub fn as_u64(&self) -> Option<u64> {
        match self {
            Number::Integer(n) if *n >= 0 => Some(*n as u64),
            _ => None,
        }
    }

    /// Returns the number as an f64.
    #[must_use]
    pub fn as_f64(&self) -> f64 {
        match self {
            Number::Integer(n) => *n as f64,
            Number::Float(n) => *n,
        }
    }

    /// Returns true if the number is an integer.
    #[must_use]
    pub fn is_integer(&self) -> bool {
        matches!(self, Number::Integer(_))
    }

    /// Returns true if the number is a float.
    #[must_use]
    pub fn is_float(&self) -> bool {
        matches!(self, Number::Float(_))
    }

    /// Returns true if the number can be represented as an i64.
    ///
    /// This is true for all integer values.
    #[must_use]
    pub fn is_i64(&self) -> bool {
        matches!(self, Number::Integer(_))
    }

    /// Returns true if the number can be represented as a u64.
    ///
    /// This is true for non-negative integer values.
    #[must_use]
    pub fn is_u64(&self) -> bool {
        matches!(self, Number::Integer(n) if *n >= 0)
    }

    /// Returns true if the number can be represented as an f64.
    ///
    /// This is always true as both integers and floats can be converted to f64.
    #[must_use]
    pub fn is_f64(&self) -> bool {
        true
    }

    /// Returns true if the number is NaN (Not a Number).
    #[must_use]
    pub fn is_nan(&self) -> bool {
        match self {
            Number::Float(n) => n.is_nan(),
            Number::Integer(_) => false,
        }
    }

    /// Returns true if the number is positive or negative infinity.
    #[must_use]
    pub fn is_infinite(&self) -> bool {
        match self {
            Number::Float(n) => n.is_infinite(),
            Number::Integer(_) => false,
        }
    }

    /// Returns true if the number is neither infinite nor NaN.
    #[must_use]
    pub fn is_finite(&self) -> bool {
        match self {
            Number::Float(n) => n.is_finite(),
            Number::Integer(_) => true,
        }
    }
}

impl fmt::Display for Number {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Number::Integer(n) => write!(f, "{n}"),
            Number::Float(n) => write!(f, "{n}"),
        }
    }
}

/// Error returned when parsing a number from a string fails.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParseNumberError {
    _private: (),
}

impl fmt::Display for ParseNumberError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid number")
    }
}

impl std::error::Error for ParseNumberError {}

impl FromStr for Number {
    type Err = ParseNumberError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();

        // Handle special float values
        match s {
            ".nan" | ".NaN" | ".NAN" => return Ok(Number::Float(f64::NAN)),
            ".inf" | ".Inf" | ".INF" => return Ok(Number::Float(f64::INFINITY)),
            "+.inf" | "+.Inf" | "+.INF" => return Ok(Number::Float(f64::INFINITY)),
            "-.inf" | "-.Inf" | "-.INF" => return Ok(Number::Float(f64::NEG_INFINITY)),
            _ => {}
        }

        // Try parsing as integer first
        if let Ok(n) = s.parse::<i64>() {
            return Ok(Number::Integer(n));
        }

        // Handle hex (0x), octal (0o), and binary (0b) integers
        if s.len() > 2 {
            let (prefix, rest) = s.split_at(2);
            match prefix {
                "0x" | "0X" => {
                    if let Ok(n) = i64::from_str_radix(rest, 16) {
                        return Ok(Number::Integer(n));
                    }
                }
                "0o" | "0O" => {
                    if let Ok(n) = i64::from_str_radix(rest, 8) {
                        return Ok(Number::Integer(n));
                    }
                }
                "0b" | "0B" => {
                    if let Ok(n) = i64::from_str_radix(rest, 2) {
                        return Ok(Number::Integer(n));
                    }
                }
                _ => {}
            }
        }

        // Try parsing as float
        if let Ok(n) = s.parse::<f64>() {
            return Ok(Number::Float(n));
        }

        Err(ParseNumberError { _private: () })
    }
}

impl PartialEq for Number {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Number::Integer(a), Number::Integer(b)) => a == b,
            (Number::Float(a), Number::Float(b)) => {
                // Treat NaN == NaN to satisfy the Eq contract (reflexivity)
                (a.is_nan() && b.is_nan()) || a == b
            }
            _ => false,
        }
    }
}

impl Eq for Number {}

impl Hash for Number {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            Number::Integer(n) => {
                0u8.hash(state);
                n.hash(state);
            }
            Number::Float(n) => {
                1u8.hash(state);
                // Use bits for hashing floats - NaN values will hash consistently
                n.to_bits().hash(state);
            }
        }
    }
}

impl PartialOrd for Number {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Number {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            (Number::Integer(a), Number::Integer(b)) => a.cmp(b),
            (Number::Float(a), Number::Float(b)) => {
                // Handle NaN: treat all NaN as equal and greater than any non-NaN
                match (a.is_nan(), b.is_nan()) {
                    (true, true) => Ordering::Equal,
                    (true, false) => Ordering::Greater,
                    (false, true) => Ordering::Less,
                    (false, false) => a.partial_cmp(b).unwrap_or(Ordering::Equal),
                }
            }
            (Number::Integer(a), Number::Float(b)) => {
                let a_f = *a as f64;
                if b.is_nan() {
                    Ordering::Less
                } else {
                    a_f.partial_cmp(b).unwrap_or(Ordering::Equal)
                }
            }
            (Number::Float(a), Number::Integer(b)) => {
                let b_f = *b as f64;
                if a.is_nan() {
                    Ordering::Greater
                } else {
                    a.partial_cmp(&b_f).unwrap_or(Ordering::Equal)
                }
            }
        }
    }
}

// ============================================================================
// Number From impls
// ============================================================================

impl From<i8> for Number {
    fn from(v: i8) -> Self {
        Number::Integer(i64::from(v))
    }
}

impl From<i16> for Number {
    fn from(v: i16) -> Self {
        Number::Integer(i64::from(v))
    }
}

impl From<i32> for Number {
    fn from(v: i32) -> Self {
        Number::Integer(i64::from(v))
    }
}

impl From<i64> for Number {
    fn from(v: i64) -> Self {
        Number::Integer(v)
    }
}

impl From<isize> for Number {
    fn from(v: isize) -> Self {
        Number::Integer(v as i64)
    }
}

impl From<u8> for Number {
    fn from(v: u8) -> Self {
        Number::Integer(i64::from(v))
    }
}

impl From<u16> for Number {
    fn from(v: u16) -> Self {
        Number::Integer(i64::from(v))
    }
}

impl From<u32> for Number {
    fn from(v: u32) -> Self {
        Number::Integer(i64::from(v))
    }
}

impl From<u64> for Number {
    fn from(v: u64) -> Self {
        if v <= i64::MAX as u64 {
            Number::Integer(v as i64)
        } else {
            Number::Float(v as f64)
        }
    }
}

impl From<usize> for Number {
    fn from(v: usize) -> Self {
        Number::from(v as u64)
    }
}

impl From<f32> for Number {
    fn from(v: f32) -> Self {
        Number::Float(f64::from(v))
    }
}

impl From<f64> for Number {
    fn from(v: f64) -> Self {
        Number::Float(v)
    }
}

// ============================================================================
// Tag utilities
// ============================================================================

/// Strips a leading `!` from a string, if present.
///
/// # Example
///
/// ```rust
/// use noyalib::nobang;
///
/// assert_eq!(nobang("!foo"), "foo");
/// assert_eq!(nobang("foo"), "foo");
/// assert_eq!(nobang("!!int"), "!int");
/// ```
#[must_use]
pub fn nobang(s: &str) -> &str {
    s.strip_prefix('!').unwrap_or(s)
}

/// Result of checking whether a value looks like a YAML tag.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MaybeTag<T> {
    /// The value is a tag (starts with `!`).
    Tag(String),
    /// The value is not a tag.
    NotTag(T),
}

/// Checks whether a value's display representation looks like a YAML tag.
///
/// A value is considered a tag if its string representation starts with `!`.
///
/// # Example
///
/// ```rust
/// use noyalib::{check_for_tag, MaybeTag};
///
/// let result = check_for_tag(&"!mytag");
/// assert!(matches!(result, MaybeTag::Tag(_)));
///
/// let result = check_for_tag(&"plain");
/// assert!(matches!(result, MaybeTag::NotTag(_)));
/// ```
pub fn check_for_tag<T: fmt::Display>(value: &T) -> MaybeTag<String> {
    let s = value.to_string();
    if s.starts_with('!') {
        MaybeTag::Tag(s)
    } else {
        MaybeTag::NotTag(s)
    }
}

/// A YAML tag.
///
/// Tags are used in YAML to denote the type of a value.
/// For example, `!custom_type value` has the tag `!custom_type`.
///
/// Tag comparison ignores a leading `!` prefix, so `Tag::new("!foo") ==
/// Tag::new("foo")`.
///
/// # Example
///
/// ```rust
/// use noyalib::Tag;
///
/// let tag = Tag::new("!custom");
/// assert_eq!(tag.as_str(), "!custom");
/// assert_eq!(Tag::new("!foo"), Tag::new("foo"));
/// ```
#[derive(Debug, Clone)]
pub struct Tag(String);

impl Tag {
    /// Creates a new tag from a string.
    #[must_use]
    pub fn new(tag: impl Into<String>) -> Self {
        Self(tag.into())
    }

    /// Returns the tag as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consumes the tag and returns the inner string.
    #[must_use]
    pub fn into_string(self) -> String {
        self.0
    }

    /// Returns the tag string without a leading `!`, if present.
    #[must_use]
    pub fn nobang(&self) -> &str {
        nobang(&self.0)
    }
}

impl fmt::Display for Tag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for Tag {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

impl From<String> for Tag {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl AsRef<str> for Tag {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl PartialEq for Tag {
    fn eq(&self, other: &Self) -> bool {
        nobang(&self.0) == nobang(&other.0)
    }
}

impl Eq for Tag {}

impl Hash for Tag {
    fn hash<H: Hasher>(&self, state: &mut H) {
        nobang(&self.0).hash(state);
    }
}

impl PartialOrd for Tag {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Tag {
    fn cmp(&self, other: &Self) -> Ordering {
        nobang(&self.0).cmp(nobang(&other.0))
    }
}

impl TryFrom<&[u8]> for Tag {
    type Error = std::str::Utf8Error;

    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        std::str::from_utf8(bytes).map(Tag::new)
    }
}

/// A tagged YAML value.
///
/// Represents a value with an explicit YAML tag, such as `!custom_type value`.
/// Tags are used to specify the type or interpretation of a value.
///
/// # Example
///
/// ```rust
/// use noyalib::{Tag, TaggedValue, Value};
///
/// let tagged = TaggedValue::new(
///     Tag::new("!timestamp"),
///     Value::String("2024-01-01".to_string()),
/// );
/// assert_eq!(tagged.tag().as_str(), "!timestamp");
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct TaggedValue {
    /// The tag.
    tag: Tag,
    /// The value.
    value: Box<Value>,
}

impl TaggedValue {
    /// Creates a new tagged value.
    #[must_use]
    pub fn new(tag: Tag, value: Value) -> Self {
        Self {
            tag,
            value: Box::new(value),
        }
    }

    /// Returns a reference to the tag.
    #[must_use]
    pub fn tag(&self) -> &Tag {
        &self.tag
    }

    /// Returns a reference to the value.
    #[must_use]
    pub fn value(&self) -> &Value {
        &self.value
    }

    /// Returns a mutable reference to the value.
    #[must_use]
    pub fn value_mut(&mut self) -> &mut Value {
        &mut self.value
    }

    /// Consumes the tagged value and returns the tag and value.
    #[must_use]
    pub fn into_parts(self) -> (Tag, Value) {
        (self.tag, *self.value)
    }
}

impl fmt::Display for TaggedValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.tag, self.value)
    }
}

impl Serialize for TaggedValue {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(1))?;
        map.serialize_entry(self.tag.as_str(), self.value())?;
        map.end()
    }
}

impl<'de> Deserialize<'de> for TaggedValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::{MapAccess, Visitor};

        struct TaggedValueVisitor;

        impl<'de> Visitor<'de> for TaggedValueVisitor {
            type Value = TaggedValue;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a single-entry map representing a tagged value")
            }

            fn visit_map<A>(self, mut map: A) -> Result<TaggedValue, A::Error>
            where
                A: MapAccess<'de>,
            {
                let (tag, value): (String, Value) = map
                    .next_entry()?
                    .ok_or_else(|| serde::de::Error::custom("expected a single-entry map"))?;
                Ok(TaggedValue::new(Tag::new(tag), value))
            }
        }

        deserializer.deserialize_map(TaggedValueVisitor)
    }
}

impl<'de> serde::Deserializer<'de> for &'de TaggedValue {
    type Error = crate::Error;

    fn deserialize_any<V>(self, visitor: V) -> crate::Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_map(TaggedValueMapAccess {
            tag: Some(self.tag.as_str()),
            value: Some(self.value()),
        })
    }

    fn deserialize_enum<V>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> crate::Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_enum(TaggedValueEnumAccess { tagged: self })
    }

    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 u8 u16 u32 u64 f32 f64 char str string bytes
        byte_buf option unit unit_struct newtype_struct seq tuple
        tuple_struct map struct identifier ignored_any
    }
}

struct TaggedValueMapAccess<'de> {
    tag: Option<&'de str>,
    value: Option<&'de Value>,
}

impl<'de> serde::de::MapAccess<'de> for TaggedValueMapAccess<'de> {
    type Error = crate::Error;

    fn next_key_seed<K>(&mut self, seed: K) -> crate::Result<Option<K::Value>>
    where
        K: serde::de::DeserializeSeed<'de>,
    {
        match self.tag.take() {
            Some(tag) => seed
                .deserialize(serde::de::value::BorrowedStrDeserializer::new(tag))
                .map(Some),
            None => Ok(None),
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> crate::Result<V::Value>
    where
        V: serde::de::DeserializeSeed<'de>,
    {
        match self.value.take() {
            Some(value) => seed.deserialize(value),
            None => Err(serde::de::Error::custom("value is missing")),
        }
    }
}

struct TaggedValueEnumAccess<'de> {
    tagged: &'de TaggedValue,
}

impl<'de> serde::de::EnumAccess<'de> for TaggedValueEnumAccess<'de> {
    type Error = crate::Error;
    type Variant = TaggedValueVariantAccess<'de>;

    fn variant_seed<V>(self, seed: V) -> crate::Result<(V::Value, Self::Variant)>
    where
        V: serde::de::DeserializeSeed<'de>,
    {
        let variant = seed.deserialize(
            serde::de::value::BorrowedStrDeserializer::<crate::Error>::new(
                self.tagged.tag.nobang(),
            ),
        )?;
        Ok((
            variant,
            TaggedValueVariantAccess {
                value: self.tagged.value(),
            },
        ))
    }
}

struct TaggedValueVariantAccess<'de> {
    value: &'de Value,
}

impl<'de> serde::de::VariantAccess<'de> for TaggedValueVariantAccess<'de> {
    type Error = crate::Error;

    fn unit_variant(self) -> crate::Result<()> {
        Ok(())
    }

    fn newtype_variant_seed<T>(self, seed: T) -> crate::Result<T::Value>
    where
        T: serde::de::DeserializeSeed<'de>,
    {
        seed.deserialize(self.value)
    }

    fn tuple_variant<V>(self, _len: usize, visitor: V) -> crate::Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        serde::Deserializer::deserialize_seq(self.value, visitor)
    }

    fn struct_variant<V>(
        self,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> crate::Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        serde::Deserializer::deserialize_map(self.value, visitor)
    }
}

/// Represents any valid YAML value.
#[derive(Debug, Clone, Default)]
pub enum Value {
    /// Represents a YAML null value.
    #[default]
    Null,
    /// Represents a YAML boolean.
    Bool(bool),
    /// Represents a YAML number (integer or float).
    Number(Number),
    /// Represents a YAML string.
    String(String),
    /// Represents a YAML sequence (array).
    Sequence(Sequence),
    /// Represents a YAML mapping (object).
    Mapping(Mapping),
    /// Represents a tagged YAML value.
    Tagged(Box<TaggedValue>),
}

impl Value {
    /// Returns true if the value is null.
    #[must_use]
    pub fn is_null(&self) -> bool {
        matches!(self, Value::Null)
    }

    /// Returns true if the value is a boolean.
    #[must_use]
    pub fn is_bool(&self) -> bool {
        matches!(self, Value::Bool(_))
    }

    /// Returns true if the value is a number.
    #[must_use]
    pub fn is_number(&self) -> bool {
        matches!(self, Value::Number(_))
    }

    /// Returns true if the value is a string.
    #[must_use]
    pub fn is_string(&self) -> bool {
        matches!(self, Value::String(_))
    }

    /// Returns true if the value is a sequence.
    #[must_use]
    pub fn is_sequence(&self) -> bool {
        matches!(self, Value::Sequence(_))
    }

    /// Returns true if the value is a mapping.
    #[must_use]
    pub fn is_mapping(&self) -> bool {
        matches!(self, Value::Mapping(_))
    }

    /// Returns true if the value is tagged.
    #[must_use]
    pub fn is_tagged(&self) -> bool {
        matches!(self, Value::Tagged(_))
    }

    /// Returns the value as a boolean if it is one.
    #[must_use]
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// Returns `Some(())` if the value is null, `None` otherwise.
    #[must_use]
    pub fn as_null(&self) -> Option<()> {
        match self {
            Value::Null => Some(()),
            _ => None,
        }
    }

    /// Returns the value as an i64 if it is an integer.
    #[must_use]
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Value::Number(n) => n.as_i64(),
            _ => None,
        }
    }

    /// Returns the value as a u64 if it is a non-negative integer.
    #[must_use]
    pub fn as_u64(&self) -> Option<u64> {
        match self {
            Value::Number(n) => n.as_u64(),
            _ => None,
        }
    }

    /// Returns the value as an f64 if it is a number.
    #[must_use]
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Value::Number(n) => Some(n.as_f64()),
            _ => None,
        }
    }

    /// Returns true if the value is an integer that fits in i64.
    #[must_use]
    pub fn is_i64(&self) -> bool {
        match self {
            Value::Number(n) => n.is_i64(),
            _ => false,
        }
    }

    /// Returns true if the value is a non-negative integer that fits in u64.
    #[must_use]
    pub fn is_u64(&self) -> bool {
        match self {
            Value::Number(n) => n.is_u64(),
            _ => false,
        }
    }

    /// Returns true if the value is a number (always convertible to f64).
    #[must_use]
    pub fn is_f64(&self) -> bool {
        matches!(self, Value::Number(_))
    }

    /// Returns the value as a string slice if it is a string.
    #[must_use]
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::String(s) => Some(s),
            _ => None,
        }
    }

    /// Returns the value as a sequence if it is one.
    #[must_use]
    pub fn as_sequence(&self) -> Option<&Sequence> {
        match self {
            Value::Sequence(s) => Some(s),
            _ => None,
        }
    }

    /// Returns the value as a mutable sequence if it is one.
    #[must_use]
    pub fn as_sequence_mut(&mut self) -> Option<&mut Sequence> {
        match self {
            Value::Sequence(s) => Some(s),
            _ => None,
        }
    }

    /// Returns the value as a mapping if it is one.
    #[must_use]
    pub fn as_mapping(&self) -> Option<&Mapping> {
        match self {
            Value::Mapping(m) => Some(m),
            _ => None,
        }
    }

    /// Returns the value as a mutable mapping if it is one.
    #[must_use]
    pub fn as_mapping_mut(&mut self) -> Option<&mut Mapping> {
        match self {
            Value::Mapping(m) => Some(m),
            _ => None,
        }
    }

    /// Returns the value as a tagged value if it is one.
    #[must_use]
    pub fn as_tagged(&self) -> Option<&TaggedValue> {
        match self {
            Value::Tagged(t) => Some(t),
            _ => None,
        }
    }

    /// Returns the value as a mutable tagged value if it is one.
    #[must_use]
    pub fn as_tagged_mut(&mut self) -> Option<&mut TaggedValue> {
        match self {
            Value::Tagged(t) => Some(t),
            _ => None,
        }
    }

    /// Index into a sequence or mapping.
    #[must_use]
    pub fn get<I: ValueIndex>(&self, index: I) -> Option<&Value> {
        index.index_into(self)
    }

    /// Mutably index into a sequence or mapping.
    #[must_use]
    pub fn get_mut<I: ValueIndex>(&mut self, index: I) -> Option<&mut Value> {
        index.index_into_mut(self)
    }

    /// Access a nested value using a path string.
    ///
    /// Supports dot notation for mappings and bracket notation for sequences:
    /// - `"foo.bar"` - access key "bar" in mapping "foo"
    /// - `"items[0]"` - access index 0 in sequence "items"
    /// - `"items[0].name"` - access key "name" in first element of sequence
    ///   "items"
    ///
    /// # Example
    ///
    /// ```rust
    /// use noyalib::{from_str, Value};
    ///
    /// let yaml = r#"
    /// server:
    ///   host: localhost
    ///   port: 8080
    /// items:
    ///   - name: first
    ///   - name: second
    /// "#;
    ///
    /// let value: Value = from_str(yaml).unwrap();
    ///
    /// assert_eq!(
    ///     value.get_path("server.host").unwrap().as_str(),
    ///     Some("localhost")
    /// );
    /// assert_eq!(value.get_path("server.port").unwrap().as_i64(), Some(8080));
    /// assert_eq!(
    ///     value.get_path("items[0].name").unwrap().as_str(),
    ///     Some("first")
    /// );
    /// assert_eq!(
    ///     value.get_path("items[1].name").unwrap().as_str(),
    ///     Some("second")
    /// );
    /// ```
    #[must_use]
    pub fn get_path(&self, path: &str) -> Option<&Value> {
        let segments = parse_path(path);
        let mut current = self;

        for segment in segments {
            current = match segment {
                PathSegment::Key(key) => current.get(key.as_str())?,
                PathSegment::Index(idx) => current.get(idx)?,
            };
        }

        Some(current)
    }

    /// Mutably access a nested value using a path string.
    ///
    /// See [`get_path`](Self::get_path) for path syntax documentation.
    ///
    /// # Example
    ///
    /// ```rust
    /// use noyalib::{from_str, Value};
    ///
    /// let yaml = "server:\n  port: 8080\n";
    /// let mut value: Value = from_str(yaml).unwrap();
    ///
    /// if let Some(port) = value.get_path_mut("server.port") {
    ///     *port = Value::from(9090);
    /// }
    ///
    /// assert_eq!(value.get_path("server.port").unwrap().as_i64(), Some(9090));
    /// ```
    #[must_use]
    pub fn get_path_mut(&mut self, path: &str) -> Option<&mut Value> {
        let segments = parse_path(path);
        let mut current = self;

        for segment in segments {
            current = match segment {
                PathSegment::Key(key) => current.get_mut(key.as_str())?,
                PathSegment::Index(idx) => current.get_mut(idx)?,
            };
        }

        Some(current)
    }

    /// Deep merge another value into this one.
    ///
    /// Merge behavior:
    /// - Mappings: keys from `other` are merged recursively; `other` keys
    ///   override `self` keys
    /// - Sequences: `other` sequence replaces `self` sequence (use
    ///   `merge_concat` for concatenation)
    /// - Scalars: `other` value replaces `self` value
    /// - Null in `other`: replaces `self` value
    ///
    /// # Example
    ///
    /// ```rust
    /// use noyalib::{from_str, Value};
    ///
    /// let mut base: Value = from_str(
    ///     "
    /// server:
    ///   host: localhost
    ///   port: 8080
    /// ",
    /// )
    /// .unwrap();
    ///
    /// let override_val: Value = from_str(
    ///     "
    /// server:
    ///   port: 9090
    ///   ssl: true
    /// ",
    /// )
    /// .unwrap();
    ///
    /// base.merge(override_val);
    ///
    /// assert_eq!(
    ///     base.get_path("server.host").unwrap().as_str(),
    ///     Some("localhost")
    /// );
    /// assert_eq!(base.get_path("server.port").unwrap().as_i64(), Some(9090));
    /// assert_eq!(base.get_path("server.ssl").unwrap().as_bool(), Some(true));
    /// ```
    pub fn merge(&mut self, other: Value) {
        match (self, other) {
            (Value::Mapping(base), Value::Mapping(other)) => {
                for (key, other_value) in other {
                    match base.get_mut(&key) {
                        Some(base_value) => {
                            base_value.merge(other_value);
                        }
                        None => {
                            let _ = base.insert(key, other_value);
                        }
                    }
                }
            }
            (this, other) => {
                *this = other;
            }
        }
    }

    /// Deep merge with sequence concatenation.
    ///
    /// Similar to [`merge`](Self::merge), but sequences are concatenated
    /// instead of replaced.
    ///
    /// # Example
    ///
    /// ```rust
    /// use noyalib::{from_str, Value};
    ///
    /// let mut base: Value = from_str("items:\n  - a\n  - b\n").unwrap();
    /// let other: Value = from_str("items:\n  - c\n  - d\n").unwrap();
    ///
    /// base.merge_concat(other);
    ///
    /// let items = base.get("items").unwrap().as_sequence().unwrap();
    /// assert_eq!(items.len(), 4);
    /// ```
    pub fn merge_concat(&mut self, other: Value) {
        match (self, other) {
            (Value::Mapping(base), Value::Mapping(other)) => {
                for (key, other_value) in other {
                    match base.get_mut(&key) {
                        Some(base_value) => {
                            base_value.merge_concat(other_value);
                        }
                        None => {
                            let _ = base.insert(key, other_value);
                        }
                    }
                }
            }
            (Value::Sequence(base), Value::Sequence(other)) => {
                base.extend(other);
            }
            (this, other) => {
                *this = other;
            }
        }
    }

    /// Remove a key from a mapping.
    ///
    /// Returns the removed value if the key existed and this is a mapping.
    ///
    /// # Example
    ///
    /// ```rust
    /// use noyalib::{from_str, Value};
    ///
    /// let mut value: Value = from_str("a: 1\nb: 2\n").unwrap();
    /// let removed = value.remove("a");
    ///
    /// assert_eq!(removed.unwrap().as_i64(), Some(1));
    /// assert!(value.get("a").is_none());
    /// ```
    pub fn remove(&mut self, key: &str) -> Option<Value> {
        match self {
            Value::Mapping(map) => map.shift_remove(key),
            _ => None,
        }
    }

    /// Insert a key-value pair into a mapping.
    ///
    /// Returns the previous value if the key existed. Returns `None` if this is
    /// not a mapping.
    ///
    /// # Example
    ///
    /// ```rust
    /// use noyalib::{from_str, Value};
    ///
    /// let mut value: Value = from_str("a: 1\n").unwrap();
    /// value.insert("b", Value::from(2));
    ///
    /// assert_eq!(value.get("b").unwrap().as_i64(), Some(2));
    /// ```
    pub fn insert(&mut self, key: impl Into<String>, value: Value) -> Option<Value> {
        match self {
            Value::Mapping(map) => map.insert(key.into(), value),
            _ => None,
        }
    }

    /// Performs merging of `<<` keys into the surrounding mapping.
    ///
    /// This implements YAML's merge key functionality as described in
    /// <https://yaml.org/type/merge.html>.
    ///
    /// The merge key `<<` is used to indicate that all the keys of one or more
    /// specified mappings should be inserted into the current mapping. If a key
    /// already exists in the current mapping, its value is NOT overridden.
    ///
    /// # Example
    ///
    /// ```rust
    /// use noyalib::{from_str, Value};
    ///
    /// let config = r#"
    /// defaults: &defaults
    ///   timeout: 30
    ///   retries: 3
    ///
    /// server:
    ///   <<: *defaults
    ///   host: localhost
    ///   timeout: 60
    /// "#;
    ///
    /// let mut value: Value = from_str(config).unwrap();
    /// value.apply_merge().unwrap();
    ///
    /// // The server mapping now has merged values from defaults
    /// assert_eq!(value["server"]["host"].as_str(), Some("localhost"));
    /// assert_eq!(value["server"]["timeout"].as_i64(), Some(60)); // Not overridden
    /// assert_eq!(value["server"]["retries"].as_i64(), Some(3));  // Merged from defaults
    /// ```
    ///
    /// # Multiple Merge Sources
    ///
    /// When `<<` is followed by a sequence of mappings, they are merged in
    /// order. Earlier mappings in the sequence take precedence for
    /// duplicate keys.
    ///
    /// ```rust
    /// use noyalib::{from_str, Value};
    ///
    /// let yaml = r#"
    /// a: &a
    ///   x: 1
    /// b: &b
    ///   x: 2
    ///   y: 2
    /// merged:
    ///   <<: [*a, *b]
    ///   z: 3
    /// "#;
    ///
    /// let mut value: Value = from_str(yaml).unwrap();
    /// value.apply_merge().unwrap();
    ///
    /// assert_eq!(value["merged"]["x"].as_i64(), Some(1)); // From *a (first)
    /// assert_eq!(value["merged"]["y"].as_i64(), Some(2)); // From *b
    /// assert_eq!(value["merged"]["z"].as_i64(), Some(3)); // Direct value
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - A merge key value is a scalar (not a mapping or sequence of mappings)
    /// - A merge key value is a tagged value
    /// - A sequence in a merge key contains non-mapping values
    pub fn apply_merge(&mut self) -> crate::Result<()> {
        match self {
            Value::Mapping(mapping) => {
                // First, recursively apply merge to all values
                for value in mapping.values_mut() {
                    value.apply_merge()?;
                }

                // Then process the << key if present
                let merge_value = mapping.remove("<<");
                let merge_sequence = match merge_value {
                    Some(Value::Sequence(seq)) => seq,
                    Some(value) => vec![value],
                    None => vec![],
                };

                // Process each merge source
                for value in merge_sequence {
                    match value {
                        Value::Mapping(merge_map) => {
                            // Merge keys from source, but don't override existing keys
                            for (k, v) in merge_map {
                                let _ = mapping.entry(k).or_insert(v);
                            }
                        }
                        Value::Sequence(_) => {
                            return Err(crate::Error::SequenceInMergeElement);
                        }
                        Value::Tagged(_) => {
                            return Err(crate::Error::TaggedInMerge);
                        }
                        _ => {
                            return Err(crate::Error::ScalarInMergeElement);
                        }
                    }
                }
            }
            Value::Sequence(seq) => {
                // Recursively apply merge to sequence elements
                for value in seq {
                    value.apply_merge()?;
                }
            }
            Value::Tagged(tagged) => {
                // Recursively apply merge to tagged value
                tagged.value_mut().apply_merge()?;
            }
            // Scalars don't need merge processing
            _ => {}
        }

        Ok(())
    }

    /// Recursively strips tags from this value, returning the untagged value.
    ///
    /// If the value is `Value::Tagged`, the inner value is returned
    /// (recursively untagged). Sequences and mappings have their elements
    /// recursively untagged.
    #[must_use]
    pub fn untag(self) -> Self {
        match self {
            Value::Tagged(tagged) => tagged.value.untag(),
            Value::Sequence(seq) => Value::Sequence(seq.into_iter().map(Value::untag).collect()),
            Value::Mapping(map) => {
                let untagged: Mapping = map.into_iter().map(|(k, v)| (k, v.untag())).collect();
                Value::Mapping(untagged)
            }
            other => other,
        }
    }

    /// Returns a reference to the innermost untagged value.
    ///
    /// If the value is `Value::Tagged`, returns a reference to the inner value
    /// (recursively following tags). Does not recurse into sequences or
    /// mappings.
    #[must_use]
    pub fn untag_ref(&self) -> &Self {
        match self {
            Value::Tagged(tagged) => tagged.value.untag_ref(),
            other => other,
        }
    }

    /// Returns a mutable reference to the innermost untagged value.
    ///
    /// If the value is `Value::Tagged`, returns a mutable reference to the
    /// inner value (recursively following tags). Does not recurse into
    /// sequences or mappings.
    #[must_use]
    pub fn untag_mut(&mut self) -> &mut Self {
        match self {
            Value::Tagged(tagged) => tagged.value.untag_mut(),
            other => other,
        }
    }
}

/// A segment in a path expression.
#[derive(Debug, Clone)]
enum PathSegment {
    /// A key in a mapping.
    Key(String),
    /// An index in a sequence.
    Index(usize),
}

/// Parse a path string into segments.
///
/// Supports:
/// - Dot notation: "foo.bar.baz"
/// - Bracket notation: "items[0]"
/// - Mixed: "items[0].name"
fn parse_path(path: &str) -> Vec<PathSegment> {
    let mut segments = Vec::new();
    let mut current = String::new();
    let mut chars = path.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            '.' => {
                if !current.is_empty() {
                    segments.push(PathSegment::Key(std::mem::take(&mut current)));
                }
            }
            '[' => {
                if !current.is_empty() {
                    segments.push(PathSegment::Key(std::mem::take(&mut current)));
                }
                // Parse the index
                let mut index_str = String::new();
                while let Some(&c) = chars.peek() {
                    if c == ']' {
                        let _ = chars.next();
                        break;
                    }
                    index_str.push(c);
                    let _ = chars.next();
                }
                if let Ok(idx) = index_str.parse::<usize>() {
                    segments.push(PathSegment::Index(idx));
                }
            }
            ']' => {
                // Should be consumed by '[' handler, but handle gracefully
            }
            _ => {
                current.push(c);
            }
        }
    }

    if !current.is_empty() {
        segments.push(PathSegment::Key(current));
    }

    segments
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Null, Value::Null) => true,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Number(a), Value::Number(b)) => a == b,
            (Value::String(a), Value::String(b)) => a == b,
            (Value::Sequence(a), Value::Sequence(b)) => a == b,
            (Value::Mapping(a), Value::Mapping(b)) => a == b,
            (Value::Tagged(a), Value::Tagged(b)) => a == b,
            _ => false,
        }
    }
}

impl Eq for Value {}

impl Hash for Value {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // Discriminant for variant type
        std::mem::discriminant(self).hash(state);
        match self {
            Value::Null => {}
            Value::Bool(b) => b.hash(state),
            Value::Number(n) => n.hash(state),
            Value::String(s) => s.hash(state),
            Value::Sequence(seq) => {
                seq.len().hash(state);
                for v in seq {
                    v.hash(state);
                }
            }
            Value::Mapping(map) => {
                map.len().hash(state);
                for (k, v) in map {
                    k.hash(state);
                    v.hash(state);
                }
            }
            Value::Tagged(tagged) => {
                tagged.tag().hash(state);
                tagged.value().hash(state);
            }
        }
    }
}

impl PartialOrd for Value {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Value {
    fn cmp(&self, other: &Self) -> Ordering {
        // Order: Null < Bool < Number < String < Sequence < Mapping < Tagged
        fn type_order(v: &Value) -> u8 {
            match v {
                Value::Null => 0,
                Value::Bool(_) => 1,
                Value::Number(_) => 2,
                Value::String(_) => 3,
                Value::Sequence(_) => 4,
                Value::Mapping(_) => 5,
                Value::Tagged(_) => 6,
            }
        }

        match type_order(self).cmp(&type_order(other)) {
            Ordering::Equal => {}
            ord => return ord,
        }

        match (self, other) {
            (Value::Null, Value::Null) => Ordering::Equal,
            (Value::Bool(a), Value::Bool(b)) => a.cmp(b),
            (Value::Number(a), Value::Number(b)) => a.cmp(b),
            (Value::String(a), Value::String(b)) => a.cmp(b),
            (Value::Sequence(a), Value::Sequence(b)) => a.len().cmp(&b.len()).then_with(|| {
                for (av, bv) in a.iter().zip(b.iter()) {
                    match av.cmp(bv) {
                        Ordering::Equal => continue,
                        ord => return ord,
                    }
                }
                Ordering::Equal
            }),
            (Value::Mapping(a), Value::Mapping(b)) => a.len().cmp(&b.len()).then_with(|| {
                for ((ak, av), (bk, bv)) in a.iter().zip(b.iter()) {
                    match ak.cmp(bk) {
                        Ordering::Equal => {}
                        ord => return ord,
                    }
                    match av.cmp(bv) {
                        Ordering::Equal => continue,
                        ord => return ord,
                    }
                }
                Ordering::Equal
            }),
            (Value::Tagged(a), Value::Tagged(b)) => a
                .tag()
                .as_str()
                .cmp(b.tag().as_str())
                .then_with(|| a.value().cmp(b.value())),
            _ => unreachable!("type_order check ensures same variants"),
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Null => write!(f, "null"),
            Value::Bool(b) => write!(f, "{b}"),
            Value::Number(n) => write!(f, "{n}"),
            Value::String(s) => write!(f, "{s}"),
            Value::Sequence(s) => {
                write!(f, "[")?;
                for (i, v) in s.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{v}")?;
                }
                write!(f, "]")
            }
            Value::Mapping(m) => {
                write!(f, "{{")?;
                for (i, (k, v)) in m.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{k}: {v}")?;
                }
                write!(f, "}}")
            }
            Value::Tagged(t) => write!(f, "{t}"),
        }
    }
}

/// A type that can be used to index into a `Value`.
///
/// This trait provides methods for accessing elements within a [`Value`] by
/// index. It is implemented for:
/// - `usize` - for indexing into sequences
/// - `&str` - for indexing into mappings by string key
/// - `String` - for indexing into mappings by owned string
/// - `&String` - for indexing into mappings by string reference
///
/// # Example
///
/// ```rust
/// use noyalib::{from_str, Value, ValueIndex};
///
/// let yaml = r#"
/// items:
///   - name: first
///   - name: second
/// config:
///   host: localhost
/// "#;
///
/// let value: Value = from_str(yaml).unwrap();
///
/// // Using usize to index into sequences
/// assert_eq!(
///     value
///         .get("items")
///         .unwrap()
///         .get(0)
///         .unwrap()
///         .get("name")
///         .unwrap()
///         .as_str(),
///     Some("first")
/// );
///
/// // Using &str to index into mappings
/// assert_eq!(
///     value.get("config").unwrap().get("host").unwrap().as_str(),
///     Some("localhost")
/// );
/// ```
pub trait ValueIndex {
    /// Index into a value, returning a reference to the element if found.
    ///
    /// Returns `None` if:
    /// - The value is not the appropriate type for this index (e.g., indexing a
    ///   mapping with `usize`)
    /// - The index/key doesn't exist
    fn index_into(self, value: &Value) -> Option<&Value>;

    /// Mutably index into a value, returning a mutable reference to the element
    /// if found.
    ///
    /// Returns `None` if:
    /// - The value is not the appropriate type for this index
    /// - The index/key doesn't exist
    fn index_into_mut(self, value: &mut Value) -> Option<&mut Value>;

    /// Index into a value, inserting a default value if the key doesn't exist.
    ///
    /// This method is useful for building nested structures or ensuring a key
    /// exists.
    ///
    /// # Behavior
    ///
    /// - For sequences: panics if the index is out of bounds
    /// - For mappings: creates a null entry if the key doesn't exist
    /// - For null values: converts to an empty mapping (for string keys only)
    ///
    /// # Panics
    ///
    /// Panics if:
    /// - The value is not the appropriate type for this index
    /// - Indexing a sequence with an out-of-bounds index
    fn index_or_insert(self, value: &mut Value) -> &mut Value;
}

impl ValueIndex for usize {
    fn index_into(self, value: &Value) -> Option<&Value> {
        match value {
            Value::Sequence(s) => s.get(self),
            Value::Tagged(t) => self.index_into(t.value()),
            _ => None,
        }
    }

    fn index_into_mut(self, value: &mut Value) -> Option<&mut Value> {
        match value {
            Value::Sequence(s) => s.get_mut(self),
            Value::Tagged(t) => self.index_into_mut(t.value_mut()),
            _ => None,
        }
    }

    fn index_or_insert(self, value: &mut Value) -> &mut Value {
        match value {
            Value::Sequence(s) => {
                let len = s.len();
                s.get_mut(self).unwrap_or_else(|| {
                    panic!(
                        "cannot access index {} of YAML sequence of length {}",
                        self, len
                    )
                })
            }
            Value::Tagged(t) => self.index_or_insert(t.value_mut()),
            _ => panic!(
                "cannot access index {} of YAML {}",
                self,
                value_type_name(value)
            ),
        }
    }
}

impl ValueIndex for &str {
    fn index_into(self, value: &Value) -> Option<&Value> {
        match value {
            Value::Mapping(m) => m.get(self),
            Value::Tagged(t) => self.index_into(t.value()),
            _ => None,
        }
    }

    fn index_into_mut(self, value: &mut Value) -> Option<&mut Value> {
        match value {
            Value::Mapping(m) => m.get_mut(self),
            Value::Tagged(t) => self.index_into_mut(t.value_mut()),
            _ => None,
        }
    }

    fn index_or_insert(self, value: &mut Value) -> &mut Value {
        // If the value is null, convert it to an empty mapping
        if let Value::Null = value {
            *value = Value::Mapping(Mapping::new());
        }

        match value {
            Value::Mapping(m) => {
                let _ = m.entry(self.to_owned()).or_insert(Value::Null);
                m.get_mut(self).unwrap()
            }
            Value::Tagged(t) => self.index_or_insert(t.value_mut()),
            _ => panic!(
                "cannot access key {:?} in YAML {}",
                self,
                value_type_name(value)
            ),
        }
    }
}

impl ValueIndex for String {
    fn index_into(self, value: &Value) -> Option<&Value> {
        self.as_str().index_into(value)
    }

    fn index_into_mut(self, value: &mut Value) -> Option<&mut Value> {
        self.as_str().index_into_mut(value)
    }

    fn index_or_insert(self, value: &mut Value) -> &mut Value {
        self.as_str().index_or_insert(value)
    }
}

impl ValueIndex for &String {
    fn index_into(self, value: &Value) -> Option<&Value> {
        self.as_str().index_into(value)
    }

    fn index_into_mut(self, value: &mut Value) -> Option<&mut Value> {
        self.as_str().index_into_mut(value)
    }

    fn index_or_insert(self, value: &mut Value) -> &mut Value {
        self.as_str().index_or_insert(value)
    }
}

impl ValueIndex for &Value {
    fn index_into(self, value: &Value) -> Option<&Value> {
        match self {
            Value::String(s) => s.as_str().index_into(value),
            Value::Number(Number::Integer(n)) if *n >= 0 => {
                usize::try_from(*n).ok()?.index_into(value)
            }
            _ => None,
        }
    }

    fn index_into_mut(self, value: &mut Value) -> Option<&mut Value> {
        match self {
            Value::String(s) => s.as_str().index_into_mut(value),
            Value::Number(Number::Integer(n)) if *n >= 0 => {
                usize::try_from(*n).ok()?.index_into_mut(value)
            }
            _ => None,
        }
    }

    fn index_or_insert(self, value: &mut Value) -> &mut Value {
        match self {
            Value::String(s) => s.as_str().index_or_insert(value),
            Value::Number(Number::Integer(n)) if *n >= 0 => {
                let idx =
                    usize::try_from(*n).unwrap_or_else(|_| panic!("index {} overflows usize", n));
                idx.index_or_insert(value)
            }
            _ => panic!("cannot index with {:?}", self),
        }
    }
}

/// Returns the type name of a value for error messages.
fn value_type_name(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Sequence(_) => "sequence",
        Value::Mapping(_) => "mapping",
        Value::Tagged(_) => "tagged value",
    }
}

impl<'de> Deserialize<'de> for Value {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::{MapAccess, SeqAccess, Visitor};

        struct ValueVisitor;

        impl<'de> Visitor<'de> for ValueVisitor {
            type Value = Value;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("any valid YAML value")
            }

            fn visit_bool<E>(self, v: bool) -> Result<Value, E> {
                Ok(Value::Bool(v))
            }

            fn visit_i64<E>(self, v: i64) -> Result<Value, E> {
                Ok(Value::Number(Number::Integer(v)))
            }

            fn visit_u64<E>(self, v: u64) -> Result<Value, E> {
                Ok(Value::Number(Number::Integer(v as i64)))
            }

            fn visit_f64<E>(self, v: f64) -> Result<Value, E> {
                Ok(Value::Number(Number::Float(v)))
            }

            fn visit_str<E>(self, v: &str) -> Result<Value, E> {
                Ok(Value::String(v.to_owned()))
            }

            fn visit_string<E>(self, v: String) -> Result<Value, E> {
                Ok(Value::String(v))
            }

            fn visit_none<E>(self) -> Result<Value, E> {
                Ok(Value::Null)
            }

            fn visit_unit<E>(self) -> Result<Value, E> {
                Ok(Value::Null)
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                let mut vec = Vec::new();
                while let Some(elem) = seq.next_element()? {
                    vec.push(elem);
                }
                Ok(Value::Sequence(vec))
            }

            fn visit_map<A>(self, mut map: A) -> Result<Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                let mut mapping = Mapping::new();
                while let Some((key, value)) = map.next_entry::<String, Value>()? {
                    let _ = mapping.insert(key, value);
                }
                Ok(Value::Mapping(mapping))
            }
        }

        deserializer.deserialize_any(ValueVisitor)
    }
}

impl Serialize for Value {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Value::Null => serializer.serialize_none(),
            Value::Bool(b) => serializer.serialize_bool(*b),
            Value::Number(Number::Integer(n)) => serializer.serialize_i64(*n),
            Value::Number(Number::Float(n)) => serializer.serialize_f64(*n),
            Value::String(s) => serializer.serialize_str(s),
            Value::Sequence(s) => s.serialize(serializer),
            Value::Mapping(m) => {
                use serde::ser::SerializeMap;
                let mut map = serializer.serialize_map(Some(m.len()))?;
                for (k, v) in m {
                    map.serialize_entry(k, v)?;
                }
                map.end()
            }
            Value::Tagged(tagged) => {
                // Serialize as a single-entry map with tag as key
                use serde::ser::SerializeMap;
                let mut map = serializer.serialize_map(Some(1))?;
                map.serialize_entry(tagged.tag().as_str(), tagged.value())?;
                map.end()
            }
        }
    }
}

// ============================================================================
// Deserializer implementation for &Value
// ============================================================================

impl<'de> serde::de::IntoDeserializer<'de, crate::Error> for &'de Value {
    type Deserializer = Self;

    fn into_deserializer(self) -> Self::Deserializer {
        self
    }
}

struct ValueSeqAccess<'de> {
    iter: std::slice::Iter<'de, Value>,
}

impl<'de> serde::de::SeqAccess<'de> for ValueSeqAccess<'de> {
    type Error = crate::Error;

    fn next_element_seed<T>(&mut self, seed: T) -> crate::Result<Option<T::Value>>
    where
        T: serde::de::DeserializeSeed<'de>,
    {
        match self.iter.next() {
            Some(value) => seed.deserialize(value).map(Some),
            None => Ok(None),
        }
    }
}

struct ValueMapAccess<'de> {
    iter: Iter<'de, String, Value>,
    value: Option<&'de Value>,
}

impl<'de> serde::de::MapAccess<'de> for ValueMapAccess<'de> {
    type Error = crate::Error;

    fn next_key_seed<K>(&mut self, seed: K) -> crate::Result<Option<K::Value>>
    where
        K: serde::de::DeserializeSeed<'de>,
    {
        match self.iter.next() {
            Some((key, value)) => {
                self.value = Some(value);
                seed.deserialize(serde::de::value::BorrowedStrDeserializer::new(key))
                    .map(Some)
            }
            None => Ok(None),
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> crate::Result<V::Value>
    where
        V: serde::de::DeserializeSeed<'de>,
    {
        match self.value.take() {
            Some(value) => seed.deserialize(value),
            None => Err(serde::de::Error::custom("value is missing")),
        }
    }
}

impl<'de> serde::Deserializer<'de> for &'de Value {
    type Error = crate::Error;

    fn deserialize_any<V>(self, visitor: V) -> crate::Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        match self {
            Value::Null => visitor.visit_unit(),
            Value::Bool(b) => visitor.visit_bool(*b),
            Value::Number(Number::Integer(n)) => visitor.visit_i64(*n),
            Value::Number(Number::Float(n)) => visitor.visit_f64(*n),
            Value::String(s) => visitor.visit_borrowed_str(s),
            Value::Sequence(seq) => visitor.visit_seq(ValueSeqAccess { iter: seq.iter() }),
            Value::Mapping(map) => visitor.visit_map(ValueMapAccess {
                iter: map.iter(),
                value: None,
            }),
            Value::Tagged(tagged) => {
                let tagged_ref: &'de TaggedValue = tagged;
                serde::Deserializer::deserialize_any(tagged_ref, visitor)
            }
        }
    }

    fn deserialize_enum<V>(
        self,
        name: &'static str,
        variants: &'static [&'static str],
        visitor: V,
    ) -> crate::Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        match self {
            Value::Tagged(tagged) => {
                let tagged_ref: &'de TaggedValue = tagged;
                serde::Deserializer::deserialize_enum(tagged_ref, name, variants, visitor)
            }
            Value::String(s) => visitor
                .visit_enum(serde::de::value::BorrowedStrDeserializer::<crate::Error>::new(s)),
            _ => serde::Deserializer::deserialize_any(self, visitor),
        }
    }

    fn deserialize_seq<V>(self, visitor: V) -> crate::Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        match self {
            Value::Sequence(seq) => visitor.visit_seq(ValueSeqAccess { iter: seq.iter() }),
            _ => serde::Deserializer::deserialize_any(self, visitor),
        }
    }

    fn deserialize_map<V>(self, visitor: V) -> crate::Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        match self {
            Value::Mapping(map) => visitor.visit_map(ValueMapAccess {
                iter: map.iter(),
                value: None,
            }),
            _ => serde::Deserializer::deserialize_any(self, visitor),
        }
    }

    fn deserialize_struct<V>(
        self,
        name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> crate::Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        if name == crate::spanned::SPANNED_TYPE_NAME {
            let p: *const Value = self;
            let ptr = p as usize;
            let (start, end) = crate::span_context::lookup_span(ptr)
                .unwrap_or((crate::Location::default(), crate::Location::default()));
            return visitor.visit_map(crate::de::SpannedMapAccess::new(start, end, self));
        }
        serde::Deserializer::deserialize_map(self, visitor)
    }

    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 u8 u16 u32 u64 f32 f64 char str string bytes
        byte_buf option unit unit_struct newtype_struct tuple
        tuple_struct identifier ignored_any
    }
}

// ============================================================================
// From implementations for Value
// ============================================================================

impl From<()> for Value {
    fn from(_: ()) -> Self {
        Value::Null
    }
}

impl From<bool> for Value {
    fn from(v: bool) -> Self {
        Value::Bool(v)
    }
}

impl From<i8> for Value {
    fn from(v: i8) -> Self {
        Value::Number(Number::Integer(i64::from(v)))
    }
}

impl From<i16> for Value {
    fn from(v: i16) -> Self {
        Value::Number(Number::Integer(i64::from(v)))
    }
}

impl From<i32> for Value {
    fn from(v: i32) -> Self {
        Value::Number(Number::Integer(i64::from(v)))
    }
}

impl From<i64> for Value {
    fn from(v: i64) -> Self {
        Value::Number(Number::Integer(v))
    }
}

impl From<u8> for Value {
    fn from(v: u8) -> Self {
        Value::Number(Number::Integer(i64::from(v)))
    }
}

impl From<u16> for Value {
    fn from(v: u16) -> Self {
        Value::Number(Number::Integer(i64::from(v)))
    }
}

impl From<u32> for Value {
    fn from(v: u32) -> Self {
        Value::Number(Number::Integer(i64::from(v)))
    }
}

impl From<f32> for Value {
    fn from(v: f32) -> Self {
        Value::Number(Number::Float(f64::from(v)))
    }
}

impl From<f64> for Value {
    fn from(v: f64) -> Self {
        Value::Number(Number::Float(v))
    }
}

impl From<String> for Value {
    fn from(v: String) -> Self {
        Value::String(v)
    }
}

impl From<&str> for Value {
    fn from(v: &str) -> Self {
        Value::String(v.to_owned())
    }
}

impl<T: Into<Value>> From<Vec<T>> for Value {
    fn from(v: Vec<T>) -> Self {
        Value::Sequence(v.into_iter().map(Into::into).collect())
    }
}

impl<T: Into<Value>> From<Option<T>> for Value {
    fn from(v: Option<T>) -> Self {
        match v {
            Some(v) => v.into(),
            None => Value::Null,
        }
    }
}

impl From<Number> for Value {
    fn from(v: Number) -> Self {
        Value::Number(v)
    }
}

impl From<Mapping> for Value {
    fn from(v: Mapping) -> Self {
        Value::Mapping(v)
    }
}

// Note: From<Sequence> is covered by From<Vec<T>> since Sequence = Vec<Value>

impl From<TaggedValue> for Value {
    fn from(v: TaggedValue) -> Self {
        Value::Tagged(Box::new(v))
    }
}

impl From<u64> for Value {
    fn from(v: u64) -> Self {
        Value::Number(Number::from(v))
    }
}

impl From<isize> for Value {
    fn from(v: isize) -> Self {
        Value::Number(Number::from(v))
    }
}

impl From<usize> for Value {
    fn from(v: usize) -> Self {
        Value::Number(Number::from(v))
    }
}

impl<'a> From<Cow<'a, str>> for Value {
    fn from(v: Cow<'a, str>) -> Self {
        Value::String(v.into_owned())
    }
}

impl<T: Clone + Into<Value>> From<&[T]> for Value {
    fn from(v: &[T]) -> Self {
        Value::Sequence(v.iter().cloned().map(Into::into).collect())
    }
}

impl<T: Into<Value>> FromIterator<T> for Value {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        Value::Sequence(iter.into_iter().map(Into::into).collect())
    }
}

// ============================================================================
// Index trait implementations for Value
// ============================================================================

impl Index<usize> for Value {
    type Output = Value;

    /// Index into a YAML sequence.
    ///
    /// # Panics
    ///
    /// Panics if the value is not a sequence or if the index is out of bounds.
    ///
    /// # Example
    ///
    /// ```rust
    /// use noyalib::{from_str, Value};
    ///
    /// let yaml = "- a\n- b\n- c\n";
    /// let value: Value = from_str(yaml).unwrap();
    /// assert_eq!(value[0].as_str(), Some("a"));
    /// assert_eq!(value[1].as_str(), Some("b"));
    /// ```
    fn index(&self, index: usize) -> &Self::Output {
        self.get(index)
            .expect("index out of bounds or not a sequence")
    }
}

impl IndexMut<usize> for Value {
    /// Mutably index into a YAML sequence.
    ///
    /// # Panics
    ///
    /// Panics if the value is not a sequence or if the index is out of bounds.
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        self.get_mut(index)
            .expect("index out of bounds or not a sequence")
    }
}

impl Index<&str> for Value {
    type Output = Value;

    /// Index into a YAML mapping by key.
    ///
    /// # Panics
    ///
    /// Panics if the value is not a mapping or if the key is not found.
    ///
    /// # Example
    ///
    /// ```rust
    /// use noyalib::{from_str, Value};
    ///
    /// let yaml = "name: test\nversion: 1\n";
    /// let value: Value = from_str(yaml).unwrap();
    /// assert_eq!(value["name"].as_str(), Some("test"));
    /// assert_eq!(value["version"].as_i64(), Some(1));
    /// ```
    fn index(&self, key: &str) -> &Self::Output {
        self.get(key).expect("key not found or not a mapping")
    }
}

impl IndexMut<&str> for Value {
    /// Mutably index into a YAML mapping by key.
    ///
    /// # Panics
    ///
    /// Panics if the value is not a mapping or if the key is not found.
    fn index_mut(&mut self, key: &str) -> &mut Self::Output {
        self.get_mut(key).expect("key not found or not a mapping")
    }
}
