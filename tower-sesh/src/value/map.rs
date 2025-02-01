// Adapted from https://github.com/serde-rs/json.

//! A map of `String` to [`Value`].

use std::{
    borrow::Borrow,
    collections::{btree_map, BTreeMap},
    fmt,
    hash::Hash,
    iter::FusedIterator,
    ops,
};

use serde::{Deserialize, Serialize};

use super::Value;

/// Represents a serializable key/value type.
pub struct Map<K, V> {
    map: MapImpl<K, V>,
}

type MapImpl<K, V> = BTreeMap<K, V>;

impl Map<String, Value> {
    /// Makes a new, empty `Map`.
    ///
    /// # Examples
    ///
    /// ```
    /// use tower_sesh::value::Map;
    ///
    /// let mut map = Map::new();
    ///
    /// // entries can now be inserted into the empty map
    /// map.insert("sesh".to_owned(), "a".into());
    /// ```
    #[inline]
    #[must_use]
    pub fn new() -> Map<String, Value> {
        Map {
            map: MapImpl::new(),
        }
    }

    /// Clears the map, removing all elements.
    ///
    /// # Examples
    ///
    /// ```
    /// use tower_sesh::value::Map;
    ///
    /// let mut a = Map::new();
    /// a.insert("sesh".to_owned(), "a".into());
    /// a.clear();
    /// assert!(a.is_empty());
    /// ```
    #[inline]
    pub fn clear(&mut self) {
        self.map.clear()
    }

    /// Returns a reference to the value corresponding to the key.
    ///
    /// The key may be any borrowed form of the map's key type, but the ordering
    /// on the borrowed form *must* match the ordering on the key type.
    ///
    /// # Examples
    ///
    /// ```
    /// use tower_sesh::value::Map;
    ///
    /// let mut map = Map::new();
    /// map.insert("sesh".to_owned(), "a".into());
    /// assert_eq!(map.get("sesh").and_then(|v| v.as_str()), Some("a"));
    /// assert_eq!(map.get("notexist"), None);
    /// ```
    #[inline]
    pub fn get<Q>(&self, key: &Q) -> Option<&Value>
    where
        String: Borrow<Q>,
        Q: ?Sized + Ord + Eq + Hash,
    {
        self.map.get(key)
    }

    /// Returns the key-value pair matching the given key.
    ///
    /// The key may be any borrowed form of the map's key type, but the ordering
    /// on the borrowed form *must* match the ordering on the key type.
    ///
    /// # Examples
    ///
    /// ```
    /// use tower_sesh::value::Map;
    /// # use std::collections::BTreeMap;
    ///
    /// # fn test() {
    /// let mut map = Map::new();
    /// map.insert("sesh".to_owned(), "a".into());
    /// # assert_eq!(
    /// #     map.get_key_value("sesh")
    /// #         .and_then(|(k, v)| Some(k.as_str()).zip(v.as_str())),
    /// #     Some(("sesh", "a"))
    /// # );
    /// # assert_eq!(map.get_key_value("notexist"), None);
    /// # }
    /// # test();
    /// #
    /// # let mut map = BTreeMap::new();
    /// # map.insert("sesh", "a");
    /// assert_eq!(map.get_key_value("sesh"), Some((&"sesh", &"a")));
    /// assert_eq!(map.get_key_value("notexist"), None);
    /// ```
    #[inline]
    pub fn get_key_value<Q>(&self, key: &Q) -> Option<(&String, &Value)>
    where
        String: Borrow<Q>,
        Q: ?Sized + Ord + Eq + Hash,
    {
        self.map.get_key_value(key)
    }

    /// Returns true if the map contains a value for the specified key.
    ///
    /// The key may be any borrowed form of the map's key type, but the ordering
    /// on the borrowed form *must* match the ordering on the key type.
    ///
    /// # Examples
    ///
    /// ```
    /// use tower_sesh::value::Map;
    ///
    /// let mut map = Map::new();
    /// map.insert("sesh".to_owned(), "a".into());
    /// assert_eq!(map.contains_key("sesh"), true);
    /// assert_eq!(map.contains_key("notexist"), false);
    /// ```
    #[inline]
    pub fn contains_key<Q>(&self, key: &Q) -> bool
    where
        String: Borrow<Q>,
        Q: ?Sized + Ord + Eq + Hash,
    {
        self.map.contains_key(key)
    }

