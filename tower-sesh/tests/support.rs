use std::{collections::HashMap, fmt, marker::PhantomData, sync::Arc};

use async_trait::async_trait;
use parking_lot::Mutex;
use rand::Rng;
use tower_sesh_core::{
    store::{self, Result, SessionStoreImpl},
    Record, SessionKey, SessionStore, Ttl,
};

/// A `SessionStore` that only returns errors.
pub struct ErrStore<T> {
    error_fn: Box<dyn Fn() -> store::Error + Send + Sync + 'static>,
    _marker: PhantomData<fn() -> T>,
}

impl<T> ErrStore<T> {
    pub fn new<F>(f: F) -> Self
    where
        F: Fn() -> store::Error + Send + Sync + 'static,
    {
        ErrStore {
            error_fn: Box::new(f),
            _marker: PhantomData,
        }
    }
}

impl<T> SessionStore<T> for ErrStore<T> where T: Send + Sync + 'static {}
#[async_trait]
impl<T> SessionStoreImpl<T> for ErrStore<T>
where
    T: Send + Sync + 'static,
{
    async fn create(&self, _data: &T, _ttl: Ttl) -> Result<SessionKey> {
        Err((self.error_fn)())
    }

    async fn load(&self, _session_key: &SessionKey) -> Result<Option<Record<T>>> {
        Err((self.error_fn)())
    }

    async fn update(&self, _session_key: &SessionKey, _data: &T, _ttl: Ttl) -> Result<()> {
        Err((self.error_fn)())
    }

    async fn update_ttl(&self, _session_key: &SessionKey, _ttl: Ttl) -> Result<()> {
        Err((self.error_fn)())
    }

    async fn delete(&self, _session_key: &SessionKey) -> Result<()> {
        Err((self.error_fn)())
    }
}

#[derive(Debug)]
pub struct MockStore<T> {
    inner: Arc<Mutex<MockStoreInner<T>>>,
}

struct MockStoreInner<T> {
    operations: Vec<Arc<Operation<T>>>,
    operations_map: HashMap<SessionKey, Vec<OperationMapEntry<T>>>,
    rng: Option<Box<dyn rand::CryptoRng + Send + 'static>>,
}

impl<T> fmt::Debug for MockStoreInner<T>
where
    T: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut d = f.debug_struct("MockStoreInner");

        d.field("operations", &self.operations);
        d.field("operations_map", &self.operations_map);

        d.finish()
    }
}

struct OperationMapEntry<T> {
    operation: std::sync::Weak<Operation<T>>,
    state: EntryState,
}

impl<T> OperationMapEntry<T> {
    fn new(operation: std::sync::Weak<Operation<T>>) -> Self {
        OperationMapEntry {
            operation,
            state: EntryState::Valid,
        }
    }
}

impl<T> fmt::Debug for OperationMapEntry<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut d = f.debug_struct("OperationMapEntry");

        d.field("operation", &WeakOperationDebug(&self.operation));
        d.field("state", &self.state);

        d.finish()
    }
}

struct WeakOperationDebug<'a, T>(&'a std::sync::Weak<Operation<T>>);

impl<T> fmt::Debug for WeakOperationDebug<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(upgraded) = self.0.upgrade() {
            match upgraded.as_ref() {
                Operation::Create { .. } => f.write_str("Operation::Create { .. }"),
                Operation::Load { .. } => f.write_str("Operation::Load { .. }"),
                Operation::Update { .. } => f.write_str("Operation::Update { .. }"),
                Operation::UpdateTtl { .. } => f.write_str("Operation::UpdateTtl { .. }"),
                Operation::Delete { .. } => f.write_str("Operation::Delete { .. }"),
            }
        } else {
            f.write_str("(Weak)")
        }
    }
}

#[derive(Debug)]
enum EntryState {
    Expired,
    Valid,
}

#[derive(Debug)]
enum Operation<T> {
    Create {
        data: T,
        ttl: Ttl,
        result: CreateResult,
    },
    Load {
        session_key: SessionKey,
        result: LoadResult<T>,
    },
    Update {
        session_key: SessionKey,
        data: T,
        ttl: Ttl,
    },
    UpdateTtl {
        session_key: SessionKey,
        ttl: Ttl,
    },
    Delete {
        session_key: SessionKey,
    },
}

