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
    /// 
    /// 
    /// 
    pub fn new(expire: u64) -> Arc<Self> {
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