    /// Returns a mutable reference to the value corresponding to the key.
    ///
    /// The key may be any borrowed form of the map's key type, but the ordering
    /// on the borrowed form *must* match the ordering on the key type.
    ///
    /// # Examples
    ///
    /// ```
    /// use tower_sesh::value::Map;
    ///
    /// let mut map = Map::new();
    /// map.insert("sesh".to_owned(), "a".into());
    /// if let Some(x) = map.get_mut("sesh") {
    ///     *x = "b".into();
    /// }
    /// assert_eq!(map["sesh"], "b");
    /// ```
    #[inline]
    pub fn get_mut<Q>(&mut self, key: &Q) -> Option<&mut Value>
    where
        String: Borrow<Q>,
        Q: ?Sized + Ord + Eq + Hash,
    {
        self.map.get_mut(key)
    }

    /// Inserts a key-value pair into the map.
    ///
    /// If the map did not have this key present, `None` is returned.
    ///
    /// If the map did have this key present, the value is updated, and the old
    /// value is returned.
    ///
    /// # Examples
    ///
    /// ```
    /// use tower_sesh::value::Map;
    ///
    /// let mut map = Map::new();
    /// assert_eq!(map.insert("sesh".to_owned(), "a".into()), None);
    /// assert_eq!(map.is_empty(), false);
    ///
    /// map.insert("sesh".to_owned(), "b".into());
    /// let prev = map.insert("sesh".to_owned(), "c".into());
    /// assert_eq!(prev.as_ref().and_then(|v| v.as_str()), Some("b"));
    /// assert_eq!(map["sesh"], "c");
    /// ```
    #[inline]
    pub fn insert(&mut self, key: String, value: Value) -> Option<Value> {
        self.map.insert(key, value)
    }

    /// Removes a key from the map, returning the value at the key if the key
    /// was previously in the map.
    ///
    /// The key may be any borrowed form of the map's key type, but the ordering
    /// on the borrowed form *must* match the ordering on the key type.
    ///
    /// # Examples
    ///
    /// ```
    /// use tower_sesh::value::Map;
    ///
    /// let mut map = Map::new();
    /// map.insert("sesh".to_owned(), "a".into());
    /// assert_eq!(map.remove("sesh").as_ref().and_then(|v| v.as_str()), Some("a"));
    /// assert_eq!(map.remove("sesh"), None);
    /// ```
    #[inline]
    pub fn remove<Q>(&mut self, key: &Q) -> Option<Value>
    where
        String: Borrow<Q>,
        Q: ?Sized + Ord + Eq + Hash,
    {
        self.map.remove(key)
    }

    /// Removes a key from the map, returning the stored key and value if the
    /// key was previously in the map.
    ///
    /// The key may be any borrowed form of the map's key type, but the ordering
    /// on the borrowed form *must* match the ordering on the key type.
    ///
    /// # Examples
    ///
    /// ```
    /// use tower_sesh::value::Map;
    /// # use std::collections::BTreeMap;
    ///
    /// # fn test() {
    /// let mut map = Map::new();
    /// map.insert("sesh".to_owned(), "a".into());
    /// # assert_eq!(
    /// #     map
    /// #         .remove_entry("sesh")
    /// #         .as_ref()
    /// #         .and_then(|(k, v)| Some(k.as_str()).zip(v.as_str())),
    /// #     Some(("sesh", "a"))
    /// # );
    /// # assert_eq!(map.remove_entry("sesh"), None);
    /// # }
    /// # test();
    /// # let mut map = BTreeMap::new();
    /// # map.insert("sesh", "a");
    /// assert_eq!(map.remove_entry("sesh"), Some(("sesh", "a")));
    /// assert_eq!(map.remove_entry("sesh"), None);
    /// ```
    #[inline]
    pub fn remove_entry<Q>(&mut self, key: &Q) -> Option<(String, Value)>
    where
        String: Borrow<Q>,
        Q: ?Sized + Ord + Eq + Hash,
    {
        self.map.remove_entry(key)
    }

    /// Retains only the elements specified by the predicate.
    ///
    /// In other words, remove all pairs `(k, v)` for which `f(&k, &mut v)`
    /// returns `false`.
    ///
    /// # Examples
    ///
    /// ```
    /// use tower_sesh::{value::Map, Value};
    ///
    /// let mut map = Map::from_iter([
    ///     ("rust".into(), "a".into()),
    ///     ("sesh".into(), "b".into()),
    ///     ("tower".into(), "c".into()),
    /// ]);
    /// // Keep only the elements with keys of length 4.
    /// map.retain(|k, _| k.len() == 4);
    /// let elements = map.into_iter().collect::<Vec<(String, Value)>>();
    /// assert_eq!(
    ///     elements,
    ///     vec![("rust".into(), "a".into()), ("sesh".into(), "b".into())]
    /// );
    /// ```
    #[inline]
    pub fn retain<F>(&mut self, f: F)
    where
        F: FnMut(&String, &mut Value) -> bool,
    {
        self.map.retain(f)
    }

