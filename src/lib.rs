pub mod cache;
pub mod config;
pub mod db;
pub mod errors;
pub mod jwt;
pub mod modules;
pub mod seed;
pub mod services;
pub mod utils;

use axum::Router;
use std::sync::Arc;
use tower_http::{compression::CompressionLayer, cors::CorsLayer, trace::TraceLayer};
use tracing_subscriber::Layer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::modules::auth::repository::{PgUserRepository, UserRepository};
use crate::modules::classes::repository::{ClassRepository, PgClassRepository};
use crate::modules::proposals::repository::{PgProposalRepository, ProposalRepository};
use crate::modules::schedule::repository::{PgScheduleRepository, ScheduleRepository};
use crate::modules::subjects::repository::{PgSubjectRepository, SubjectRepository};
use crate::services::email::service::{EmailService, MockEmailService, SmtpEmailService};

// Estado compartido que Axum inyecta en cada handler
#[derive(Clone)]
pub struct AppState {
    pub pool: Arc<sqlx::PgPool>,
    pub user_repo: Arc<dyn UserRepository>,
    pub schedule_repo: Arc<dyn ScheduleRepository>,
    pub class_repo: Arc<dyn ClassRepository>,
    pub proposals_repo: Arc<dyn ProposalRepository>,
    pub subjects_repo: Arc<dyn SubjectRepository>,
    pub email: Arc<dyn EmailService>,
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

    // 4b. Seed de desarrollo
    if !config.is_production() {
        seed::run(&pool).await?;
    }

    // 5. Servicio de email
    let email = Arc::new(SmtpEmailService::new(
        &config.smtp_host,
        config.smtp_port,
        &config.smtp_user,
        &config.smtp_password,
        &config.smtp_from,
        &config.base_url,
    )?);

    // 6. Caché
    let cache = Arc::new(cache::MokaCache::new(1_000, 600));

    // 7. Estado
    let state = AppState {
        user_repo: Arc::new(PgUserRepository::new(pool.clone())),
        class_repo: Arc::new(PgClassRepository::new(pool.clone())),
        proposals_repo: Arc::new(PgProposalRepository::new(pool.clone())),
        schedule_repo: Arc::new(PgScheduleRepository::new(pool.clone())),
        subjects_repo: Arc::new(PgSubjectRepository::new(pool.clone())),
        pool: Arc::new(pool),
        email,
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

pub async fn create_test_app(db_url: &str) -> (axum::Router, sqlx::PgPool) {
    let pool = db::create_pool(db_url)
        .await
        .expect("No se pudo conectar a la BBDD de test");

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Migraciones fallaron");

    let pool = Arc::new(pool);
    let user_repo = Arc::new(PgUserRepository::new((*pool).clone()));
    let class_repo = Arc::new(PgClassRepository::new((*pool).clone()));
    let proposals_repo = Arc::new(PgProposalRepository::new((*pool).clone()));
    let schedule_repo = Arc::new(PgScheduleRepository::new((*pool).clone()));
    let subject_repo = Arc::new(PgSubjectRepository::new((*pool).clone()));
    let state = AppState {
        pool: pool.clone(),
        user_repo,
        class_repo,
        proposals_repo,
        schedule_repo,
        subjects_repo: subject_repo,
        email: Arc::new(MockEmailService),
        cache: Arc::new(cache::MokaCache::new(100, 60)),
        config: Arc::new(config::Config {
            database_url: db_url.to_string(),
            jwt_secret: "test-secret-key".to_string(),
            jwt_access_ttl_seconds: 900,
            server_port: 0,
            rust_env: config::Environment::Development,
            smtp_host: "localhost".to_string(),
            smtp_port: 1025,
            smtp_user: "test".to_string(),
            smtp_password: "test".to_string(),
            smtp_from: "no-reply@horarioshub.com".to_string(),
            base_url: "http://localhost:3000".to_string(),
        }),
    };

    let app = modules::routes().with_state(state);
    let pool_for_tests = (*pool).clone();
    (app, pool_for_tests)
}
