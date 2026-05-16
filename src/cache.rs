use async_trait::async_trait;
use moka::future::Cache;
use std::time::Duration;

//TODO: sustituir en un futuro por una implementación de Redis o similar, para compartir cache entre instancias
#[async_trait]
pub trait AppCache: Send + Sync {
    async fn get(&self, key: &str) -> Option<String>;
    async fn set(&self, key: &str, value: String);
    async fn invalidate(&self, key: &str);
    async fn invalidate_prefix(&self, prefix: &str);
}

pub struct MokaCache {
    inner: Cache<String, String>,
}

impl MokaCache {
    pub fn new(max_capacity: u64, ttl_seconds: u64) -> Self {
        Self {
            inner: Cache::builder()
                .max_capacity(max_capacity)
                .time_to_live(Duration::from_secs(ttl_seconds))
                .build(),
        }
    }
}

#[async_trait]
impl AppCache for MokaCache {
    async fn get(&self, key: &str) -> Option<String> {
        self.inner.get(key).await
    }

    async fn set(&self, key: &str, value: String) {
        self.inner.insert(key.to_string(), value).await;
    }

    async fn invalidate(&self, key: &str) {
        self.inner.invalidate(key).await;
    }

    // Invalidar un horario si cambia
    async fn invalidate_prefix(&self, prefix: &str) {
        let keys_to_remove: Vec<String> = self
            .inner
            .iter()
            .filter(|(k, _)| k.starts_with(prefix))
            .map(|(k, _)| k.as_ref().clone())
            .collect();

        for key in keys_to_remove {
            self.inner.invalidate(&key).await;
        }
    }
}