    /// Moves all elements from other into self, leaving other empty.
    ///
    /// If a key from `other` is already present in `self`, the respective
    /// value from `self` will be overwritten with the respective value from
    /// `other`.
    ///
    /// # Examples
    ///
    /// ```
    /// use tower_sesh::value::Map;
    ///
    /// let mut a = Map::new();
    /// a.insert("one".to_owned(), "a".into());
    /// a.insert("two".to_owned(), "b".into());
    /// a.insert("three".to_owned(), "c".into()); // Note: Key ("three") also present in b.
    ///
    /// let mut b = Map::new();
    /// b.insert("three".to_owned(), "d".into()); // Note: Key ("three") also present in a.
    /// b.insert("four".to_owned(), "e".into());
    /// b.insert("five".to_owned(), "f".into());
    ///
    /// a.append(&mut b);
    ///
    /// assert_eq!(a.len(), 5);
    /// assert_eq!(b.len(), 0);
    ///
    /// assert_eq!(a["one"], "a");
    /// assert_eq!(a["two"], "b");
    /// assert_eq!(a["three"], "d"); // Note: "c" has been overwritten.
    /// assert_eq!(a["four"], "e");
    /// assert_eq!(a["five"], "f");
    /// ```
    #[inline]
    pub fn append(&mut self, other: &mut Map<String, Value>) {
        self.map.append(&mut other.map)
    }

    /// Gets the given key's corresponding entry in the map for in-place
    /// manipulation.
    ///
    /// # Examples
    ///
    /// ```
    /// use tower_sesh::value::Map;
    ///
    /// let mut count = Map::new();
    ///
    /// // count the number of occurrences of letters in the vec
    /// for x in ["a", "b", "a", "c", "a", "b"] {
    ///     count.entry(x)
    ///         .and_modify(|curr| *curr = (curr.as_u64().unwrap_or(0) + 1).into())
    ///         .or_insert_with(|| 1.into());
    /// }
    ///
    /// assert_eq!(count["a"], 3);
    /// assert_eq!(count["b"], 2);
    /// assert_eq!(count["c"], 1);
    /// ```
    pub fn entry<S>(&mut self, key: S) -> Entry
    where
        S: Into<String>,
    {
        use btree_map::Entry as EntryImpl;

        match self.map.entry(key.into()) {
            EntryImpl::Vacant(vacant) => Entry::Vacant(VacantEntry { vacant }),
            EntryImpl::Occupied(occupied) => Entry::Occupied(OccupiedEntry { occupied }),
        }
    }

    /// Returns the number of elements in the map.
    ///
    /// # Examples
    ///
    /// ```
    /// use tower_sesh::value::Map;
    ///
    /// let mut a = Map::new();
    /// assert_eq!(a.len(), 0);
    /// a.insert("sesh".to_owned(), "a".into());
    /// assert_eq!(a.len(), 1);
    /// ```
    #[inline]
    #[must_use]
    pub fn len(&self) -> usize {
        self.map.len()
    }

