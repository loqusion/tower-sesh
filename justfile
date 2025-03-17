test *FLAGS:
    #!/usr/bin/env bash
    set -euxo pipefail

    REDIS_CONTAINER_ID=$(docker run --detach --publish 127.0.0.1:6379:6379 redis:7.4.1-alpine)
    finish() {
        docker stop --time 1 "$REDIS_CONTAINER_ID" >/dev/null
    }
    trap finish EXIT

    REDIS_URL="redis://localhost:6379" cargo nextest run --workspace --features test-util {{FLAGS}}

test-caching-store *FLAGS:
    #!/usr/bin/env bash
    set -euxo pipefail

    REDIS_CONTAINER_ID=$(docker run --detach --publish 127.0.0.1:6379:6379 redis:7.4.1-alpine)
    finish() {
        docker stop --time 1 "$REDIS_CONTAINER_ID" >/dev/null
    }
    trap finish EXIT

    RUSTFLAGS="--cfg tower_sesh_test_caching_store" REDIS_URL="redis://localhost:6379" \
        cargo nextest run --features test-util {{FLAGS}}

test-miri-lib *FLAGS:
    MIRIFLAGS='-Zmiri-disable-isolation -Zmiri-strict-provenance' \
        cargo +nightly miri nextest run --workspace --lib --features test-util {{FLAGS}}

test-miri-tests *FLAGS:
    MIRIFLAGS='-Zmiri-disable-isolation -Zmiri-strict-provenance' \
        cargo +nightly miri nextest run --package tower-sesh --tests --features test-util {{FLAGS}}

test-miri-doc *FLAGS:
    MIRIFLAGS='-Zmiri-disable-isolation -Zmiri-strict-provenance' \
        cargo +nightly miri test --workspace --doc --all-features {{FLAGS}}

doctest *FLAGS:
    cargo test --workspace --doc --all-features {{FLAGS}}

bench *FLAGS:
    #!/usr/bin/env bash
    set -euxo pipefail

    REDIS_CONTAINER_ID=$(docker run --detach --publish 127.0.0.1:6379:6379 redis:7.4.1-alpine)
    finish() {
        docker stop --time 1 "$REDIS_CONTAINER_ID" >/dev/null
    }
    trap finish EXIT

    REDIS_URL="redis://localhost:6379" cargo bench -q --features full {{FLAGS}}

doc *FLAGS:
    RUSTDOCFLAGS="--cfg docsrs --cfg tower_sesh_docs_local" cargo +nightly doc --all-features {{FLAGS}}
