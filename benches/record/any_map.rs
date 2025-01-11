use std::{any::Any, collections::HashMap};

use serde::{ser::SerializeMap, Deserialize, Serialize};

#[derive(Clone)]
pub struct Record {
    data: AnyMap,
}

impl Record {
    #[inline]
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
        }
    }

    pub fn insert<T: AnyClone + Send + Sync + 'static>(&mut self, k: &str, v: T) -> Option<T> {
        self.data
            .insert(k.to_owned(), Box::new(v))
            .and_then(|boxed| boxed.into_any().downcast().ok().map(|boxed| *boxed))
    }

    pub fn get<T: Send + Sync + 'static>(&self, key: &str) -> Result<Option<&T>, GetError> {
        self.data
            .get(key)
            .map(|boxed| boxed.as_any().downcast_ref().ok_or(GetError))
            .transpose()
    }

    pub fn get_mut<T: Send + Sync + 'static>(
        &mut self,
        key: &str,
    ) -> Result<Option<&mut T>, GetError> {
        self.data
            .get_mut(key)
            .map(|boxed| boxed.as_any_mut().downcast_mut().ok_or(GetError))
            .transpose()
    }

    pub fn remove<T: Send + Sync + 'static>(&mut self, key: &str) -> Option<T> {
        // In the real implementation, we might want to validate that value can
        // downcast to type T before removing
        self.data
            .remove(key)
            .and_then(|boxed| boxed.into_any().downcast().ok().map(|boxed| *boxed))
    }
}

#[derive(Debug, thiserror::Error)]
#[error("failed to downcast value to target type")]
#[non_exhaustive]
pub struct GetError;

impl<'a, T> Extend<(&'a str, T)> for Record
where
    T: AnyClone + Send + Sync + 'static,
{
    fn extend<I: IntoIterator<Item = (&'a str, T)>>(&mut self, iter: I) {
        let iter = iter
            .into_iter()
            .map(|(k, v)| -> (String, Box<dyn AnyClone + Send + Sync>) {
                (k.to_owned(), Box::new(v))
            });
        self.data.extend(iter)
    }
}

impl<T> Extend<(String, T)> for Record
where
    T: AnyClone + Send + Sync + 'static,
{
    fn extend<I: IntoIterator<Item = (String, T)>>(&mut self, iter: I) {
        let iter = iter
            .into_iter()
            .map(|(k, v)| -> (String, Box<dyn AnyClone + Send + Sync>) { (k, Box::new(v)) });
        self.data.extend(iter)
    }
}

impl Serialize for Record {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut iter = self.data.iter();
        let mut serializer = serializer.serialize_map(Some(self.data.len()))?;
        iter.try_for_each(|(key, value)| serializer.serialize_entry(&key, &value))?;
        serializer.end()
    }
}

impl<'de> Deserialize<'de> for Record {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        todo!()
    }
}

type AnyMap = HashMap<String, Box<dyn AnyClone + Send + Sync>>;

#[typetag::serde(tag = "t", content = "v")]
pub trait AnyClone: Any {
    fn clone_box(&self) -> Box<dyn AnyClone + Send + Sync>;
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn into_any(self: Box<Self>) -> Box<dyn Any>;
}

macro_rules! impl_any_clone {
    ($ty:ty) => {
        #[::typetag::serde]
        impl AnyClone for $ty {
            fn clone_box(&self) -> ::std::boxed::Box<dyn AnyClone + Send + Sync> {
                ::std::boxed::Box::new(::core::clone::Clone::clone(self))
            }

            fn as_any(&self) -> &dyn ::core::any::Any {
                self
            }

            fn as_any_mut(&mut self) -> &mut dyn ::core::any::Any {
                self
            }

            fn into_any(self: ::std::boxed::Box<Self>) -> ::std::boxed::Box<dyn ::core::any::Any> {
                self
            }
        }
    };
}

impl_any_clone!(String);

impl Clone for Box<dyn AnyClone + Send + Sync> {
    fn clone(&self) -> Self {
        (**self).clone_box()
    }
}
