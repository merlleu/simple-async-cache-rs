use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::Hash;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::Mutex;
use tokio::time::sleep;

pub struct InnerCacheLayer<K, V> {
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
    pub fn new(expire: u64) -> Arc<Self> {
        if expire < 3 {
            panic!("'expire' shouldn't be lower than 2.")
        }

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
        tokio::spawn(async move {
            let mut n = first_refresh;
            loop {
                let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64;
                if (n + expire) * 1000 < now {
                    sleep(Duration::from_secs(1)).await;
                    continue;
                }
                sleep(Duration::from_millis((n + expire) * 1000 - now)).await;

                let mut lock = cloned.inner.lock().await;
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


    /// Fetch the key from the cache.
    /// Returns an [`Arc`] to the [`Mutex`] for the key containing an Option.
    /// The [`Mutex`] prevents DogPile effect.
    /// 
    /// ```rust
    /// let cache = store.get("key_1".to_string()).await;
    /// let mut result = cache.lock().await;
    /// match &mut *result {
    ///     Some(val) => {
    ///         // You can  get here the cached value for key_1 if it is already available.
    ///     }
    ///     None => {
    ///         // There is no existing entry for key_1, you can do any expansive task to get the value and store it then.
    ///         *result = Some("This is the content for key_1.");
    ///     }
    /// }
    /// ```
    pub async fn get(&self, key: K) -> Arc<Mutex<Option<V>>> {
        let mut lock = self.inner.lock().await;
        let val = match lock.map.get(&key) {
            Some(v) => v.clone(),
            None => {
                let v = Arc::new(Mutex::new(None));
                lock.map.insert(key.clone(), v.clone());
                let current_time = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs();
                match lock.expiration_map.get_mut(&current_time) {
                    Some(arr) => {
                        arr.push(key);
                    }
                    None => {
                        lock.expiration_map.insert(current_time, vec![key]);
                    }
                }
                v
            }
        };

        val
    }
}
