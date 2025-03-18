mod support;

mod memory_store {
    use tower_sesh::store::MemoryStore;

    tower_sesh_test::test_suite!(MemoryStore::new());
}

mod memory_store_caching_store {
    use tower_sesh::store::{CachingStore, MemoryStore};

    tower_sesh_test::test_suite!(CachingStore::from_cache_and_store(
        MemoryStore::new(),
        MemoryStore::new()
    ));
}

mod mock_store {
    use super::support::MockStore;

    tower_sesh_test::test_suite!(MockStore::new());
}
