mod support;

mod memory_store {
    use tower_sesh::store::MemoryStore;
    use tower_sesh_test::test_suite;

    test_suite! {
        store: MemoryStore::new(),
    }
}

mod memory_store_caching_store {
    use tower_sesh::store::{CachingStore, MemoryStore};
    use tower_sesh_test::test_suite;

    test_suite! {
        store: CachingStore::from_cache_and_store(
            MemoryStore::new(),
            MemoryStore::new(),
        ),
    }
}

#[cfg(not(miri))]
mod mock_store {
    use tower_sesh_test::test_suite;

    use super::support::MockStore;

    test_suite! {
        store: MockStore::new(),
    }
}
