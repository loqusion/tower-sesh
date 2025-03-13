test *FLAGS:
    #!/usr/bin/env bash
    set -euxo pipefail

    REDIS_CONTAINER_ID=$(docker run --detach --publish 127.0.0.1:6379:6379 redis:7.4.1-alpine)
    finish() {
        docker stop --time 1 "$REDIS_CONTAINER_ID" >/dev/null
    }
    trap finish EXIT

    REDIS_URL="redis://localhost:6379" cargo nextest run --workspace --features test-util {{FLAGS}}

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