    /// Returns `true` if the map contains no elements.
    ///
    /// # Examples
    ///
    /// ```
    /// use tower_sesh::value::Map;
    ///
    /// let mut a = Map::new();
    /// assert!(a.is_empty());
    /// a.insert("sesh".to_owned(), "a".into());
    /// assert!(!a.is_empty());
    /// ```
    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    /// Gets an iterator over the entries of the map.
    ///
    /// # Examples
    ///
    /// ```
    /// use tower_sesh::value::Map;
    ///
    /// let mut map = Map::new();
    /// map.insert("1".to_owned(), "a".into());
    /// map.insert("2".to_owned(), "b".into());
    /// map.insert("3".to_owned(), "c".into());
    ///
    /// for (key, value) in map.iter() {
    ///     println!("{key}: {value:?}");
    /// }
    ///
    /// let (first_key, first_value) = map.iter().next().unwrap();
    /// assert_eq!((first_key.as_str(), first_value.as_str().unwrap()), ("1", "a"));
    /// ```
    #[inline]
    pub fn iter(&self) -> Iter<'_> {
        Iter {
            iter: self.map.iter(),
        }
    }

    /// Gets a mutable iterator over the entries of the map.
    ///
    /// # Examples
    ///
    /// ```
    /// use tower_sesh::value::Map;
    ///
    /// let mut map = Map::from_iter([
    ///     ("a".to_owned(), 1.into()),
    ///     ("b".to_owned(), 2.into()),
    ///     ("c".to_owned(), 3.into()),
    /// ]);
    ///
    /// // add 10 to the value if the key isn't "a"
    /// for (key, value) in map.iter_mut() {
    ///     if key != "a" {
    ///         *value = (value.as_u64().unwrap_or(0) + 10).into();
    ///     }
    /// }
    ///
    /// assert_eq!(map["a"], 1);
    /// assert_eq!(map["b"], 12);
    /// assert_eq!(map["c"], 13);
    /// ```
    #[inline]
    pub fn iter_mut(&mut self) -> IterMut<'_> {
        IterMut {
            iter: self.map.iter_mut(),
        }
    }

    /// Gets an iterator over the keys of the map.
    ///
    /// # Examples
    ///
    /// ```
    /// use tower_sesh::value::Map;
    ///
    /// let mut a = Map::new();
    /// a.insert("rust".to_owned(), "a".into());
    /// a.insert("sesh".to_owned(), "b".into());
    ///
    /// let keys: Vec<_> = a.keys().cloned().collect();
    /// assert_eq!(keys, ["rust", "sesh"]);
    /// ```
    #[inline]
    pub fn keys(&self) -> Keys<'_> {
        Keys {
            iter: self.map.keys(),
        }
    }

    /// Gets an iterator over the values of the map.
    ///
    /// # Examples
    ///
    /// ```
    /// use tower_sesh::value::Map;
    ///
    /// let mut a = Map::new();
    /// a.insert("rust".to_owned(), "hello".into());
    /// a.insert("sesh".to_owned(), "goodbye".into());
    ///
    /// let values: Vec<_> = a.values().cloned().collect();
    /// assert_eq!(values, ["hello", "goodbye"]);
    /// ```
    #[inline]
    pub fn values(&self) -> Values<'_> {
        Values {
            iter: self.map.values(),
        }
    }

    /// Gets a mutable iterator over the values of the map.
    ///
    /// # Examples
    ///
    /// ```
    /// use tower_sesh::{value::Map, Value};
    ///
    /// let mut a = Map::new();
    /// a.insert("rust".to_owned(), "hello".into());
    /// a.insert("sesh".to_owned(), "goodbye".into());
    ///
    /// for value in a.values_mut() {
    ///     match value {
    ///         Value::String(s) => s.push_str("!"),
    ///         _ => unimplemented!(),
    ///     }
    /// }
    ///
    /// let values: Vec<_> = a.values().cloned().collect();
    /// assert_eq!(values, ["hello!", "goodbye!"]);
    /// ```
    #[inline]
    pub fn values_mut(&mut self) -> ValuesMut<'_> {
        ValuesMut {
            iter: self.map.values_mut(),
        }
    }

    /// Creates a consuming iterator visiting all the values of the map.
    /// The map cannot be used after calling this.
    ///
    /// # Examples
    ///
    /// ```
    /// use tower_sesh::value::Map;
    ///
    /// let mut a = Map::new();
    /// a.insert("rust".to_owned(), "hello".into());
    /// a.insert("sesh".to_owned(), "goodbye".into());
    ///
    /// let values: Vec<_> = a.into_values().collect();
    /// assert_eq!(values, ["hello", "goodbye"]);
    /// ```
    #[inline]
    pub fn into_values(self) -> IntoValues {
        IntoValues {
            iter: self.map.into_values(),
        }
    }
}

impl Default for Map<String, Value> {
    /// Creates an empty `Map`.
    #[inline]
    fn default() -> Self {
        Map::new()
    }
}

impl Clone for Map<String, Value> {
    #[inline]
    fn clone(&self) -> Self {
        Map {
            map: self.map.clone(),
        }
    }

    #[inline]
    fn clone_from(&mut self, source: &Self) {
        self.map.clone_from(&source.map)
    }
}

impl PartialEq for Map<String, Value> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.map.eq(&other.map)
    }
}

impl Eq for Map<String, Value> {}

/// Access an element of this map. Panics if the given key is not present in the
/// map.
///
/// ```
/// # use tower_sesh::Value;
/// #
/// # let val = &Value::String("".to_owned());
/// # let _ =
/// match val {
///     Value::String(s) => Some(s.as_str()),
///     Value::Array(arr) => arr[0].as_str(),
///     Value::Map(map) => map["type"].as_str(),
///     _ => None,
/// }
/// # ;
/// ```
impl<Q> ops::Index<&Q> for Map<String, Value>
where
    String: Borrow<Q>,
    Q: ?Sized + Ord + Eq + Hash,
{
    type Output = Value;

    fn index(&self, index: &Q) -> &Self::Output {
        self.map.index(index)
    }
}

/// Mutably access an element of this map. Panics if the given key is not
/// present in the map.
///
/// ```
/// # use tower_sesh::{value::Map, Value};
/// #
/// # let mut map = Map::new();
/// # map.insert("key".to_owned(), Value::Null);
/// #
/// map["key"] = Value::String("value".to_owned());
/// ```
impl<Q> ops::IndexMut<&Q> for Map<String, Value>
where
    String: Borrow<Q>,
    Q: ?Sized + Ord + Eq + Hash,
{
    fn index_mut(&mut self, index: &Q) -> &mut Self::Output {
        self.map.get_mut(index).expect("no entry found for key")
    }
}

