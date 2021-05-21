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
        cfg.try_into()
            .map_err(|_| new_io_error("config init error."))
    }
}

pub static INSTANCE: OnceCell<Pool<Postgres>> = OnceCell::new();

#[inline]
pub fn get_pool<'a>() -> Result<&'a PgPool> {
    INSTANCE.get().ok_or(new_io_error("DB get error!"))
}

pub async fn init() -> Result<()> {
    dotenv().ok();
    let cfg = Config::from_env()?;

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&cfg.database)
        .await
        .map_err(|_| new_io_error("DB postgres connect failure! check database & user/password"))?;

    INSTANCE
        .set(pool)
        .map_err(|_| new_io_error("DB set error!"))
}
