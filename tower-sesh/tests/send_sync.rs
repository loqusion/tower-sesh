// The code in this module is derived from the `tokio` crate by Tokio Contributors.
//
// Licensed under the MIT license.

#![allow(clippy::diverging_sub_expression)]

use std::{cell::Cell, marker::PhantomData, rc::Rc};

// Send: Yes, Sync: Yes
#[derive(Clone)]
#[allow(unused)]
struct YY {}

// Send: Yes, Sync: No
#[derive(Clone)]
#[allow(unused)]
struct YN {
    _value: Cell<u8>,
}

// Send: No, Sync: No
#[derive(Clone)]
#[allow(unused)]
struct NN {
    _value: Rc<u8>,
}

#[allow(dead_code)]
fn require_send<T: Send>(_t: &T) {}
#[allow(dead_code)]
fn require_sync<T: Sync>(_t: &T) {}
#[allow(dead_code)]
fn require_unpin<T: Unpin>(_t: &T) {}

#[allow(dead_code)]
struct Invalid;

#[allow(unused)]
trait AmbiguousIfSend<A> {
    fn some_item(&self) {}
}
impl<T: ?Sized> AmbiguousIfSend<()> for T {}
impl<T: ?Sized + Send> AmbiguousIfSend<Invalid> for T {}

#[allow(unused)]
trait AmbiguousIfSync<A> {
    fn some_item(&self) {}
}
impl<T: ?Sized> AmbiguousIfSync<()> for T {}
impl<T: ?Sized + Sync> AmbiguousIfSync<Invalid> for T {}

#[allow(unused)]
trait AmbiguousIfUnpin<A> {
    fn some_item(&self) {}
}
impl<T: ?Sized> AmbiguousIfUnpin<()> for T {}
impl<T: ?Sized + Unpin> AmbiguousIfUnpin<Invalid> for T {}

macro_rules! async_assert_fn_send {
    (Send & $(!)?Sync & $(!)?Unpin, $value:expr) => {
        require_send(&$value);
    };
    (!Send & $(!)?Sync & $(!)?Unpin, $value:expr) => {
        AmbiguousIfSend::some_item(&$value);
    };
}
macro_rules! async_assert_fn_sync {
    ($(!)?Send & Sync & $(!)?Unpin, $value:expr) => {
        require_sync(&$value);
    };
    ($(!)?Send & !Sync & $(!)?Unpin, $value:expr) => {
        AmbiguousIfSync::some_item(&$value);
    };
}
macro_rules! async_assert_fn_unpin {
    ($(!)?Send & $(!)?Sync & Unpin, $value:expr) => {
        require_unpin(&$value);
    };
    ($(!)?Send & $(!)?Sync & !Unpin, $value:expr) => {
        AmbiguousIfUnpin::some_item(&$value);
    };
}

macro_rules! assert_value {
    ($type:ty: $($tok:tt)*) => {
        #[allow(unreachable_code)]
        #[allow(unused_variables)]
        const _: fn() = || {
            let f: $type = todo!();
            async_assert_fn_send!($($tok)*, f);
            async_assert_fn_sync!($($tok)*, f);
            async_assert_fn_unpin!($($tok)*, f);
        };
    };
}

#[allow(dead_code)]
struct MockStore<T> {
    _marker: PhantomData<fn() -> T>,
}
impl<T> tower_sesh_core::SessionStore<T> for MockStore<T> where T: Send + Sync + 'static {}
#[async_trait::async_trait]
impl<T> tower_sesh_core::store::SessionStoreImpl<T> for MockStore<T>
where
    T: Send + Sync + 'static,
{
    async fn create(
        &self,
        _data: &T,
        _ttl: tower_sesh_core::Ttl,
    ) -> Result<tower_sesh_core::SessionKey, tower_sesh_core::store::Error> {
        unimplemented!()
    }
    async fn load(
        &self,
        _session_key: &tower_sesh_core::SessionKey,
    ) -> Result<Option<tower_sesh_core::Record<T>>, tower_sesh_core::store::Error> {
        unimplemented!()
    }
    async fn update(
        &self,
        _session_key: &tower_sesh_core::SessionKey,
        _data: &T,
        _ttl: tower_sesh_core::Ttl,
    ) -> Result<(), tower_sesh_core::store::Error> {
        unimplemented!()
    }
    async fn update_ttl(
        &self,
        _session_key: &tower_sesh_core::SessionKey,
        _ttl: tower_sesh_core::Ttl,
    ) -> Result<(), tower_sesh_core::store::Error> {
        unimplemented!()
    }
    async fn delete(
        &self,
        _session_key: &tower_sesh_core::SessionKey,
    ) -> Result<(), tower_sesh_core::store::Error> {
        unimplemented!()
    }
}

assert_value!(tower_sesh::Session<YY>: Send & Sync & Unpin);
assert_value!(tower_sesh::Session<YN>: Send & Sync & Unpin);
assert_value!(tower_sesh::Session<NN>: !Send & !Sync & Unpin);
assert_value!(tower_sesh::SessionLayer<YY, MockStore<YY>>: Send & Sync & Unpin);
assert_value!(tower_sesh::Value: Send & Sync & Unpin);
assert_value!(tower_sesh::middleware::SessionManager<(), YY, MockStore<YY>, tower_sesh::config::PrivateCookie>: Send & Sync & Unpin);
assert_value!(tower_sesh::session::SessionGuard<YY>: !Send & Sync & Unpin);
assert_value!(tower_sesh::session::SessionGuard<YN>: !Send & !Sync & Unpin);
assert_value!(tower_sesh::session::SessionGuard<NN>: !Send & !Sync & Unpin);
#[cfg(feature = "axum")]
assert_value!(tower_sesh::session::SessionRejection: Send & Sync & Unpin);
assert_value!(tower_sesh::store::CachingStore<YY, MockStore<YY>, MockStore<YY>>: Send & Sync & Unpin);
assert_value!(tower_sesh::value::Map<String, tower_sesh::Value>: Send & Sync & Unpin);
assert_value!(tower_sesh::value::Number: Send & Sync & Unpin);
