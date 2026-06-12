use dotenvy::dotenv;
use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub jwt_secret: String,
    pub jwt_access_ttl_seconds: u64,
    pub server_port: u16,
    pub rust_env: Environment,
    pub smtp_host: String,
    pub smtp_port: u16,
    pub smtp_user: String,
    pub smtp_password: String,
    pub smtp_from: String,
    pub base_url: String,
    pub scraper_url: String,
    pub scraper_min_sessions: usize,
    pub auto_select_max_concurrent: usize,
    pub allowed_origin: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Environment {
    Development,
    Production,
}

impl Config {
    pub fn load() -> anyhow::Result<Self> {
        let _ = dotenv();

        Ok(Self {
            database_url: required("DATABASE_URL")?,
            jwt_secret: required("JWT_SECRET")?,
            jwt_access_ttl_seconds: env::var("JWT_ACCESS_TTL_SECONDS")
                .unwrap_or_else(|_| "900".into())
                .parse()?,
            server_port: env::var("SERVER_PORT")
                .unwrap_or_else(|_| "3001".into())
                .parse()?,
            rust_env: match env::var("RUST_ENV")
                .unwrap_or_else(|_| "development".into())
                .as_str()
            {
                "production" => Environment::Production,
                _ => Environment::Development,
            },
            smtp_host: required("SMTP_HOST")?,
            smtp_port: env::var("SMTP_PORT")
                .unwrap_or_else(|_| "587".into())
                .parse()?,
            smtp_user: required("SMTP_USER")?,
            smtp_password: required("SMTP_PASSWORD")?,
            smtp_from: required("SMTP_FROM")?,
            base_url: required("BASE_URL")?,
            scraper_url: required("SCRAPER_URL")?,
            scraper_min_sessions: env::var("SCRAPER_MIN_SESSIONS")
                .unwrap_or_else(|_| "1000".into())
                .parse()?,
            auto_select_max_concurrent: env::var("AUTO_SELECT_MAX_CONCURRENT")
                .unwrap_or_else(|_| "5".into())
                .parse()?,
            allowed_origin: env::var("ALLOWED_ORIGIN")
                .unwrap_or_else(|_| "http://localhost:3000".into()),
        })
    }

    pub fn is_production(&self) -> bool {
        self.rust_env == Environment::Production
    }
}

fn required(key: &str) -> anyhow::Result<String> {
    env::var(key).map_err(|_| anyhow::anyhow!("{key} es requerido pero no se encontro"))
}
