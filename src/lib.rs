pub mod cache;
pub mod config;
pub mod db;
pub mod errors;
pub mod jwt;
pub mod metrics;
pub mod modules;
pub mod seed;
pub mod services;
pub mod utils;

use axum::Router;
use axum::http::{HeaderValue, Method, header};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::Semaphore;
use tokio_cron_scheduler::{Job, JobScheduler};
use tower_http::{compression::CompressionLayer, cors::CorsLayer, trace::TraceLayer};
use tracing_subscriber::Layer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::modules::classes::repository::{ClassRepository, PgClassRepository};
use crate::modules::health::repository::{HealthRepository, PgHealthRepository};
use crate::modules::notifications::repository::{NotificationRepository, PgNotificationRepository};
use crate::modules::notifications::service::{EmailQueue, run_email_worker};
use crate::modules::proposals::repository::{PgProposalRepository, ProposalRepository};
use crate::modules::schedule::repository::{PgScheduleRepository, ScheduleRepository};
use crate::modules::scraper::repository::{PgScraperRepository, ScraperRepository};
use crate::modules::scraper::service::run_sync;
use crate::modules::subjects::repository::{PgSubjectRepository, SubjectRepository};
use crate::modules::user_metrics::repository::{PgUserMetricsRepository, UserMetricsRepository};
use crate::modules::users::repository::{PgUserRepository, UserRepository};
use crate::services::email::service::{EmailService, MockEmailService, SmtpEmailService};

// Estado compartido que Axum inyecta en cada handler
#[derive(Clone)]
pub struct AppState {
    pub pool: Arc<sqlx::PgPool>,
    pub health_repo: Arc<dyn HealthRepository>,
    pub user_repo: Arc<dyn UserRepository>,
    pub schedule_repo: Arc<dyn ScheduleRepository>,
    pub class_repo: Arc<dyn ClassRepository>,
    pub proposals_repo: Arc<dyn ProposalRepository>,
    pub subjects_repo: Arc<dyn SubjectRepository>,
    pub scraper_repo: Arc<dyn ScraperRepository>,
    pub user_metrics_repo: Arc<dyn UserMetricsRepository>,
    pub notifications_repo: Arc<dyn NotificationRepository>,
    pub email: Arc<dyn EmailService>,
    pub email_queue: EmailQueue,
    pub cache: Arc<dyn cache::AppCache>,
    pub config: Arc<config::Config>,
    pub auto_select_semaphore: Arc<Semaphore>,
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

    // 5.5 Cola de emails de notificación (worker en background)
    let (email_queue, email_rx) = EmailQueue::new(1_000);
    tokio::spawn(run_email_worker(email_rx, email.clone()));

    // 6. Caché
    let cache = Arc::new(cache::MokaCache::new(1_000, 600));

    let auto_select_semaphore = Arc::new(Semaphore::new(config.auto_select_max_concurrent));

    // 6.5 Levantamos prometheus
    let prometheus_handle = metrics::setup_recorder();

    // 7. Estado
    let state = AppState {
        health_repo: Arc::new(PgHealthRepository::new(pool.clone())),
        user_repo: Arc::new(PgUserRepository::new(pool.clone())),
        class_repo: Arc::new(PgClassRepository::new(pool.clone())),
        proposals_repo: Arc::new(PgProposalRepository::new(pool.clone())),
        schedule_repo: Arc::new(PgScheduleRepository::new(pool.clone())),
        subjects_repo: Arc::new(PgSubjectRepository::new(pool.clone())),
        scraper_repo: Arc::new(PgScraperRepository::new(pool.clone())),
        user_metrics_repo: Arc::new(PgUserMetricsRepository::new(pool.clone())),
        notifications_repo: Arc::new(PgNotificationRepository::new(pool.clone())),
        pool: Arc::new(pool),
        email,
        email_queue,
        cache,
        config: Arc::new(config.clone()),
        auto_select_semaphore,
    };

    // 8. Arrancar el scheduduler del cronjob
    start_scraper_scheduler(state.clone()).await?;

    // 9. Router
    let cors = CorsLayer::new()
        .allow_origin(config.allowed_origin.parse::<HeaderValue>().unwrap())
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PATCH,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers([header::CONTENT_TYPE])
        .allow_credentials(true);

