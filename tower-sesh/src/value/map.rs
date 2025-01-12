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
    /// Makes a new empty `Map`.
    #[inline]
    pub fn new() -> Self {
        Map {
            map: MapImpl::new(),
        }
    }

    /// Clear the map, removing all values.
    #[inline]
    pub fn clear(&mut self) {
        self.map.clear()
    }

    /// Returns a reference to the value corresponding to the key.
    ///
    /// The key may be any borrowed form of the map's key type, but the ordering
    /// on the borrowed form *must* match the ordering on the key type.
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
    #[inline]
    pub fn insert(&mut self, key: String, value: Value) -> Option<Value> {
        self.map.insert(key, value)
    }

    /// Removes a key from the map, returning the value at the key if the key
    /// was previously in the map.
    ///
    /// The key may be any borrowed form of the map's key type, but the ordering
    /// on the borrowed form *must* match the ordering on the key type.
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
    #[inline]
    pub fn retain<F>(&mut self, f: F)
    where
        F: FnMut(&String, &mut Value) -> bool,
    {
        self.map.retain(f)
    }

    /// Moves all elements from other into self, leaving other empty.
    #[inline]
    pub fn append(&mut self, other: &mut Self) {
        self.map.append(&mut other.map)
    }

    /// Gets the given key's corresponding entry in the map for in-place
    /// manipulation.
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
    #[inline]
    pub fn len(&self) -> usize {
        self.map.len()
    }

    /// Returns `true` if the map contains no elements.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    /// Gets an iterator over the entries of the map.
    #[inline]
    pub fn iter(&self) -> Iter<'_> {
        Iter {
            iter: self.map.iter(),
        }
    }

    /// Gets a mutable iterator over the entries of the map.
    #[inline]
    pub fn iter_mut(&mut self) -> IterMut<'_> {
        IterMut {
            iter: self.map.iter_mut(),
        }
    }

    /// Gets an iterator over the keys of the map.
    #[inline]
    pub fn keys(&self) -> Keys<'_> {
        Keys {
            iter: self.map.keys(),
        }
    }

    /// Gets an iterator over the values of the map.
    #[inline]
    pub fn values(&self) -> Values<'_> {
        Values {
            iter: self.map.values(),
        }
    }

    /// Gets a mutable iterator over the values of the map.
    #[inline]
    pub fn values_mut(&mut self) -> ValuesMut<'_> {
        ValuesMut {
            iter: self.map.values_mut(),
        }
    }

    /// Creates a consuming iterator visiting all the values of the map.
    /// The map cannot be used after calling this.
    pub fn into_values(self) -> IntoValues {
        IntoValues {
            iter: self.map.into_values(),
        }
    }
}

impl Default for Map<String, Value> {
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
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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

/// A view into a vacant entry in a `Map`. It is part of the [`Entry`] enum.
pub struct VacantEntry<'a> {
    vacant: VacantEntryImpl<'a>,
}

/// A view into an occupied entry in a `Map`. It is part of the [`Entry`] enum.
pub struct OccupiedEntry<'a> {
    occupied: OccupiedEntryImpl<'a>,
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
    pub fn and_modify<F>(self, f: F) -> Self
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

/// An iterator over the entries of a `Map`.
pub struct Iter<'a> {
    iter: IterImpl<'a>,
}

type IterImpl<'a> = btree_map::Iter<'a, String, Value>;

delegate_iterator!((Iter<'a>) => (&'a String, &'a Value));

/// A mutable iterator over the entries of a `Map`.
pub struct IterMut<'a> {
    iter: IterMutImpl<'a>,
}

type IterMutImpl<'a> = btree_map::IterMut<'a, String, Value>;

delegate_iterator!((IterMut<'a>) => (&'a String, &'a mut Value));

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
pub struct IntoIter {
    iter: IntoIterImpl,
}

type IntoIterImpl = btree_map::IntoIter<String, Value>;

delegate_iterator!((IntoIter) => (String, Value));

////////////////////////////////////////////////////////////////////////////////

/// An iterator over the keys of a `Map`.
pub struct Keys<'a> {
    iter: KeysImpl<'a>,
}

type KeysImpl<'a> = btree_map::Keys<'a, String, Value>;

delegate_iterator!((Keys<'a>) => &'a String);

////////////////////////////////////////////////////////////////////////////////

/// An iterator over the values of a `Map`.
pub struct Values<'a> {
    iter: ValuesImpl<'a>,
}

type ValuesImpl<'a> = btree_map::Values<'a, String, Value>;

delegate_iterator!((Values<'a>) => &'a Value);

//////////////////////////////////////////////////////////////////////////////

/// A mutable iterator over the values of a `Map`.
pub struct ValuesMut<'a> {
    iter: ValuesMutImpl<'a>,
}

type ValuesMutImpl<'a> = btree_map::ValuesMut<'a, String, Value>;

delegate_iterator!((ValuesMut<'a>) => &'a mut Value);

////////////////////////////////////////////////////////////////////////////////

/// An owning iterator over the values of a `Map`.
pub struct IntoValues {
    iter: IntoValuesImpl,
}

type IntoValuesImpl = btree_map::IntoValues<String, Value>;

delegate_iterator!((IntoValues) => Value);