impl fmt::Debug for Map<String, Value> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.map.fmt(f)
    }
}

impl Serialize for Map<String, Value> {
    #[inline]
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

impl<'de> Deserialize<'de> for Map<String, Value> {
    #[inline]
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct Visitor;

        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = Map<String, Value>;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a map")
            }

            #[inline]
            fn visit_unit<E>(self) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(Map::new())
            }

            #[inline]
            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::MapAccess<'de>,
            {
                let mut values = Map::new();

                while let Some((key, value)) = map.next_entry()? {
                    values.insert(key, value);
                }

                Ok(values)
            }
        }

        deserializer.deserialize_map(Visitor)
    }
}

impl FromIterator<(String, Value)> for Map<String, Value> {
    fn from_iter<T: IntoIterator<Item = (String, Value)>>(iter: T) -> Self {
        Map {
            map: FromIterator::from_iter(iter),
        }
    }
}

impl Extend<(String, Value)> for Map<String, Value> {
    fn extend<T: IntoIterator<Item = (String, Value)>>(&mut self, iter: T) {
        self.map.extend(iter)
    }
}

////////////////////////////////////////////////////////////////////////////////

/// A view into a single entry in a map, which may be either vacant or occupied.
///
/// This `enum` is constructed from the [`entry`] method on [`Map`].
///
/// [`entry`]: Map::entry
pub enum Entry<'a> {
    Vacant(VacantEntry<'a>),
    Occupied(OccupiedEntry<'a>),
}

impl fmt::Debug for Entry<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Entry::Vacant(v) => f.debug_tuple("Entry").field(v).finish(),
            Entry::Occupied(o) => f.debug_tuple("Entry").field(o).finish(),
        }
    }
}

/// A view into a vacant entry in a `Map`.
/// It is part of the [`Entry`] enum.
pub struct VacantEntry<'a> {
    vacant: VacantEntryImpl<'a>,
}

impl fmt::Debug for VacantEntry<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("VacantEntry").field(self.key()).finish()
    }
}

/// A view into an occupied entry in a `Map`.
/// It is part of the [`Entry`] enum.
pub struct OccupiedEntry<'a> {
    occupied: OccupiedEntryImpl<'a>,
}

impl fmt::Debug for OccupiedEntry<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OccupiedEntry")
            .field("key", self.key())
            .field("value", self.get())
            .finish()
    }
}

type VacantEntryImpl<'a> = btree_map::VacantEntry<'a, String, Value>;

type OccupiedEntryImpl<'a> = btree_map::OccupiedEntry<'a, String, Value>;

impl<'a> Entry<'a> {
    /// Returns a reference to this entry's key.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut map = tower_sesh::value::Map::new();
    /// assert_eq!(map.entry("sesh").key(), &"sesh");
    /// ```
    pub fn key(&self) -> &String {
        match self {
            Entry::Vacant(e) => e.key(),
            Entry::Occupied(e) => e.key(),
        }
    }

    /// Ensures a value is in the entry by inserting the default if empty, and
    /// returns a mutable reference to the value in the entry.
    ///
    /// # Examples
    ///
    /// ```
    /// # use tower_sesh::Value;
    /// #
    /// let mut map = tower_sesh::value::Map::new();
    /// map.entry("sesh").or_insert(Value::from(12));
    ///
    /// assert_eq!(map["sesh"], 12);
    /// ```
    pub fn or_insert(self, default: Value) -> &'a mut Value {
        match self {
            Entry::Vacant(e) => e.insert(default),
            Entry::Occupied(e) => e.into_mut(),
        }
    }

    /// Ensures a value is in the entry by inserting the result of the default
    /// function if empty, and returns a mutable reference to the value in the
    /// entry.
    ///
    /// # Examples
    ///
    /// ```
    /// # use tower_sesh::Value;
    /// #
    /// let mut map = tower_sesh::value::Map::new();
    /// map.entry("sesh").or_insert_with(|| Value::from("hoho"));
    ///
    /// assert_eq!(map["sesh"], "hoho".to_owned());
    /// ```
    pub fn or_insert_with<F>(self, default: F) -> &'a mut Value
    where
        F: FnOnce() -> Value,
    {
        match self {
            Entry::Vacant(e) => e.insert(default()),
            Entry::Occupied(e) => e.into_mut(),
        }
    }

    /// Provides in-place mutable access to an occupied entry before any
    /// potential inserts into the map.
    ///
    /// # Examples
    ///
    /// ```
    /// # use tower_sesh::Value;
    /// #
    /// let mut map = tower_sesh::value::Map::new();
    /// map.entry("sesh")
    ///     .and_modify(|e| *e = Value::from("rust"))
    ///     .or_insert_with(|| Value::from("cpp"));
    ///
    /// assert_eq!(map["sesh"], "cpp");
    ///
    /// map.entry("sesh")
    ///     .and_modify(|e| *e = Value::from("rust"))
    ///     .or_insert_with(|| Value::from("cpp"));
    ///
    /// assert_eq!(map["sesh"], "rust");
    /// ```
    pub fn and_modify<F>(self, f: F) -> Entry<'a>
    where
        F: FnOnce(&mut Value),
    {
        match self {
            Entry::Vacant(e) => Entry::Vacant(e),
            Entry::Occupied(mut e) => {
                f(e.get_mut());
                Entry::Occupied(e)
            }
        }
    }
}

