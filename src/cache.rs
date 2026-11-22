use std::{
    collections::HashMap,
    future::Future,
    hash::Hash,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use anyhow::Result;

pub const TTL: Duration = Duration::from_secs(24 * 60 * 60);

/// A value that is reloaded once it is older than [`TTL`], with concurrent loads coalesced.
pub struct Cached<T> {
    slot: tokio::sync::Mutex<Option<(Instant, Arc<T>)>>,
}

impl<T> Default for Cached<T> {
    fn default() -> Self {
        Self {
            slot: Default::default(),
        }
    }
}

impl<T> Cached<T> {
    pub async fn get<F, Fut>(&self, load: F) -> Result<Arc<T>>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<T>>,
    {
        let mut slot = self.slot.lock().await;
        if let Some((at, v)) = slot.as_ref() {
            if at.elapsed() < TTL {
                return Ok(v.clone());
            }
        }
        let v = Arc::new(load().await?);
        *slot = Some((Instant::now(), v.clone()));
        Ok(v)
    }
}

/// Successful lookups by key that expire after [`TTL`]. Failures are not stored.
pub struct Memo<K, V> {
    entries: Mutex<HashMap<K, (Instant, V)>>,
}

impl<K, V> Default for Memo<K, V> {
    fn default() -> Self {
        Self {
            entries: Default::default(),
        }
    }
}

impl<K: Eq + Hash + Clone, V: Clone> Memo<K, V> {
    pub async fn get<Fut>(&self, key: &K, load: impl FnOnce() -> Fut) -> Result<V>
    where
        Fut: Future<Output = Result<V>>,
    {
        {
            let mut entries = self.entries.lock().unwrap();
            entries.retain(|_, (at, _)| at.elapsed() < TTL);
            if let Some((_, v)) = entries.get(key) {
                return Ok(v.clone());
            }
        }
        let v = load().await?;
        self.entries
            .lock()
            .unwrap()
            .insert(key.clone(), (Instant::now(), v.clone()));
        Ok(v)
    }
}
