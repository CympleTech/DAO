use config::Config as CFG;
use dotenv::dotenv;
use once_cell::sync::OnceCell;
use serde::Deserialize;
use sqlx::postgres::PgPoolOptions;
use sqlx::{PgPool, Pool, Postgres};
use std::env;
use tdn::types::primitive::{new_io_error, Result};

#[derive(Debug, Deserialize)]
struct Config {
    database: String,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        let mut cfg = CFG::new();

        // database url.
        let database = env::var("DATABASE_URL").expect("DATABASE_URL missing");

        cfg.set("database", database)
            .map_err(|_| new_io_error("set config error."))?;

        // others.
        for (key, value) in env::vars() {
            cfg.set(&key, value)
                .map_err(|_| new_io_error("set config error."))?;
        }
        cfg.try_into().map_err(|_| new_io_error("config error."))
    }
}

pub static INSTANCE: OnceCell<DB> = OnceCell::new();

#[inline]
pub fn get_pool<'a>() -> Result<&'a PgPool> {
    INSTANCE
        .get()
        .map(|db| &db.pool)
        .ok_or(new_io_error("DB error!"))
}

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
