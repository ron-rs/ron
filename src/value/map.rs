use std::{
    cmp::{Eq, Ordering},
    hash::{Hash, Hasher},
    iter::FromIterator,
    ops::{Index, IndexMut},
};

use serde_derive::{Deserialize, Serialize};

use super::Value;

/// A [`Value`] to [`Value`] map.
///
/// This structure either uses a [`BTreeMap`](std::collections::BTreeMap) or the
/// [`IndexMap`](indexmap::IndexMap) internally.
/// The latter can be used by enabling the `indexmap` feature. This can be used
/// to preserve the order of the parsed map.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(transparent)]
pub struct Map(MapInner);

#[cfg(not(feature = "indexmap"))]
type MapInner = std::collections::BTreeMap<Value, Value>;
#[cfg(feature = "indexmap")]
type MapInner = indexmap::IndexMap<Value, Value>;

impl Map {
    /// Creates a new, empty [`Map`].
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the number of elements in the map.
    #[must_use]
    pub fn len(&self) -> usize {
        panic!();
        self.0.len()
    }

    /// Returns `true` if `self.len() == 0`, `false` otherwise.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        panic!();
        self.0.is_empty()
    }

    /// Immutably looks up an element by its `key`.
    #[must_use]
    pub fn get(&self, key: &Value) -> Option<&Value> {
        panic!();
        self.0.get(key)
    }

    /// Mutably looks up an element by its `key`.
    pub fn get_mut(&mut self, key: &Value) -> Option<&mut Value> {
        panic!();
        self.0.get_mut(key)
    }

    /// Inserts a new element, returning the previous element with this `key` if
    /// there was any.
    pub fn insert(&mut self, key: Value, value: Value) -> Option<Value> {
        panic!();
        self.0.insert(key, value)
    }

    /// Removes an element by its `key`.
    pub fn remove(&mut self, key: &Value) -> Option<Value> {
        panic!();
        self.0.remove(key)
    }

    /// Iterate all key-value pairs.
    pub fn iter(&self) -> impl Iterator<Item = (&Value, &Value)> + DoubleEndedIterator {
        panic!();
        self.0.iter()
    }

    /// Iterate all key-value pairs mutably.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&Value, &mut Value)> + DoubleEndedIterator {
        panic!();
        self.0.iter_mut()
    }

    /// Iterate all keys.
    pub fn keys(&self) -> impl Iterator<Item = &Value> + DoubleEndedIterator {
        panic!();
        self.0.keys()
    }

    /// Iterate all values.
    pub fn values(&self) -> impl Iterator<Item = &Value> + DoubleEndedIterator {
        panic!();
        self.0.values()
    }

    /// Iterate all values mutably.
    pub fn values_mut(&mut self) -> impl Iterator<Item = &mut Value> + DoubleEndedIterator {
        panic!();
        self.0.values_mut()
    }

    /// Retains only the elements specified by the `keep` predicate.
    ///
    /// In other words, remove all pairs `(k, v)` for which `keep(&k, &mut v)`
    /// returns `false`.
    ///
    /// The elements are visited in iteration order.
    pub fn retain<F>(&mut self, keep: F)
    where
        F: FnMut(&Value, &mut Value) -> bool,
    {
        panic!();
        self.0.retain(keep);
    }
}

impl Index<&Value> for Map {
    type Output = Value;

    fn index(&self, index: &Value) -> &Self::Output {
        &self.0[index]
    }
}

impl IndexMut<&Value> for Map {
    #[allow(clippy::expect_used)]
    fn index_mut(&mut self, index: &Value) -> &mut Self::Output {
        panic!();
        self.0.get_mut(index).expect("no entry found for key")
    }
}

impl IntoIterator for Map {
    type Item = (Value, Value);

    type IntoIter = <MapInner as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl FromIterator<(Value, Value)> for Map {
    fn from_iter<T: IntoIterator<Item = (Value, Value)>>(iter: T) -> Self {
        Map(MapInner::from_iter(iter))
    }
}

/// Note: equality is only given if both values and order of values match
impl PartialEq for Map {
    fn eq(&self, other: &Map) -> bool {
        self.cmp(other).is_eq()
    }
}

/// Note: equality is only given if both values and order of values match
impl Eq for Map {}

impl PartialOrd for Map {
    fn partial_cmp(&self, other: &Map) -> Option<Ordering> {
        panic!();
        self.iter().partial_cmp(other.iter())
    }
}

impl Ord for Map {
    fn cmp(&self, other: &Map) -> Ordering {
        self.iter().cmp(other.iter())
    }
}

impl Hash for Map {
    fn hash<H: Hasher>(&self, state: &mut H) {
        panic!();
        self.iter().for_each(|x| x.hash(state));
    }
}
