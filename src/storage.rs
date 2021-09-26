use config::Config as CFG;
use dotenv::dotenv;
use image::{load_from_memory, DynamicImage, GenericImageView};
use once_cell::sync::OnceCell;
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use serde::Deserialize;
use sqlx::postgres::PgPoolOptions;
use sqlx::{PgPool, Pool, Postgres};
use std::env;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use tdn::types::{group::GroupId, primitive::Result};
use tokio::fs;

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
            .map_err(|_| anyhow!("set config error."))?;

        // others.
        for (key, value) in env::vars() {
            cfg.set(&key, value)
                .map_err(|_| anyhow!("set config error."))?;
        }
        cfg.try_into().map_err(|_| anyhow!("config init error."))
    }
}

pub static INSTANCE: OnceCell<Pool<Postgres>> = OnceCell::new();

#[inline]
pub fn get_pool<'a>() -> Result<&'a PgPool> {
    INSTANCE.get().ok_or(anyhow!("DB get error!"))
}

pub async fn init() -> Result<()> {
    dotenv().ok();
    let cfg = Config::from_env()?;

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&cfg.database)
        .await
        .map_err(|_| anyhow!("DB postgres connect failure! check database & user/password"))?;

    INSTANCE.set(pool).map_err(|_| anyhow!("DB set error!"))
}

const FILES_DIR: &'static str = "files";
const IMAGE_DIR: &'static str = "images";
const THUMB_DIR: &'static str = "thumbs";
const EMOJI_DIR: &'static str = "emojis";
const RECORD_DIR: &'static str = "records";
const AVATAR_DIR: &'static str = "avatars";

pub(crate) async fn init_local_files(base: &PathBuf, gid: &GroupId) -> Result<()> {
    let mut home = base.clone();
    home.push(gid.to_hex());

    let mut files_path = home.clone();
    files_path.push(FILES_DIR);
    if !files_path.exists() {
        fs::create_dir_all(files_path).await?;
    }
    let mut image_path = home.clone();
    image_path.push(IMAGE_DIR);
    if !image_path.exists() {
        fs::create_dir_all(image_path).await?;
    }
    let mut thumb_path = home.clone();
    thumb_path.push(THUMB_DIR);
    if !thumb_path.exists() {
        fs::create_dir_all(thumb_path).await?;
    }
    let mut emoji_path = home.clone();
    emoji_path.push(EMOJI_DIR);
    if !emoji_path.exists() {
        fs::create_dir_all(emoji_path).await?;
    }
    let mut record_path = home.clone();
    record_path.push(RECORD_DIR);
    if !record_path.exists() {
        fs::create_dir_all(record_path).await?;
    }
    let mut avatar_path = home.clone();
    avatar_path.push(AVATAR_DIR);
    if !avatar_path.exists() {
        fs::create_dir_all(avatar_path).await?;
    }
    Ok(())
}

pub(crate) async fn read_file(base: &PathBuf, gid: &GroupId, name: &str) -> Result<Vec<u8>> {
    let mut path = base.clone();
    path.push(gid.to_hex());
    path.push(FILES_DIR);
    path.push(name);
    if path.exists() {
        Ok(fs::read(path).await?)
    } else {
        Ok(vec![])
    }
}

pub(crate) async fn read_image(base: &PathBuf, gid: &GroupId, name: &str) -> Result<Vec<u8>> {
    let mut path = base.clone();
    path.push(gid.to_hex());
    path.push(IMAGE_DIR);
    path.push(name);
    if path.exists() {
        Ok(fs::read(path).await?)
    } else {
        Ok(vec![])
    }
}

pub(crate) async fn write_file(
    base: &PathBuf,
    gid: &GroupId,
    name: &str,
    bytes: &[u8],
) -> Result<String> {
    let mut path = base.clone();
    path.push(gid.to_hex());
    path.push(FILES_DIR);
    path.push(name);
    fs::write(path, bytes).await?;
    Ok(name.to_owned())
}

#[inline]
fn image_name() -> String {
    let mut name: String = thread_rng()
        .sample_iter(&Alphanumeric)
        .take(20)
        .map(char::from)
        .collect();
    name.push_str(".png");
    name
}

