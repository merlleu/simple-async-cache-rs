use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::Hash;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::Mutex;

struct InnerCacheLayer<K, V> {
    pub map: HashMap<K, Arc<Mutex<Option<V>>>>,
    pub expiration_map: HashMap<u64, Vec<K>>,
}

pub struct AsyncCacheStore<K, V> {
    inner: Mutex<InnerCacheLayer<K, V>>,
}

impl<K: 'static + Eq + Hash + Debug + Sync + Send + Clone, V: 'static + Sync + Send>
    AsyncCacheStore<K, V>
{
    /// Construct a new [`AsyncCacheStore`] instance.
    /// Note: expire is the number of seconds for the cached value to expire.
    ///
    /// **Panic**:
    /// If you set expire to less than 3 seconds.
    /// This limitaion exists because we expire value only every seconds, meaning there could be desynchronizations with a TTL lower than 3.
    ///
    /// ```rust
    /// use simple_async_cache_rs::AsyncCacheStore;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let cache_ttl = 60; // number of seconds before the cached item is expired.
    ///     let store: AsyncCacheStore<u64, String> = AsyncCacheStore::new();
    /// }
    /// ```
    pub fn new() -> Arc<Self> {
        let a = Arc::new(AsyncCacheStore {
            inner: Mutex::new(InnerCacheLayer {
                map: HashMap::new(),
                expiration_map: HashMap::new(),
            }),
        });
        let cloned = a.clone();
        let first_refresh = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            - 1;

        let mut timer = tokio::time::interval(tokio::time::Duration::from_secs(1));

        tokio::spawn(async move {
            let mut n = first_refresh;
            loop {
                timer.tick().await;
                let mut lock = cloned.inner.lock().await;
                println!(
                    "{} {}",
                    SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_secs(),
                    n
                );
                match lock.expiration_map.remove(&n) {
                    Some(expired) => {
                        for item in expired {
                            lock.map.remove(&item);
                        }
                    }
                    None => {}
                }
                n += 1;
            }
        });
        a
    }

    /// Fetch the key from the cache or creates with the supplied TTL in seconds.
    /// Returns an [`std::sync::Arc`] to the [`tokio::sync::Mutex`] for the key containing an Option.
    /// The [`tokio::sync::Mutex`] prevents DogPile effect.
    ///
    /// ```rust
    /// let cache = store.get("key_1".to_string(), 10).await;
    /// let mut result = cache.lock().await;
    /// match &mut *result {
    ///     Some(val) => {
    ///         // You can  get here the cached value for key_1 if it is already available.
    ///     }
    ///     None => {
    ///         // There is no existing entry for key_1, you can do any expansive task to get the value and store it then.
    ///         *result = Some("This is the content for key_1.".to_string());
    ///     }
    /// }
    /// ```
    pub async fn get(&self, key: K, ttl: u64) -> Arc<Mutex<Option<V>>> {
        let mut lock = self.inner.lock().await;
        match lock.map.get(&key) {
            Some(v) => v.clone(),
            None => {
                let v = Arc::new(Mutex::new(None));
                lock.map.insert(key.clone(), v.clone());
                lock.expiration_map
                    .entry(
                        SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap()
                            .as_secs()
                            + ttl,
                    )
                    .or_default()
                    .push(key);

                v
            }
        }
    }

    pub async fn exists(&self, key: K) -> bool {
        let lock = self.inner.lock().await;
        lock.map.get(&key).is_some()
    }

    pub async fn ready(&self, key: K) -> bool {
        let lock = self.inner.lock().await;
        match lock.map.get(&key) {
            Some(v) => v.lock().await.is_some(),
            None => false,
        }
    }

    /// Expire immediatly the an item from the cache.
    pub async fn expire(&self, key: &K) {
        let mut lock = self.inner.lock().await;
        lock.map.remove(key);
    }
}