#[derive(Debug)]
enum CreateResult {
    Created { session_key: SessionKey },
    MaxIterationsReached,
}

#[derive(Debug)]
enum LoadResult<T> {
    Vacant,
    Occupied { data: T, ttl: Ttl },
}

impl<T> MockStore<T>
where
    T: Clone,
{
    pub fn new() -> Self {
        let inner = Arc::new(Mutex::new(MockStoreInner::new()));
        MockStore { inner }
    }

    #[track_caller]
    pub fn assert_finished() {
        todo!()
    }
}

impl<T> Clone for MockStore<T> {
    fn clone(&self) -> Self {
        MockStore {
            inner: Arc::clone(&self.inner),
        }
    }
}

impl<T> Default for MockStore<T>
where
    T: Clone,
{
    fn default() -> Self {
        MockStore::new()
    }
}

impl<T> SessionStore<T> for MockStore<T> where T: Clone + Send + Sync + 'static {}
#[async_trait]
impl<T> SessionStoreImpl<T> for MockStore<T>
where
    T: Clone + Send + Sync + 'static,
{
    async fn create(&self, data: &T, ttl: Ttl) -> Result<SessionKey> {
        let mut guard = self.inner.lock();

        const MAX_ITERATIONS: usize = 8;
        for _ in 0..MAX_ITERATIONS {
            let session_key = guard.random_key();
            let result = guard.load_result(&session_key);
            match result {
                LoadResult::Vacant => {
                    let operation = Arc::new(Operation::Create {
                        data: data.to_owned(),
                        ttl,
                        result: CreateResult::Created {
                            session_key: session_key.clone(),
                        },
                    });
                    let operations = guard.operations_map.entry(session_key.clone()).or_default();
                    operations.push(OperationMapEntry::new(Arc::downgrade(&operation)));
                    guard.operations.push(operation);

                    return Ok(session_key);
                }
                LoadResult::Occupied { .. } => continue,
            }
        }

        guard.operations.push(Arc::new(Operation::Create {
            data: data.to_owned(),
            ttl,
            result: CreateResult::MaxIterationsReached,
        }));

        Err(store::Error::max_iterations_reached())
    }

    async fn load(&self, session_key: &SessionKey) -> Result<Option<Record<T>>> {
        let mut guard = self.inner.lock();

        let result = guard.load_result(session_key);
        let record = match &result {
            LoadResult::Vacant => None,
            LoadResult::Occupied { data, ttl } => Some(Record::new(data.to_owned(), *ttl)),
        };
        let operation = Arc::new(Operation::Load {
            session_key: session_key.to_owned(),
            result,
        });

        let operations = guard
            .operations_map
            .entry(session_key.to_owned())
            .or_default();
        operations.push(OperationMapEntry::new(Arc::downgrade(&operation)));
        guard.operations.push(operation);

        Ok(record)
    }

    async fn update(&self, session_key: &SessionKey, data: &T, ttl: Ttl) -> Result<()> {
        let mut guard = self.inner.lock();

        let operation = Arc::new(Operation::Update {
            session_key: session_key.to_owned(),
            data: data.to_owned(),
            ttl,
        });

        let operations = guard
            .operations_map
            .entry(session_key.to_owned())
            .or_default();
        operations.push(OperationMapEntry::new(Arc::downgrade(&operation)));
        guard.operations.push(operation);

        Ok(())
    }

    async fn update_ttl(&self, session_key: &SessionKey, ttl: Ttl) -> Result<()> {
        let mut guard = self.inner.lock();

        guard.revalidate_last_operation_which_modified_ttl(session_key);

        let operation = Arc::new(Operation::UpdateTtl {
            session_key: session_key.to_owned(),
            ttl,
        });

        let operations = guard
            .operations_map
            .entry(session_key.to_owned())
            .or_default();
        operations.push(OperationMapEntry::new(Arc::downgrade(&operation)));
        guard.operations.push(operation);

        Ok(())
    }

    async fn delete(&self, session_key: &SessionKey) -> Result<()> {
        let mut guard = self.inner.lock();

        let operation = Arc::new(Operation::Delete {
            session_key: session_key.to_owned(),
        });

        let operations = guard
            .operations_map
            .entry(session_key.to_owned())
            .or_default();
        operations.push(OperationMapEntry::new(Arc::downgrade(&operation)));
        guard.operations.push(operation);

        Ok(())
    }
}

