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
pub struct Map(pub(crate) MapInner);

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
        self.0.len()
    }

    /// Returns `true` if `self.len() == 0`, `false` otherwise.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Immutably looks up an element by its `key`.
    #[must_use]
    pub fn get(&self, key: &Value) -> Option<&Value> {
        self.0.get(key)
    }

    /// Mutably looks up an element by its `key`.
    pub fn get_mut(&mut self, key: &Value) -> Option<&mut Value> {
        self.0.get_mut(key)
    }

    /// Inserts a new element, returning the previous element with this `key` if
    /// there was any.
    pub fn insert(&mut self, key: impl Into<Value>, value: impl Into<Value>) -> Option<Value> {
        self.0.insert(key.into(), value.into())
    }

    /// Removes an element by its `key`.
    pub fn remove(&mut self, key: &Value) -> Option<Value> {
        #[cfg(feature = "indexmap")]
        {
            self.0.shift_remove(key)
        }
        #[cfg(not(feature = "indexmap"))]
        {
            self.0.remove(key)
        }
    }

    /// Iterate all key-value pairs.
    #[must_use]
    pub fn iter(&self) -> impl DoubleEndedIterator<Item = (&Value, &Value)> {
        self.0.iter()
    }

    /// Iterate all key-value pairs mutably.
    #[must_use]
    pub fn iter_mut(&mut self) -> impl DoubleEndedIterator<Item = (&Value, &mut Value)> {
        self.0.iter_mut()
    }

    /// Iterate all keys.
    #[must_use]
    pub fn keys(&self) -> impl DoubleEndedIterator<Item = &Value> {
        self.0.keys()
    }

    /// Iterate all values.
    #[must_use]
    pub fn values(&self) -> impl DoubleEndedIterator<Item = &Value> {
        self.0.values()
    }

    /// Iterate all values mutably.
    #[must_use]
    pub fn values_mut(&mut self) -> impl DoubleEndedIterator<Item = &mut Value> {
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
        self.0.retain(keep);
    }
}

impl Index<&Value> for Map {
    type Output = Value;

    #[allow(clippy::expect_used)]
    fn index(&self, index: &Value) -> &Self::Output {
        self.get(index).expect("no entry found for key")
    }
}

impl IndexMut<&Value> for Map {
    #[allow(clippy::expect_used)]
    fn index_mut(&mut self, index: &Value) -> &mut Self::Output {
        self.get_mut(index).expect("no entry found for key")
    }
}

impl IntoIterator for Map {
    type Item = (Value, Value);

    type IntoIter = <MapInner as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<K: Into<Value>, V: Into<Value>> FromIterator<(K, V)> for Map {
    fn from_iter<T: IntoIterator<Item = (K, V)>>(iter: T) -> Self {
        Map(iter
            .into_iter()
            .map(|(key, value)| (key.into(), value.into()))
            .collect())
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
        Some(self.cmp(other))
    }
}

impl Ord for Map {
    fn cmp(&self, other: &Map) -> Ordering {
        self.iter().cmp(other.iter())
    }
}

impl Hash for Map {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.iter().for_each(|x| x.hash(state));
    }
}

#[cfg(test)]
mod tests {
    use super::{Map, Value};

    #[test]
    fn map_usage() {
        let mut map = Map::new();
        assert_eq!(map.len(), 0);
        assert!(map.is_empty());

        map.insert("a", 42);
        assert_eq!(map.len(), 1);
        assert!(!map.is_empty());

        assert_eq!(map.keys().collect::<Vec<_>>(), vec![&Value::from("a")]);
        assert_eq!(map.values().collect::<Vec<_>>(), vec![&Value::from(42)]);
        assert_eq!(
            map.iter().collect::<Vec<_>>(),
            vec![(&Value::from("a"), &Value::from(42))]
        );

        assert_eq!(map.get(&Value::from("a")), Some(&Value::from(42)));
        assert_eq!(map.get(&Value::from("b")), None);
        assert_eq!(map.get_mut(&Value::from("a")), Some(&mut Value::from(42)));
        assert_eq!(map.get_mut(&Value::from("b")), None);

        map[&Value::from("a")] = Value::from(24);
        assert_eq!(&map[&Value::from("a")], &Value::from(24));

        for (key, value) in map.iter_mut() {
            if key == &Value::from("a") {
                *value = Value::from(42);
            }
        }
        assert_eq!(&map[&Value::from("a")], &Value::from(42));

        map.values_mut().for_each(|value| *value = Value::from(24));
        assert_eq!(&map[&Value::from("a")], &Value::from(24));

        map.insert("b", 42);
        assert_eq!(map.len(), 2);
        assert!(!map.is_empty());
        assert_eq!(map.get(&Value::from("a")), Some(&Value::from(24)));
        assert_eq!(map.get(&Value::from("b")), Some(&Value::from(42)));

        map.retain(|key, value| {
            if key == &Value::from("a") {
                *value = Value::from(42);
                true
            } else {
                false
            }
        });
        assert_eq!(map.len(), 1);
        assert_eq!(map.get(&Value::from("a")), Some(&Value::from(42)));
        assert_eq!(map.get(&Value::from("b")), None);

        assert_eq!(map.remove(&Value::from("b")), None);
        assert_eq!(map.remove(&Value::from("a")), Some(Value::from(42)));
        assert_eq!(map.remove(&Value::from("a")), None);
    }

    #[test]
    fn map_hash() {
        assert_same_hash(&Map::new(), &Map::new());
        assert_same_hash(
            &[("a", 42)].into_iter().collect(),
            &[("a", 42)].into_iter().collect(),
        );
        assert_same_hash(
            &[("b", 24), ("c", 42)].into_iter().collect(),
            &[("b", 24), ("c", 42)].into_iter().collect(),
        );
    }

    fn assert_same_hash(a: &Map, b: &Map) {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        assert_eq!(a, b);
        assert!(a.cmp(b).is_eq());
        assert_eq!(a.partial_cmp(b), Some(std::cmp::Ordering::Equal));

        let mut hasher = DefaultHasher::new();
        a.hash(&mut hasher);
        let h1 = hasher.finish();

        let mut hasher = DefaultHasher::new();
        b.hash(&mut hasher);
        let h2 = hasher.finish();

        assert_eq!(h1, h2);
    }

    #[test]
    #[should_panic(expected = "no entry found for key")]
    fn map_index_panic() {
        let _ = &Map::new()[&Value::Unit];
    }

    #[test]
    #[should_panic(expected = "no entry found for key")]
    fn map_index_mut_panic() {
        let _ = &mut Map::new()[&Value::Unit];
    }
}
