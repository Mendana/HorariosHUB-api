use dotenvy::dotenv;
use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub jwt_scret: String,
    pub jwt_access_ttl_seconds: u64,
    pub server_port: u16,
    pub rust_env: Environment,
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
            jwt_scret: required("JWT_SECRET")?,
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
        })
    }

    pub fn is_production(&self) -> bool {
        self.rust_env == Environment::Production
    }
}

fn required(key: &str) -> anyhow::Result<String> {
    env::var(key).map_err(|_| anyhow::anyhow!("{key} es requerido pero no se encontro"))
}
