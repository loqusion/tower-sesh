#![cfg(feature = "test-util")]

use std::time::Duration;

use redis::aio::ConnectionManagerConfig;
use serde::{de::DeserializeOwned, Serialize};
use tower_sesh_store_redis::RedisStore;

const REDIS_IMAGE: &str = "redis:7.4.2-alpine";
const VALKEY_IMAGE: &str = "valkey/valkey:8.1.0-alpine";

mod container {
    use std::mem;

    use anyhow::Context;
    use tower_sesh_core::util::Report;
    use xshell::{cmd, Shell};

    pub fn run(image: &str) -> anyhow::Result<ContainerGuard> {
        #[derive(Clone, Debug)]
        struct Cleanup<'a> {
            shell: &'a Shell,
            id: &'a str,
        }
        impl Cleanup<'_> {
            fn with(self, port: u16) -> ContainerGuard {
                let guard = ContainerGuard {
                    shell: self.shell.to_owned(),
                    id: self.id.to_owned(),
                    port,
                };

                mem::forget(self);

                guard
            }
        }
        impl Drop for Cleanup<'_> {
            fn drop(&mut self) {
                stop(self.shell, self.id);
            }
        }

        let sh = Shell::new()?;

        let run_opts = [
            "--detach",
            "--publish",
            "127.0.0.1::6379/tcp", // publish the exposed port to a random host port
            "--rm",
            "--stop-timeout",
            "60",
            "--health-cmd",
            "redis-cli ping",
            "--health-interval",
            "500ms",
            "--health-timeout",
            "500ms",
            "--health-retries",
            "3",
        ];
        let id = cmd!(sh, "docker container run {run_opts...} {image}").read()?;

        // If we return early, this cleans up the running container
        let guard = Cleanup {
            shell: &sh,
            id: &id,
        };

        let inspect_opts = [
            "--format",
            r#"{{ (index (index .NetworkSettings.Ports "6379/tcp") 0).HostPort }}"#,
        ];
        let port = cmd!(sh, "docker container inspect {inspect_opts...} {id}").read()?;
        let port = port
            .parse()
            .with_context(|| format!("failed to parse port number: {port}"))?;

        Ok(guard.with(port))
    }

    fn stop(sh: &Shell, id: &str) {
        fn _stop(sh: &Shell, id: &str) -> xshell::Result<()> {
            cmd!(sh, "docker container stop --timeout 1 {id}")
                .quiet()
                .ignore_stdout()
                .run()?;

            Ok(())
        }

        if let Err(err) = _stop(sh, id) {
            eprintln!("{}", Report::new(err));
        }
    }

    #[must_use = "if unused the docker container will immediately be stopped"]
    #[derive(Clone, Debug)]
    pub struct ContainerGuard {
        shell: Shell,
        id: String,
        pub port: u16,
    }

    impl Drop for ContainerGuard {
        fn drop(&mut self) {
            stop(&self.shell, &self.id);
        }
    }
}

async fn store<T>(url: &str) -> RedisStore<T>
where
    T: Serialize + DeserializeOwned + Send + Sync + 'static,
{
    let config = ConnectionManagerConfig::new()
        .set_connection_timeout(Duration::from_secs(5))
        .set_number_of_retries(1);

    RedisStore::with_config(url, config)
        .await
        .expect("failed to connect to redis")
}

mod redis_store {
    use tower_sesh_test::test_suite;

    use super::{container, store, REDIS_IMAGE};

    test_suite! {
        guard: container = container::run(REDIS_IMAGE).unwrap(),
        store: store(&format!("redis://localhost:{}", container.port)).await,
    }
}

mod valkey_store {
    use tower_sesh_test::test_suite;

    use super::{container, store, VALKEY_IMAGE};

    test_suite! {
        guard: container = container::run(VALKEY_IMAGE).unwrap(),
        store: store(&format!("redis://localhost:{}", container.port)).await,
    }
}

mod redis_caching_store {
    use tower_sesh::store::{CachingStore, MemoryStore};
    use tower_sesh_test::test_suite;

    use super::{container, store, REDIS_IMAGE};

    test_suite! {
        guard: container = container::run(REDIS_IMAGE).unwrap(),
        store: CachingStore::from_cache_and_store(
            MemoryStore::new(),
            store(&format!("redis://localhost:{}", container.port)).await,
        ),
    }
}

mod valkey_caching_store {
    use tower_sesh::store::{CachingStore, MemoryStore};
    use tower_sesh_test::test_suite;

    use super::{container, store, VALKEY_IMAGE};

    test_suite! {
        guard: container = container::run(VALKEY_IMAGE).unwrap(),
        store: CachingStore::from_cache_and_store(
            MemoryStore::new(),
            store(&format!("redis://localhost:{}", container.port)).await,
        ),
    }
}