    let app = Router::new()
        .merge(modules::routes(true))
        .layer(axum::middleware::from_fn(metrics::track_http_metrics))
        .layer(TraceLayer::new_for_http())
        .layer(CompressionLayer::new())
        .layer(cors)
        .with_state(state);

    // 9.5 Anidamos el prometheus a un router interno
    let metrics_app = metrics::metrics_router(prometheus_handle);

    // 10. Servidor
    let addr = format!("0.0.0.0:{}", config.server_port);
    let metrics_addr = format!("0.0.0.0:{}", config.metrics_port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("Servidor escuchando en {}", addr);
    let metrics_listener = tokio::net::TcpListener::bind(&metrics_addr).await?;
    tracing::info!("Métricas (interno) escuchando en {}", metrics_addr);

    tokio::try_join!(
        axum::serve(
            listener,
            app.into_make_service_with_connect_info::<SocketAddr>()
        ),
        axum::serve(metrics_listener, metrics_app),
    )?;
    Ok(())
}

/// Configura un cronjob que ejecuta el scraper todos los días a las 5 AM
async fn start_scraper_scheduler(state: AppState) -> anyhow::Result<()> {
    let scheduler = JobScheduler::new().await?;

    let job = Job::new_async("0 0 5 * * *", move |_uuid, _lock| {
        let state = state.clone();
        Box::pin(async move {
            tracing::info!("Cronjob del scraper iniciado");

            let full_scraper_url = format!("{}/scrape", state.config.scraper_url);

            match run_sync(
                state.scraper_repo.as_ref(),
                state.notifications_repo.as_ref(),
                &state.email_queue,
                &full_scraper_url,
                state.config.scraper_min_sessions,
                "cronjob",
            )
            .await
            {
                Ok(result) if result.aborted => {
                    tracing::warn!(reason = ?result.abort_reason, "Cronjob del scraper abortado");
                }
                Ok(result) => {
                    tracing::info!(
                        sessions_inserted = result.sessions_inserted,
                        changes_applied = result.changes_applied,
                        pending_rejected = result.pending_rejected,
                        rejected_archived = result.rejected_archived,
                        "Cronjob del scraper finalizado exitosamente"
                    );
                }
                Err(e) => {
                    tracing::error!(error = ?e, "Error ejecutando cronjob del scraper");
                }
            }
        })
    })?;

    scheduler.add(job).await?;
    scheduler.start().await?;

    tracing::info!("Scheduler del scraper iniciado, próxima ejecución a las 5 AM");
    Ok(())
}

pub async fn create_test_app(db_url: &str) -> (axum::Router, sqlx::PgPool) {
    create_test_app_with_scraper(db_url, "http://localhost:4000").await
}

pub async fn create_test_app_with_scraper(
    db_url: &str,
    scraper_url: &str,
) -> (axum::Router, sqlx::PgPool) {
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
    let scraper_repo = Arc::new(PgScraperRepository::new((*pool).clone()));
    let user_metrics_repo = Arc::new(PgUserMetricsRepository::new((*pool).clone()));
    let notifications_repo = Arc::new(PgNotificationRepository::new((*pool).clone()));
    let email: Arc<dyn EmailService> = Arc::new(MockEmailService);
    let (email_queue, email_rx) = EmailQueue::new(1_000);
    tokio::spawn(run_email_worker(email_rx, email.clone()));
    let state = AppState {
        pool: pool.clone(),
        health_repo: Arc::new(PgHealthRepository::new((*pool).clone())),
        user_repo,
        class_repo,
        proposals_repo,
        schedule_repo,
        subjects_repo: subject_repo,
        scraper_repo,
        user_metrics_repo,
        notifications_repo,
        email,
        email_queue,
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
            scraper_url: scraper_url.to_string(),
            scraper_min_sessions: 1000,
            auto_select_max_concurrent: 5,
            allowed_origin: "http://localhost:3000".to_string(),
            metrics_port: 9090,
        }),
        auto_select_semaphore: Arc::new(Semaphore::new(5)),
    };

    let app = modules::routes(false).with_state(state);
    let pool_for_tests = (*pool).clone();
    (app, pool_for_tests)
}
