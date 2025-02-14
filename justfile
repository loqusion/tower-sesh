test:
    #!/usr/bin/env bash
    set -euo pipefail

    REDIS_CONTAINER_ID=$(docker run --detach --publish 6379:6379 redis:7.4.1-alpine)
    finish() {
        docker stop --time 1 "$REDIS_CONTAINER_ID" >/dev/null
    }
    trap finish EXIT

    REDIS_URL="redis://localhost:6379" cargo test --workspace

doc *FLAGS:
    RUSTDOCFLAGS="--cfg docsrs" cargo +nightly doc --all-features {{FLAGS}}
