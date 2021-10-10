use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use tdn::types::{group::GroupId, primitive::Result};

use crate::storage::get_pool;

/// Group Chat Message Model.
pub(crate) struct Manager {
    /// db auto-increment id.
    pub id: i64,
    /// manager's gid.
    pub gid: GroupId,
    /// limit group times.
    pub times: i32,
    /// manager is suspend.
    pub is_closed: bool,
    /// manager created time.
    datetime: i64,
}

impl Manager {
    pub fn new(gid: GroupId) -> Self {
        let start = SystemTime::now();
        let datetime = start
            .duration_since(UNIX_EPOCH)
            .map(|s| s.as_secs())
            .unwrap_or(0) as i64; // safe for all life.

        Self {
            gid,
            datetime,
            times: 10,
            is_closed: false,
            id: 0,
        }
    }

    pub async fn all() -> Result<Vec<Manager>> {
        let recs = sqlx::query!(
            "SELECT id, gid, times, is_closed, datetime FROM managers WHERE is_deleted = false ORDER BY id",
        )
            .fetch_all(get_pool()?).await.map_err(|_| anyhow!("database failure."))?;

        let mut managers = vec![];

        for res in recs {
            managers.push(Self {
                id: res.id,
                gid: GroupId::from_hex(res.gid).unwrap_or(GroupId::default()),
                times: res.times,
                is_closed: res.is_closed,
                datetime: res.datetime,
            });
        }

        Ok(managers)
    }

    pub async fn get(gid: &GroupId) -> Result<Self> {
        let rec = sqlx::query!(
            "SELECT id, gid, times, is_closed, datetime FROM managers WHERE gid=$1",
            gid.to_hex()
        )
        .fetch_one(get_pool()?)
        .await
        .map_err(|_| anyhow!("database failure."))?;

        Ok(Self {
            id: rec.id,
            gid: GroupId::from_hex(rec.gid).unwrap_or(GroupId::default()),
            times: rec.times,
            is_closed: rec.is_closed,
            datetime: rec.datetime,
        })
    }

    pub async fn insert(&mut self) -> Result<()> {
        let unique_check =
            sqlx::query!("SELECT id from managers WHERE gid = $1", self.gid.to_hex())
                .fetch_optional(get_pool()?)
                .await
                .map_err(|_| anyhow!("database failure."))?;

        if let Some(rec) = unique_check {
            self.id = rec.id;
            let _ = sqlx::query!("UPDATE managers SET is_closed = $1, datetime = $2, is_deleted = false WHERE id = $3",
                self.is_closed,
                self.datetime,
                self.id
            ).execute(get_pool()?).await.map_err(|_| anyhow!("database failure."))?;
        } else {
            let rec = sqlx::query!(
            "INSERT INTO managers ( gid, times, is_closed, datetime ) VALUES ( $1, $2, $3, $4 ) RETURNING id",
            self.gid.to_hex(),
            self.times,
            self.is_closed,
            self.datetime
        ).fetch_one(get_pool()?).await.map_err(|_| anyhow!("database failure."))?;
            self.id = rec.id;
        }
        Ok(())
    }

    pub async fn reduce(&self) -> Result<()> {
        let _ = sqlx::query!(
            "UPDATE managers SET times = $1 WHERE id = $2",
            self.times - 1,
            self.id
        )
        .execute(get_pool()?)
        .await
        .map_err(|_| anyhow!("database failure."))?;

        Ok(())
    }
}
