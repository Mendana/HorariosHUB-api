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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn get_clave_inexistente_devuelve_none() {
        let cache = MokaCache::new(100, 60);
        assert_eq!(cache.get("no_existe").await, None);
    }

    #[tokio::test]
    async fn set_y_get_devuelve_valor() {
        let cache = MokaCache::new(100, 60);
        cache.set("clave", "valor".to_string()).await;
        assert_eq!(cache.get("clave").await, Some("valor".to_string()));
    }

    #[tokio::test]
    async fn set_sobreescribe_valor_existente() {
        let cache = MokaCache::new(100, 60);
        cache.set("clave", "primero".to_string()).await;
        cache.set("clave", "segundo".to_string()).await;
        assert_eq!(cache.get("clave").await, Some("segundo".to_string()));
    }

    #[tokio::test]
    async fn invalidate_elimina_clave() {
        let cache = MokaCache::new(100, 60);
        cache.set("clave", "valor".to_string()).await;
        cache.invalidate("clave").await;
        assert_eq!(cache.get("clave").await, None);
    }

    #[tokio::test]
    async fn invalidate_clave_inexistente_no_falla() {
        let cache = MokaCache::new(100, 60);
        cache.invalidate("no_existe").await;
    }

    #[tokio::test]
    async fn invalidate_prefix_elimina_claves_con_prefijo() {
        let cache = MokaCache::new(100, 60);
        cache.set("user:1:horario", "a".to_string()).await;
        cache.set("user:1:perfil", "b".to_string()).await;
        cache.set("user:2:horario", "c".to_string()).await;
        cache.invalidate_prefix("user:1").await;
        assert!(cache.get("user:1:horario").await.is_none());
        assert!(cache.get("user:1:perfil").await.is_none());
        assert!(cache.get("user:2:horario").await.is_some());
    }

    #[tokio::test]
    async fn invalidate_prefix_sin_coincidencias_no_falla() {
        let cache = MokaCache::new(100, 60);
        cache.set("user:1:data", "a".to_string()).await;
        cache.invalidate_prefix("schedule").await;
        assert_eq!(cache.get("user:1:data").await, Some("a".to_string()));
    }
}
