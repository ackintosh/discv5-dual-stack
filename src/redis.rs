use redis::aio::MultiplexedConnection;
use redis::{AsyncCommands, Client};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::collections::HashMap;
use std::time::Duration;

pub(crate) struct RedisClient {
    inner: MultiplexedConnection,
}

impl RedisClient {
    pub(crate) async fn new() -> Self {
        let client = Client::open("redis://redis:6379/").unwrap();
        let connection = client.get_multiplexed_async_connection().await.unwrap();

        RedisClient { inner: connection }
    }

    /// Push an element at the tail of the list
    pub(crate) async fn push<V: Serialize + DeserializeOwned>(&mut self, key: &str, value: V) {
        let _: () = self
            .inner
            .rpush(key, serde_json::to_string(&value).unwrap())
            .await
            .unwrap();
    }

    /// Pop the first element in a list, or block until one is available.
    pub(crate) async fn pop<V: DeserializeOwned>(&mut self, key: &str) -> V {
        let mut value = self
            .inner
            .blpop::<_, HashMap<String, String>>(key, 0_f64)
            .await
            .unwrap();
        serde_json::from_str(value.remove(key).unwrap().as_str()).unwrap()
    }

    /// Signal an entry on the given key and then wait until the specified value has been reached.
    pub(crate) async fn signal_and_wait(&mut self, key: &str, target: u64) {
        let mut count: u64 = self.inner.incr(key, 1_u64).await.unwrap();

        loop {
            if count >= target {
                break;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
            count = self.inner.get(key).await.unwrap();
        }
    }

    pub(crate) async fn remove(&mut self, key: &str) {
        self.inner.del::<&str, String>(key).await.unwrap();
    }
}
