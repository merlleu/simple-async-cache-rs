# simple-async-cache-rs

A fast asynchronous caching crate with expiration delay and custom types.

## Links
* [Repository](https://github.com/merlleu/simple-async-cache-rs)
* [Documentation](https://docs.rs/simple-async-cache-rs)
* [Crates.io](https://crates.io/crates/simple-async-cache-rs)

## Why would you use this crate ?
* You can configure expiration delay
* It supports almost any types / structs
* It's really simple and written in pure Rust
* It prevents [dogpile effect](https://www.sobstel.org/blog/preventing-dogpile-effect/)
* Its only dependency is tokio.
* It's thread-safe and fast

## Basic example:
```rs
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
```

