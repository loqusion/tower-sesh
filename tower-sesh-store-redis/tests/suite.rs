#![cfg(feature = "test-util")]

use std::{mem, time::Duration};

use redis::aio::ConnectionManagerConfig;
use serde::{de::DeserializeOwned, Serialize};
use tower_sesh_core::util::Report;
use tower_sesh_store_redis::RedisStore;
use xshell::{cmd, Shell};

const REDIS_IMAGE: &str = "redis:7.4.2-alpine";

fn redis_init() -> anyhow::Result<DockerRedisGuard> {
    #[derive(Clone, Debug)]
    struct Cleanup<'a> {
        shell: &'a Shell,
        id: &'a str,
    }
    impl Cleanup<'_> {
        fn with(self, port: u16) -> DockerRedisGuard {
            let guard = DockerRedisGuard {
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
            stop_and_remove(&self.shell, &self.id);
        }
    }

    let sh = Shell::new()?;

    let run_opts = [
        "--detach",
        "--publish-all", // publish redis's exposed port to a random host port
        "--health-cmd",
        "redis-cli ping",
        "--health-interval",
        "10s",
        "--health-timeout",
        "5s",
        "--health-retries",
        "5",
    ];
    let id = cmd!(sh, "docker run {run_opts...} {REDIS_IMAGE}").read()?;

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
    let port = port.parse()?;

    Ok(guard.with(port))
}

fn stop_and_remove(sh: &Shell, id: &str) {
    fn _stop_and_remove(sh: &Shell, id: &str) -> xshell::Result<()> {
        cmd!(sh, "docker stop --timeout 1 {id}")
            .quiet()
            .ignore_stdout()
            .run()?;
        cmd!(sh, "docker rm {id}").quiet().ignore_stdout().run()?;

        Ok(())
    }

    if let Err(err) = _stop_and_remove(sh, id) {
        eprintln!("{}", Report::new(err));
    }
}

#[must_use = "if unused the docker container will immediately be stopped"]
#[derive(Clone, Debug)]
struct DockerRedisGuard {
    shell: Shell,
    id: String,
    port: u16,
}

impl Drop for DockerRedisGuard {
    fn drop(&mut self) {
        stop_and_remove(&self.shell, &self.id);
    }
}

async fn store<T>(url: String) -> RedisStore<T>
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

#[cfg(not(tower_sesh_test_caching_store))]
mod normal {
    use tower_sesh_test::test_suite;

    use super::{redis_init, store};

    test_suite! {
        guard: container = redis_init().unwrap(),
        store: store(format!("redis://localhost:{}", container.port)).await,
    }
}

#[cfg(tower_sesh_test_caching_store)]
mod with_caching_store {
    use tower_sesh::store::{CachingStore, MemoryStore};
    use tower_sesh_test::test_suite;

    use super::{redis_init, store};

    test_suite! {
        guard: container = redis_init().unwrap(),
        store: CachingStore::from_cache_and_store(
            MemoryStore::new(),
            store(format!("redis://localhost:{}", container.port)).await,
        ),
    }
}