#[inline]
fn image_thumb(bytes: &[u8]) -> Result<DynamicImage> {
    // thumbnail image. 120*800
    let img = load_from_memory(&bytes).map_err(|_e| anyhow!("image invalid format."))?;
    let (x, _) = img.dimensions();
    if x > 100 {
        Ok(img.thumbnail(120, 800))
    } else {
        Ok(img)
    }
}

pub(crate) async fn write_image(base: &PathBuf, gid: &GroupId, bytes: &[u8]) -> Result<String> {
    let mut path = base.clone();
    path.push(gid.to_hex());

    let thumb = image_thumb(bytes)?;
    let name = image_name();

    let mut thumb_path = path.clone();
    thumb_path.push(THUMB_DIR);
    thumb_path.push(name.clone());
    tokio::spawn(async move {
        let _ = thumb.save(thumb_path);
    });

    path.push(IMAGE_DIR);
    path.push(name.clone());
    fs::write(path, bytes).await?;

    Ok(name)
}

#[inline]
fn avatar_png(gid: &GroupId) -> String {
    let mut gs = gid.to_hex();
    gs.push_str(".png");
    gs
}

pub(crate) async fn read_avatar(
    base: &PathBuf,
    gid: &GroupId,
    remote: &GroupId,
) -> Result<Vec<u8>> {
    let mut path = base.clone();
    path.push(gid.to_hex());
    path.push(AVATAR_DIR);
    path.push(avatar_png(remote));
    if path.exists() {
        Ok(fs::read(path).await?)
    } else {
        Ok(vec![])
    }
}

pub(crate) async fn write_avatar(
    base: &PathBuf,
    gid: &GroupId,
    remote: &GroupId,
    bytes: &Vec<u8>,
) -> Result<()> {
    if bytes.len() < 1 {
        return Ok(());
    }
    let mut path = base.clone();
    path.push(gid.to_hex());
    path.push(AVATAR_DIR);
    path.push(avatar_png(remote));
    Ok(fs::write(path, bytes).await?)
}

pub(crate) async fn delete_avatar(base: &PathBuf, gid: &GroupId, remote: &GroupId) -> Result<()> {
    let mut path = base.clone();
    path.push(gid.to_hex());
    path.push(AVATAR_DIR);
    path.push(avatar_png(remote));
    if path.exists() {
        Ok(fs::remove_file(path).await?)
    } else {
        Ok(())
    }
}

pub(crate) async fn read_record(base: &PathBuf, gid: &GroupId, name: &str) -> Result<Vec<u8>> {
    let mut path = base.clone();
    path.push(gid.to_hex());
    path.push(RECORD_DIR);
    path.push(name);
    if path.exists() {
        Ok(fs::read(path).await?)
    } else {
        Ok(vec![])
    }
}

pub(crate) async fn write_record(
    base: &PathBuf,
    gid: &GroupId,
    fid: &i64,
    t: &u32,
    bytes: &Vec<u8>,
) -> Result<String> {
    let start = SystemTime::now();
    let datetime = start
        .duration_since(UNIX_EPOCH)
        .map(|s| s.as_millis())
        .unwrap_or(0u128);

    let mut path = base.clone();
    path.push(gid.to_hex());
    path.push(RECORD_DIR);
    path.push(format!("{}_{}.m4a", fid, datetime));
    fs::write(path, bytes).await?;

    Ok(format!("{}-{}_{}.m4a", t, fid, datetime))
}

pub(crate) async fn _delete_record(base: &PathBuf, gid: &GroupId, name: &str) -> Result<()> {
    let mut path = base.clone();
    path.push(gid.to_hex());
    path.push(RECORD_DIR);
    path.push(name);
    Ok(fs::remove_file(path).await?)
}

pub(crate) fn _write_emoji(base: &PathBuf, gid: &GroupId) -> Result<()> {
    let mut path = base.clone();
    path.push(gid.to_hex());
    path.push(EMOJI_DIR);
    Ok(())
}
