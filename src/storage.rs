use config::{Config as CFG, ConfigError};
use dotenv::dotenv;
use once_cell::sync::OnceCell;
use serde::Deserialize;
use sqlx::postgres::PgPoolOptions;
use sqlx::{Pool, Postgres};
use std::env;

#[derive(Debug, Deserialize)]
struct Config {
    database: String,
}

impl Config {
    pub fn from_env() -> Result<Self, ConfigError> {
        let mut cfg = CFG::new();

        // database url.
        let database = env::var("DATABASE_URL").expect("DATABASE_URL missing");
        cfg.set("database", database)?;

        // others.
        for (key, value) in env::vars() {
            cfg.set(&key, value)?;
        }
        cfg.try_into()
    }
}

pub static INSTANCE: OnceCell<DB> = OnceCell::new();

pub struct DB {
    pub pool: Pool<Postgres>,
}

impl DB {
    pub fn global() -> &'static DB {
        INSTANCE.get().expect("DB is not initialized")
    }

    async fn new() -> DB {
        let cfg = Config::from_env().unwrap();
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(&cfg.database)
            .await
            .unwrap();
        DB { pool }
    }
}

pub async fn init() {
    dotenv().ok();
    let db = DB::new().await;
    let _ = INSTANCE.set(db);
}