impl<'a> VacantEntry<'a> {
    /// Gets a reference to the key that would be used when inserting a value
    /// through the `VacantEntry`.
    ///
    /// # Examples
    ///
    /// ```
    /// use tower_sesh::value::{map::Entry, Map};
    ///
    /// let mut map = Map::new();
    ///
    /// match map.entry("sesh") {
    ///     Entry::Vacant(vacant) => {
    ///         assert_eq!(vacant.key(), &"sesh");
    ///     }
    ///     Entry::Occupied(_) => unimplemented!(),
    /// }
    /// ```
    #[inline]
    pub fn key(&self) -> &String {
        self.vacant.key()
    }

    /// Sets the value of the entry with the `VacantEntry`'s key, and returns a
    /// mutable reference to it.
    ///
    /// # Examples
    ///
    /// ```
    /// use tower_sesh::value::{map::Entry, Map, Value};
    ///
    /// let mut map = Map::new();
    ///
    /// match map.entry("sesh") {
    ///     Entry::Vacant(vacant) => {
    ///         vacant.insert(Value::from("hoho"));
    ///     }
    ///     Entry::Occupied(_) => unimplemented!(),
    /// }
    /// ```
    #[inline]
    pub fn insert(self, value: Value) -> &'a mut Value {
        self.vacant.insert(value)
    }
}

impl<'a> OccupiedEntry<'a> {
    /// Gets a reference to the key in the entry.
    ///
    /// # Examples
    ///
    /// ```
    /// use tower_sesh::value::{map::Entry, Map, Value};
    ///
    /// let mut map = Map::new();
    /// map.insert("sesh".to_owned(), Value::from(12));
    ///
    /// match map.entry("sesh") {
    ///     Entry::Occupied(occupied) => {
    ///         assert_eq!(occupied.key(), &"sesh");
    ///     }
    ///     Entry::Vacant(_) => unimplemented!(),
    /// }
    /// ```
    #[inline]
    pub fn key(&self) -> &String {
        self.occupied.key()
    }

    /// Gets a reference to the value in the entry.
    ///
    /// # Examples
    ///
    /// ```
    /// use tower_sesh::value::{map::Entry, Map, Value};
    ///
    /// let mut map = Map::new();
    /// map.insert("sesh".to_owned(), Value::from(12));
    ///
    /// match map.entry("sesh") {
    ///     Entry::Occupied(occupied) => {
    ///         assert_eq!(occupied.get(), 12);
    ///     }
    ///     Entry::Vacant(_) => unimplemented!(),
    /// }
    /// ```
    #[inline]
    pub fn get(&self) -> &Value {
        self.occupied.get()
    }

    /// Gets a mutable reference to the value in the entry.
    ///
    /// # Examples
    ///
    /// ```
    /// use tower_sesh::value::{map::Entry, Map, Value};
    ///
    /// let mut map = Map::new();
    /// map.insert("sesh".to_owned(), Value::from([1, 2, 3]));
    ///
    /// match map.entry("sesh") {
    ///     Entry::Occupied(mut occupied) => {
    ///         occupied.get_mut().as_array_mut().unwrap().push(Value::from(4));
    ///     }
    ///     Entry::Vacant(_) => unimplemented!(),
    /// }
    ///
    /// assert_eq!(map["sesh"].as_array().unwrap().len(), 4);
    /// ```
    #[inline]
    pub fn get_mut(&mut self) -> &mut Value {
        self.occupied.get_mut()
    }

