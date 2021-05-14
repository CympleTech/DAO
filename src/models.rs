use group_chat_types::{GroupInfo, GroupType};
use sqlx::postgres::PgPool;
use std::time::{SystemTime, UNIX_EPOCH};
use tdn::types::{
    group::GroupId,
    primitive::{new_io_error, PeerAddr, Result},
};

/// Group Chat Model.
pub(crate) struct GroupChat {
    /// db auto-increment id.
    pub id: i64,
    /// group chat owner.
    pub owner: GroupId,
    /// group chat id.
    pub g_id: GroupId,
    /// group chat type.
    g_type: GroupType,
    /// group chat name.
    g_name: String,
    /// group chat simple intro.
    g_bio: String,
    /// group chat need manager agree.
    is_need_agree: bool,
    /// group chat encrypted-key's hash.
    key_hash: Vec<u8>,
    /// group chat is closed.
    is_closed: bool,
    /// group chat created time.
    datetime: i64,
}

impl GroupChat {
    pub fn new(
        owner: GroupId,
        g_id: GroupId,
        g_type: GroupType,
        g_name: String,
        g_bio: String,
        is_need_agree: bool,
        key_hash: Vec<u8>,
    ) -> Self {
        let start = SystemTime::now();
        let datetime = start
            .duration_since(UNIX_EPOCH)
            .map(|s| s.as_secs())
            .unwrap_or(0) as i64; // safe for all life.

        Self {
            owner,
            g_id,
            g_type,
            g_name,
            g_bio,
            is_need_agree,
            key_hash,
            datetime,
            is_closed: false,
            id: 0,
        }
    }
    pub fn to_group_info(self, avatar: Vec<u8>) -> GroupInfo {
        match self.g_type {
            GroupType::Common | GroupType::Open => GroupInfo::Common(
                self.owner,
                self.g_id,
                self.g_type,
                self.is_need_agree,
                self.g_name,
                self.g_bio,
                avatar,
            ),
            GroupType::Encrypted => GroupInfo::Common(
                // TODO decode.
                self.owner,
                self.g_id,
                self.g_type,
                self.is_need_agree,
                self.g_name,
                self.g_bio,
                avatar,
            ),
        }
    }
}

/// Group Member Model.
pub(crate) struct Member {
    /// db auto-increment id.
    id: i64,
    /// group's db id.
    fid: i64,
    /// member's Did(encrypted/not-encrytped)
    m_id: GroupId,
    /// member's addresse.
    m_addr: PeerAddr,
    /// member's name.
    m_name: String,
    /// is group manager.
    is_manager: bool,
    /// member's joined time.
    datetime: i64,
}

/// Group Chat message type.
pub(crate) enum MessageType {
    String,
    Image,
    File,
    Contact,
    Emoji,
    Record,
    Phone,
    Video,
}

/// Group Chat Message Model.
pub(crate) struct Message {
    /// db auto-increment id.
    id: i64,
    /// group's db id.
    fid: i64,
    /// member's db id.
    m_id: i64,
    /// message type.
    m_type: MessageType,
    /// message content.
    m_content: String,
    /// message created time.
    datetime: i64,
}

/// Group Chat Message Model.
pub(crate) struct Manager {
    /// db auto-increment id.
    id: i64,
    /// manager's gid.
    gid: GroupId,
    /// limit group times.
    times: i32,
    /// manager is suspend.
    is_closed: bool,
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
    pub async fn insert(&mut self, pool: &PgPool) -> Result<()> {
        let rec = sqlx::query!(
            "INSERT INTO managers ( gid, times, is_closed, datetime ) VALUES ( $1, $2, $3, $4 ) RETURNING id",
            self.gid.to_hex(),
            self.times,
            self.is_closed,
            self.datetime
        ).fetch_one(pool).await.map_err(|_| new_io_error("database failure."))?;

        self.id = rec.id;
        Ok(())
    }
}
