pub mod cache;
pub mod config;
pub mod db;
pub mod errors;
pub mod jwt;
pub mod modules;

use axum::Router;
use std::sync::Arc;
use tower_http::{compression::CompressionLayer, cors::CorsLayer, trace::TraceLayer};
use tracing_subscriber::Layer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::modules::auth::repository::{PgUserRepository, UserRepository};

// Estado compartido que Axum inyecta en cada handler
#[derive(Clone)]
pub struct AppState {
    pub pool: Arc<sqlx::PgPool>,
    pub user_repo: Arc<dyn UserRepository>,
    pub cache: Arc<dyn cache::AppCache>,
    pub config: Arc<config::Config>,
}

pub async fn run() -> anyhow::Result<()> {
    // 1. Cargar configuración
    let config = config::Config::load()?;

    // 2. Configurar logging según entorno
    let fmt_layer = if config.is_production() {
        tracing_subscriber::fmt::layer().json().boxed()
    } else {
        tracing_subscriber::fmt::layer().pretty().boxed()
    };

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "pceo_backend=debug,tower_http=debug".into()),
        )
        .with(fmt_layer)
        .init();

    // 3. Pool de base de datos
    let pool = db::create_pool(&config.database_url).await?;
    tracing::info!("Conectado a PostgreSQL");

    // 4. Migraciones automáticas al arrancar
    sqlx::migrate!("./migrations").run(&pool).await?;
    tracing::info!("Migraciones aplicadas");

    // 5. Caché
    let cache = Arc::new(cache::MokaCache::new(1_000, 600));

    // 6. Estado
    let state = AppState {
        user_repo: Arc::new(PgUserRepository::new(pool.clone())),
        pool: Arc::new(pool),
        cache,
        config: Arc::new(config.clone()),
    };

    // 7. Router
    let app = Router::new()
        .merge(modules::routes())
        .layer(TraceLayer::new_for_http())
        .layer(CompressionLayer::new())
        .layer(CorsLayer::permissive())
        .with_state(state);

    // 8. Servidor
    let addr = format!("0.0.0.0:{}", config.server_port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("Servidor escuchando en {}", addr);

    axum::serve(listener, app).await?;
    Ok(())
}

pub async fn create_test_app(db_url: &str) -> axum::Router {
    let pool = db::create_pool(db_url)
        .await
        .expect("No se pudo conectar a la BBDD de test");

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Migraciones fallaron");

    let pool = Arc::new(pool);
    let user_repo = Arc::new(PgUserRepository::new((*pool).clone()));

    let state = AppState {
        pool,
        user_repo,
        cache: Arc::new(cache::MokaCache::new(100, 60)),
        config: Arc::new(config::Config::load().expect("Config inválida")),
    };

    modules::routes().with_state(state)
}