    /// Converts the entry into a mutable reference to its value.
    ///
    /// # Examples
    ///
    /// ```
    /// use tower_sesh::value::{map::Entry, Map, Value};
    ///
    /// let mut map = Map::new();
    /// map.insert("sesh".to_owned(), Value::from([1, 2, 3]));
    ///
    /// match map.entry("sesh") {
    ///     Entry::Occupied(mut occupied) => {
    ///         occupied.into_mut().as_array_mut().unwrap().push(Value::from(4));
    ///     }
    ///     Entry::Vacant(_) => unimplemented!(),
    /// }
    ///
    /// assert_eq!(map["sesh"].as_array().unwrap().len(), 4);
    /// ```
    #[inline]
    pub fn into_mut(self) -> &'a mut Value {
        self.occupied.into_mut()
    }

    /// Takes the value of the entry out of the map, and returns it.
    ///
    /// # Examples
    ///
    /// ```
    /// use tower_sesh::value::{map::Entry, Map, Value};
    ///
    /// let mut map = Map::new();
    /// map.insert("sesh".to_owned(), Value::from(12));
    ///
    /// match map.entry("sesh") {
    ///     Entry::Occupied(occupied) => {
    ///         assert_eq!(occupied.remove(), 12);
    ///     }
    ///     Entry::Vacant(_) => unimplemented!(),
    /// }
    /// ```
    #[inline]
    pub fn remove(self) -> Value {
        self.occupied.remove()
    }

    /// Removes the entry from the map, returning the stored key and value.
    ///
    /// # Examples
    ///
    /// ```
    /// use tower_sesh::value::{map::Entry, Map, Value};
    ///
    /// let mut map = Map::new();
    /// map.insert("sesh".to_owned(), Value::from(12));
    ///
    /// match map.entry("sesh") {
    ///     Entry::Occupied(occupied) => {
    ///         let (key, value) = occupied.remove_entry();
    ///         assert_eq!(key, "sesh");
    ///         assert_eq!(value, 12);
    ///     }
    ///     Entry::Vacant(_) => unimplemented!(),
    /// }
    /// ```
    #[inline]
    pub fn remove_entry(self) -> (String, Value) {
        self.occupied.remove_entry()
    }
}

macro_rules! delegate_iterator {
    (($name:ident $($generics:tt)*) => $item:ty) => {
        impl $($generics)* Iterator for $name $($generics)* {
            type Item = $item;

            #[inline]
            fn next(&mut self) -> Option<Self::Item> {
                self.iter.next()
            }

            #[inline]
            fn size_hint(&self) -> (usize, Option<usize>) {
                self.iter.size_hint()
            }
        }

        impl $($generics)* DoubleEndedIterator for $name $($generics)* {
            #[inline]
            fn next_back(&mut self) -> Option<Self::Item> {
                self.iter.next_back()
            }
        }

        impl $($generics)* ExactSizeIterator for $name $($generics)* {
            #[inline]
            fn len(&self) -> usize {
                self.iter.len()
            }
        }

        impl $($generics)* FusedIterator for $name $($generics)* {}
    };
}

macro_rules! delegate_debug {
    ($name:ident $($generics:tt)*) => {
        impl $($generics)* std::fmt::Debug for $name $($generics)* {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                std::fmt::Debug::fmt(&self.iter, f)
            }
        }
    };
}

////////////////////////////////////////////////////////////////////////////////

impl<'a> IntoIterator for &'a Map<String, Value> {
    type Item = <Self::IntoIter as Iterator>::Item;
    type IntoIter = Iter<'a>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        Iter {
            iter: self.map.iter(),
        }
    }
}

impl<'a> IntoIterator for &'a mut Map<String, Value> {
    type Item = <Self::IntoIter as Iterator>::Item;
    type IntoIter = IterMut<'a>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        IterMut {
            iter: self.map.iter_mut(),
        }
    }
}

/// An iterator over the entries of a `Map`.
///
/// This `struct` is created by the [`iter`] method on [`Map`]. See its
/// documentation for more.
///
/// [`iter`]: Map::iter
#[must_use = "iterators are lazy and do nothing unless consumed"]
#[derive(Clone, Default)]
pub struct Iter<'a> {
    iter: IterImpl<'a>,
}

type IterImpl<'a> = btree_map::Iter<'a, String, Value>;

