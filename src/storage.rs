use dotenv::dotenv;
use std::env;
use serde::Deserialize;
use config::{Config as CFG, ConfigError};
use once_cell::sync::OnceCell;
use sqlx::{Pool, Postgres};
use sqlx::postgres::PgPoolOptions;

#[derive(Debug, Deserialize)]
struct Config {
    pool: String
}

impl Config {
    pub fn from_env() -> Result<Self, ConfigError> {
        let mut cfg = CFG::new();
        for (key, value) in env::vars() {
            cfg.set(&key, value);
        };
        cfg.try_into()
    }
}

pub static INSTANCE: OnceCell<DB> = OnceCell::new();

pub struct DB {
    pub pool: Pool<Postgres>
}

impl DB {
    pub fn global() -> &'static DB {
        INSTANCE.get().expect("DB is not initialized")
    }

    async fn new() -> DB {
        let mut cfg = Config::from_env().unwrap();
        let pool = PgPoolOptions::new().max_connections(5).connect(&cfg.pool).await.unwrap();
        DB {
            pool
        }
    }
}

pub async fn init() {
    dotenv().ok();
    let db = DB::new().await;
    INSTANCE.set(db);
}
