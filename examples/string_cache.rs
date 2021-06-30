use simple_async_cache_rs::AsyncCacheStore;
use std::time::Duration;
use tokio::time::sleep;


#[tokio::main]
async fn main() {
    // Create an AsyncCacheStore using implicit typing with an expiration delay of 10 seconds.
    let mut store = AsyncCacheStore::new(10);

    // If you want to explicitly define types you can do the following:
    // let mut store: Arc<AsyncCacheStore<String, String>> = AsyncCacheStore::new(60);

    let cache = store.get("key_1".to_string()).await;
    let mut result = cache.lock().await;
    match &mut *result {
        Some(_d) => {
            // You can  get here the cached value for key_1 if it is already available.
        }
        None => {
            // There is no existing entry for key_1, you can do any task to get the value.
            // The AsyncCacheStore prevents dogpile effect by itself.
            *result = Some("This is the first value for key_1.");
        }
    }

    // The value for key_1 is still cached.
    assert_eq!(
        *store.get("key_1".to_string()).await.lock().await,
        Some("This is the first value for key_1.")
    );

    // We sleep for 15 seconds, the value for key_1 is expired.
    sleep(Duration::from_secs(15)).await;

    assert_eq!(
        *store.get("key_1".to_string()).await.lock().await,
        None
    );
}