delegate_iterator!((Iter<'a>) => (&'a String, &'a Value));
delegate_debug!(Iter<'a>);

/// A mutable iterator over the entries of a `Map`.
///
/// This `struct` is created by the [`iter_mut`] method on [`Map`]. See its
/// documentation for more.
///
/// [`iter_mut`]: Map::iter_mut
#[must_use = "iterators are lazy and do nothing unless consumed"]
#[derive(Default)]
pub struct IterMut<'a> {
    iter: IterMutImpl<'a>,
}

type IterMutImpl<'a> = btree_map::IterMut<'a, String, Value>;

delegate_iterator!((IterMut<'a>) => (&'a String, &'a mut Value));
delegate_debug!(IterMut<'a>);

////////////////////////////////////////////////////////////////////////////////

impl IntoIterator for Map<String, Value> {
    type Item = <Self::IntoIter as Iterator>::Item;
    type IntoIter = IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        IntoIter {
            iter: self.map.into_iter(),
        }
    }
}

/// An owning iterator over the entries of a `Map`.
///
/// This `struct` is created by the [`into_iter`] method on [`Map`]
/// (provided by the [`IntoIterator`] trait). See its documentation for more.
///
/// [`into_iter`]: IntoIterator::into_iter
#[derive(Default)]
pub struct IntoIter {
    iter: IntoIterImpl,
}

type IntoIterImpl = btree_map::IntoIter<String, Value>;

delegate_iterator!((IntoIter) => (String, Value));
delegate_debug!(IntoIter);

////////////////////////////////////////////////////////////////////////////////

/// An iterator over the keys of a `Map`.
///
/// This `struct` is created by the [`keys`] method on [`Map`]. See its
/// documentation for more.
///
/// [`keys`]: Map::keys
#[must_use = "iterators are lazy and do nothing unless consumed"]
#[derive(Clone, Default)]
pub struct Keys<'a> {
    iter: KeysImpl<'a>,
}

type KeysImpl<'a> = btree_map::Keys<'a, String, Value>;

delegate_iterator!((Keys<'a>) => &'a String);
delegate_debug!(Keys<'a>);

////////////////////////////////////////////////////////////////////////////////

/// An iterator over the values of a `Map`.
///
/// This `struct` is created by the [`values`] method on [`Map`]. See its
/// documentation for more.
///
/// [`values`]: Map::values
#[must_use = "iterators are lazy and do nothing unless consumed"]
#[derive(Clone, Default)]
pub struct Values<'a> {
    iter: ValuesImpl<'a>,
}

type ValuesImpl<'a> = btree_map::Values<'a, String, Value>;

delegate_iterator!((Values<'a>) => &'a Value);
delegate_debug!(Values<'a>);

//////////////////////////////////////////////////////////////////////////////

/// A mutable iterator over the values of a `Map`.
///
/// This `struct` is created by the [`values_mut`] method on [`Map`]. See its
/// documentation for more.
///
/// [`values_mut`]: Map::values_mut
#[must_use = "iterators are lazy and do nothing unless consumed"]
#[derive(Default)]
pub struct ValuesMut<'a> {
    iter: ValuesMutImpl<'a>,
}

type ValuesMutImpl<'a> = btree_map::ValuesMut<'a, String, Value>;

delegate_iterator!((ValuesMut<'a>) => &'a mut Value);
delegate_debug!(ValuesMut<'a>);

////////////////////////////////////////////////////////////////////////////////

/// An owning iterator over the values of a `Map`.
///
/// This `struct` is created by the [`into_values`] method on [`Map`]. See its
/// documentation for more.
///
/// [`into_values`]: Map::into_values
#[must_use = "iterators are lazy and do nothing unless consumed"]
#[derive(Default)]
pub struct IntoValues {
    iter: IntoValuesImpl,
}

type IntoValuesImpl = btree_map::IntoValues<String, Value>;

delegate_iterator!((IntoValues) => Value);
delegate_debug!(IntoValues);

#[cfg(test)]
#[test]
fn test_debug() {
    let mut map = Map::from_iter([
        ("rust".to_owned(), "now".into()),
        ("sesh".to_owned(), "wow".into()),
    ]);
    assert_eq!(
        format!("{:?}", map),
        r#"{"rust": String("now"), "sesh": String("wow")}"#
    );
    assert_eq!(
        format!("{:?}", map.entry("notexist")),
        r#"Entry(VacantEntry("notexist"))"#
    );
    assert_eq!(
        format!("{:?}", map.entry("rust")),
        r#"Entry(OccupiedEntry { key: "rust", value: String("now") })"#
    );
    assert_eq!(
        format!("{:?}", map.iter()),
        r#"[("rust", String("now")), ("sesh", String("wow"))]"#
    );
    assert_eq!(
        format!("{:?}", map.iter_mut()),
        r#"[("rust", String("now")), ("sesh", String("wow"))]"#
    );
    assert_eq!(
        format!("{:?}", map.clone().into_iter()),
        r#"[("rust", String("now")), ("sesh", String("wow"))]"#
    );
    assert_eq!(format!("{:?}", map.keys()), r#"["rust", "sesh"]"#);
    assert_eq!(
        format!("{:?}", map.values()),
        r#"[String("now"), String("wow")]"#
    );
    assert_eq!(
        format!("{:?}", map.values_mut()),
        r#"[String("now"), String("wow")]"#
    );
    assert_eq!(
        format!("{:?}", map.clone().into_values()),
        r#"[String("now"), String("wow")]"#
    );
}
