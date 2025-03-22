mod support;

mod memory_store {
    use tower_sesh::store::MemoryStore;

    tower_sesh_test::test_suite! {
        store: MemoryStore::new(),
    }
}

mod memory_store_caching_store {
    use tower_sesh::store::{CachingStore, MemoryStore};

    tower_sesh_test::test_suite! {
        store: CachingStore::from_cache_and_store(
            MemoryStore::new(),
            MemoryStore::new(),
        ),
    }
}

#[cfg(not(miri))]
mod mock_store {
    use super::support::MockStore;

    tower_sesh_test::test_suite! {
        store: MockStore::new(),
    }
}