impl<T, Rng> tower_sesh_core::store::SessionStoreRng<Rng> for MockStore<T>
where
    Rng: rand::CryptoRng + Send + 'static,
{
    fn rng(&mut self, rng: Rng) {
        self.inner.lock().rng = Some(Box::new(rng));
    }
}

impl<T> MockStoreInner<T>
where
    T: Clone,
{
    fn new() -> Self {
        MockStoreInner {
            operations: Vec::new(),
            operations_map: HashMap::new(),
            rng: None,
        }
    }

    fn random_key(&mut self) -> SessionKey {
        match &mut self.rng {
            Some(rng) => rng.random(),
            None => rand::rngs::ThreadRng::default().random(),
        }
    }

    fn revalidate_last_operation_which_modified_ttl(&mut self, session_key: &SessionKey) {
        let Some(operations) = self.operations_map.get_mut(session_key) else {
            return;
        };

        for (operation, state) in operations
            .iter_mut()
            .map(|entry| (&entry.operation, &mut entry.state))
            .rev()
        {
            if matches!(state, EntryState::Expired) {
                return;
            }

            match operation.upgrade().unwrap().as_ref() {
                Operation::Create {
                    data: _,
                    ttl,
                    result: CreateResult::Created { .. },
                }
                | Operation::Update {
                    session_key: _,
                    data: _,
                    ttl,
                }
                | Operation::UpdateTtl {
                    session_key: _,
                    ttl,
                } => {
                    if *ttl >= Ttl::now_local().unwrap() {
                    } else {
                        *state = EntryState::Expired;
                    }
                    return;
                }
                Operation::Delete { session_key: _ } => return,
                Operation::Load { .. }
                | Operation::Create {
                    result: CreateResult::MaxIterationsReached,
                    ..
                } => continue,
            }
        }
    }

    fn load_result(&self, session_key: &SessionKey) -> LoadResult<T> {
        // If the latest operation was `update_ttl`, this will contain the
        // up-to-date TTL.
        let mut latest_ttl: Option<Ttl> = None;

        for (operation, state) in self
            .operations_map
            .get(session_key)
            .iter()
            .flat_map(|v| v.iter())
            .map(|entry| (&entry.operation, &entry.state))
            .rev()
        {
            if matches!(state, EntryState::Expired) {
                return LoadResult::Vacant;
            }

            match operation.upgrade().unwrap().as_ref() {
                Operation::Create {
                    data,
                    ttl,
                    result: CreateResult::Created { .. },
                } => {
                    let result = if latest_ttl.unwrap_or(*ttl) >= Ttl::now_local().unwrap() {
                        LoadResult::Occupied {
                            data: data.to_owned(),
                            ttl: latest_ttl.unwrap_or(*ttl),
                        }
                    } else {
                        LoadResult::Vacant
                    };
                    return result;
                }
                Operation::Load { .. } => continue,
                Operation::Update {
                    session_key: _,
                    data,
                    ttl,
                } => {
                    let result = if latest_ttl.unwrap_or(*ttl) >= Ttl::now_local().unwrap() {
                        LoadResult::Occupied {
                            data: data.to_owned(),
                            ttl: latest_ttl.unwrap_or(*ttl),
                        }
                    } else {
                        LoadResult::Vacant
                    };
                    return result;
                }
                Operation::UpdateTtl {
                    session_key: _,
                    ttl,
                } if latest_ttl.is_none() => {
                    if *ttl >= Ttl::now_local().unwrap() {
                        latest_ttl = Some(*ttl);
                        continue;
                    } else {
                        return LoadResult::Vacant;
                    }
                }
                Operation::UpdateTtl { .. } => continue,
                Operation::Delete { session_key: _ } => {
                    return LoadResult::Vacant;
                }
                Operation::Create {
                    data: _,
                    ttl: _,
                    result: CreateResult::MaxIterationsReached,
                } => unreachable!(),
            }
        }

        LoadResult::Vacant
    }
}
