use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use tdn::types::{
    group::GroupId,
    primitive::{PeerAddr, Result},
};

use group_chat_types::{GroupInfo, GroupType, NetworkMessage, PackedEvent};

use crate::storage::{
    get_pool, read_avatar, read_file, read_image, read_record, write_avatar, write_file,
    write_image, write_record,
};

/// Group Chat Model.
pub(crate) struct GroupChat {
    /// db auto-increment id.
    pub id: i64,
    /// group chat owner.
    pub owner: GroupId,
    /// group height.
    pub height: i64,
    /// group chat id.
    pub g_id: GroupId,
    /// group chat type.
    pub g_type: GroupType,
    /// group chat name.
    g_name: String,
    /// group chat simple intro.
    g_bio: String,
    /// group chat need manager agree.
    pub is_need_agree: bool,
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
            height: 0,
            id: 0,
        }
    }

    pub fn to_group_info(self, avatar: Vec<u8>) -> GroupInfo {
        match self.g_type {
            GroupType::Private | GroupType::Open => GroupInfo::Common(
                self.owner,
                "".to_owned(), // no-need. because in member.
                vec![],        // owner avatar no-need. because in member.
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
                "".to_owned(), // no-need. because in member.
                vec![],        // owner avatar no-need. because in member.
                self.g_id,
                self.g_type,
                self.is_need_agree,
                self.g_name,
                self.g_bio,
                avatar,
            ),
        }
    }

    pub async fn get_id(id: &i64) -> Result<GroupChat> {
        let res = sqlx::query!(
            "SELECT id, owner, height, g_id, g_type, g_name, g_bio, is_need_agree, key_hash, is_closed, datetime FROM groups WHERE is_deleted = false and id = $1",
            id
        ).fetch_one(get_pool()?).await.map_err(|_| anyhow!("database failure."))?;

        Ok(Self {
            id: res.id,
            owner: GroupId::from_hex(res.owner).unwrap_or(GroupId::default()),
            height: res.height,
            g_id: GroupId::from_hex(res.g_id).unwrap_or(GroupId::default()),
            g_type: GroupType::from_u32(res.g_type as u32),
            g_name: res.g_name,
            g_bio: res.g_bio,
            is_need_agree: res.is_need_agree,
            key_hash: hex::decode(res.key_hash).unwrap_or(vec![]),
            is_closed: res.is_closed,
            datetime: res.datetime,
        })
    }

    pub async fn all() -> Result<Vec<GroupChat>> {
        let recs = sqlx::query!(
            "SELECT id, owner, height, g_id, g_type, g_name, g_bio, is_need_agree, key_hash, is_closed, datetime FROM groups WHERE is_deleted = false ORDER BY id",
        )
            .fetch_all(get_pool()?).await.map_err(|_| anyhow!("database failure."))?;

        let mut managers = vec![];

        for res in recs {
            managers.push(Self {
                id: res.id,
                owner: GroupId::from_hex(res.owner).unwrap_or(GroupId::default()),
                height: res.height,
                g_id: GroupId::from_hex(res.g_id).unwrap_or(GroupId::default()),
                g_type: GroupType::from_u32(res.g_type as u32),
                g_name: res.g_name,
                g_bio: res.g_bio,
                is_need_agree: res.is_need_agree,
                key_hash: hex::decode(res.key_hash).unwrap_or(vec![]),
                is_closed: res.is_closed,
                datetime: res.datetime,
            });
        }

        Ok(managers)
    }

    pub async fn insert(&mut self) -> Result<()> {
        // check if unique group id.
        let unique_check =
            sqlx::query!("SELECT id from groups WHERE g_id = $1", self.g_id.to_hex())
                .fetch_optional(get_pool()?)
                .await
                .map_err(|_| anyhow!("database failure."))?;
        if unique_check.is_some() {
            return Err(anyhow!("unique group id."));
        }

        let rec = sqlx::query!(
            "INSERT INTO groups (owner, height, g_id, g_type, g_name, g_bio, is_need_agree, key_hash, is_closed, datetime) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10) RETURNING id",
            self.owner.to_hex(),
            self.height,
            self.g_id.to_hex(),
            self.g_type.to_u32() as i64,
            self.g_name,
            self.g_bio,
            self.is_need_agree,
            hex::encode(&self.key_hash),
            self.is_closed,
            self.datetime
        ).fetch_one(get_pool()?).await.map_err(|_| anyhow!("database failure."))?;

        self.id = rec.id;
        Ok(())
    }

    pub async fn add_height(id: &i64, height: &i64) -> Result<()> {
        let _ = sqlx::query!("UPDATE groups SET height = $1 WHERE id = $2", height, id)
            .execute(get_pool()?)
            .await
            .map_err(|_| anyhow!("database failure."))?;

        Ok(())
    }
}

/// Group Member Model.
pub(crate) struct Request {
    /// db auto-increment id.
    pub id: i64,
    /// group's db id.
    pub fid: i64,
    /// member's Did(encrypted/not-encrytped)
    pub m_id: GroupId,
    /// member's addresse.
    pub m_addr: PeerAddr,
    /// member's name.
    pub m_name: String,
    /// member's joined time.
    pub datetime: i64,
}

impl Request {
    pub fn new() -> Request {
        todo!()
    }

    pub fn to_member(self) -> Member {
        todo!()
    }

    pub async fn get(id: &i64) -> Result<Request> {
        todo!()
    }

    pub async fn insert(&mut self) -> Result<()> {
        todo!()
    }
}

/// Group Member Model.
pub(crate) struct Member {
    /// db auto-increment id.
    pub id: i64,
    /// group's db id.
    fid: i64,
    /// member's Did(encrypted/not-encrytped)
    pub m_id: GroupId,
    /// member's addresse.
    pub m_addr: PeerAddr,
    /// member's name.
    pub m_name: String,
    /// is group manager.
    is_manager: bool,
    /// member's joined time.
    pub datetime: i64,
}

impl Member {
    pub fn new(
        fid: i64,
        m_id: GroupId,
        m_addr: PeerAddr,
        m_name: String,
        is_manager: bool,
    ) -> Self {
        let start = SystemTime::now();
        let datetime = start
            .duration_since(UNIX_EPOCH)
            .map(|s| s.as_secs())
            .unwrap_or(0) as i64; // safe for all life.

        Self {
            fid,
            datetime,
            m_id,
            m_addr,
            m_name,
            is_manager,
            id: 0,
        }
    }

    pub async fn insert(&mut self) -> Result<()> {
        let unique_check = sqlx::query!(
            "SELECT id from members WHERE fid = $1 AND m_id = $2",
            self.fid,
            self.m_id.to_hex()
        )
        .fetch_optional(get_pool()?)
        .await
        .map_err(|_| anyhow!("database failure."))?;

        if let Some(rec) = unique_check {
            self.id = rec.id;
            let _ = sqlx::query!("UPDATE members SET m_addr = $1, m_name = $2, is_manager = $3, datetime = $4, is_deleted = false WHERE id = $5",
                self.m_addr.to_hex(),
                self.m_name,
                self.is_manager,
                self.datetime,
                self.id
            ).execute(get_pool()?).await.map_err(|_| anyhow!("database failure."))?;
        } else {
            let rec = sqlx::query!(
                "INSERT INTO members (fid, m_id, m_addr, m_name, is_manager, datetime) VALUES ($1, $2, $3, $4, $5, $6) RETURNING id",
                self.fid,
                self.m_id.to_hex(),
                self.m_addr.to_hex(),
                self.m_name,
                self.is_manager,
                self.datetime
            ).fetch_one(get_pool()?).await.map_err(|_| anyhow!("database failure."))?;
            self.id = rec.id;
        }

        Ok(())
    }

    pub async fn exist(fid: &i64, mid: &GroupId) -> Result<bool> {
        sqlx::query!(
            "SELECT id FROM members WHERE fid = $1 AND m_id = $2 AND is_deleted = false",
            fid,
            mid.to_hex()
        )
        .fetch_optional(get_pool()?)
        .await
        .map_err(|_| anyhow!("database failure."))
        .map(|v| v.is_some())
    }

    pub async fn get_id(id: &i64) -> Result<Member> {
        let rec = sqlx::query!(
            "SELECT id, fid, m_id, m_addr, m_name, is_manager, datetime FROM members WHERE id = $1",
            id,
        )
        .fetch_one(get_pool()?)
        .await
        .map_err(|_| anyhow!("database failure."))?;

        Ok(Member {
            id: rec.id,
            fid: rec.fid,
            m_id: GroupId::from_hex(rec.m_id).unwrap_or(GroupId::default()),
            m_addr: PeerAddr::from_hex(rec.m_addr).unwrap_or(PeerAddr::default()),
            m_name: rec.m_name,
            is_manager: rec.is_manager,
            datetime: rec.datetime,
        })
    }

    pub async fn get(fid: &i64, gid: &GroupId) -> Result<Member> {
        let rec = sqlx::query!(
            "SELECT id, fid, m_id, m_addr, m_name, is_manager, datetime FROM members WHERE fid = $1 AND m_id = $2 AND is_deleted = false",
            fid,
            gid.to_hex(),
        )
        .fetch_one(get_pool()?)
        .await
        .map_err(|_| anyhow!("database failure."))?;

        Ok(Member {
            id: rec.id,
            fid: rec.fid,
            m_id: GroupId::from_hex(rec.m_id).unwrap_or(GroupId::default()),
            m_addr: PeerAddr::from_hex(rec.m_addr).unwrap_or(PeerAddr::default()),
            m_name: rec.m_name,
            is_manager: rec.is_manager,
            datetime: rec.datetime,
        })
    }

    pub async fn leave(&self) -> Result<()> {
        let _ = sqlx::query!(
            "UPDATE members SET is_deleted = true WHERE id = $1",
            self.id
        )
        .execute(get_pool()?)
        .await
        .map_err(|_| anyhow!("database failure."))?;

        Ok(())
    }

    pub async fn is_manager(fid: &i64, mid: &GroupId) -> Result<bool> {
        let recs = sqlx::query!(
            "SELECT is_deleted, is_manager FROM members WHERE fid = $1 AND m_id = $2",
            fid,
            mid.to_hex()
        )
        .fetch_all(get_pool()?)
        .await
        .map_err(|_| anyhow!("database failure."))?;

        for res in recs {
            if !res.is_deleted && res.is_manager {
                return Ok(true);
            }
        }
        Ok(false)
    }
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
    Invite,
}

impl MessageType {
    fn to_i16(&self) -> i16 {
        match self {
            MessageType::String => 0,
            MessageType::Image => 1,
            MessageType::File => 2,
            MessageType::Contact => 3,
            MessageType::Emoji => 4,
            MessageType::Record => 5,
            MessageType::Phone => 6,
            MessageType::Video => 7,
            MessageType::Invite => 8,
        }
    }

    fn from_i16(i: i16) -> Self {
        match i {
            0 => MessageType::String,
            1 => MessageType::Image,
            2 => MessageType::File,
            3 => MessageType::Contact,
            4 => MessageType::Emoji,
            5 => MessageType::Record,
            6 => MessageType::Phone,
            7 => MessageType::Video,
            8 => MessageType::Invite,
            _ => MessageType::String,
        }
    }
}

/// Group Chat Message Model.
pub(crate) struct Message {
    /// db auto-increment id.
    id: i64,
    /// group's db id.
    fid: i64,
    /// member's db id.
    mid: i64,
    /// message type.
    m_type: MessageType,
    /// message content.
    m_content: String,
    /// message created time.
    datetime: i64,
}

impl Message {
    pub async fn from_network_message(
        base: &PathBuf,
        gcd: &GroupId,
        fid: &i64,
        m_id: &GroupId,
        msg: &NetworkMessage,
    ) -> Result<i64> {
        let start = SystemTime::now();
        let datetime = start
            .duration_since(UNIX_EPOCH)
            .map(|s| s.as_secs())
            .unwrap_or(0) as i64; // safe for all life.

        let member = Member::get(fid, m_id).await?;

        // handle event.
        let (m_type, raw) = match msg {
            NetworkMessage::String(content) => (MessageType::String, content.to_owned()),
            NetworkMessage::Image(bytes) => {
                let image_name = write_image(base, &gcd, bytes).await?;
                (MessageType::Image, image_name)
            }
            NetworkMessage::File(old_name, bytes) => {
                let filename = write_file(base, &gcd, &old_name, bytes).await?;
                (MessageType::File, filename)
            }
            NetworkMessage::Contact(name, rgid, addr, avatar_bytes) => {
                write_avatar(base, gcd, &rgid, avatar_bytes).await?;
                let tmp_name = name.replace(";", "-;");
                let contact_values = format!("{};;{};;{}", tmp_name, rgid.to_hex(), addr.to_hex());
                (MessageType::Contact, contact_values)
            }
            NetworkMessage::Emoji => {
                // TODO
                (MessageType::Emoji, "".to_owned())
            }
            NetworkMessage::Record(bytes, time) => {
                let record_name = write_record(base, &gcd, fid, time, bytes).await?;
                (MessageType::Record, record_name)
            }
            NetworkMessage::Phone => {
                // TODO
                (MessageType::Phone, "".to_owned())
            }
            NetworkMessage::Video => {
                // TODO
                (MessageType::Video, "".to_owned())
            }
            NetworkMessage::Invite(content) => (MessageType::Invite, content.to_owned()),
            NetworkMessage::None => (MessageType::String, "".to_owned()),
        };

        let rec = sqlx::query!(
            "INSERT INTO messages (fid, mid, m_type, m_content, datetime) VALUES ($1, $2, $3, $4, $5) RETURNING id",
            fid,
            member.id,
            m_type.to_i16(),
            raw,
            datetime,
        ).fetch_one(get_pool()?).await.map_err(|_| anyhow!("database failure."))?;

        Ok(rec.id)
    }

    async fn to_network_message(self, base: &PathBuf, gcd: &GroupId) -> Result<NetworkMessage> {
        match self.m_type {
            MessageType::String => Ok(NetworkMessage::String(self.m_content)),
            MessageType::Image => {
                let bytes = read_image(base, gcd, &self.m_content).await?;
                Ok(NetworkMessage::Image(bytes))
            }
            MessageType::File => {
                let bytes = read_file(base, gcd, &self.m_content).await?;
                Ok(NetworkMessage::File(self.m_content, bytes))
            }
            MessageType::Contact => {
                let v: Vec<&str> = self.m_content.split(";;").collect();
                if v.len() != 3 {
                    Ok(NetworkMessage::None)
                } else {
                    let cname = v[0].to_owned();
                    let cgid = GroupId::from_hex(v[1])?;
                    let caddr = PeerAddr::from_hex(v[2])?;
                    let avatar_bytes = read_avatar(base, gcd, &cgid).await?;
                    let avatar = vec![];
                    Ok(NetworkMessage::Contact(cname, cgid, caddr, avatar))
                }
            }
            MessageType::Record => {
                let (bytes, time) = if let Some(i) = self.m_content.find('-') {
                    let time = self.m_content[0..i].parse().unwrap_or(0);
                    let bytes = read_record(base, gcd, &self.m_content[i + 1..]).await?;
                    let bytes = vec![];
                    (bytes, time)
                } else {
                    (vec![], 0)
                };
                Ok(NetworkMessage::Record(bytes, time))
            }
            MessageType::Emoji => Ok(NetworkMessage::Emoji),
            MessageType::Phone => Ok(NetworkMessage::Phone),
            MessageType::Video => Ok(NetworkMessage::Video),
            MessageType::Invite => Ok(NetworkMessage::Invite(self.m_content)),
        }
    }

    pub async fn get_id(id: &i64) -> Result<Message> {
        let rec = sqlx::query!(
            "SELECT id, fid, mid, m_type, m_content, datetime FROM messages WHERE id = $1",
            id,
        )
        .fetch_one(get_pool()?)
        .await
        .map_err(|_| anyhow!("database failure."))?;

        Ok(Message {
            id: rec.id,
            fid: rec.fid,
            mid: rec.mid,
            m_type: MessageType::from_i16(rec.m_type),
            m_content: rec.m_content,
            datetime: rec.datetime,
        })
    }
}

/// Group Chat Message Model.
pub(crate) struct Manager {
    /// db auto-increment id.
    id: i64,
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
}

pub(crate) enum ConsensusType {
    GroupInfo,
    GroupTransfer,
    GroupManagerAdd,
    GroupManagerDel,
    GroupClose,
    MemberInfo,
    MemberJoin,
    MemberLeave,
    MessageCreate,
    None,
}

impl ConsensusType {
    fn to_i16(&self) -> i16 {
        match self {
            ConsensusType::None => 0,
            ConsensusType::GroupInfo => 1,
            ConsensusType::GroupTransfer => 2,
            ConsensusType::GroupManagerAdd => 3,
            ConsensusType::GroupManagerDel => 4,
            ConsensusType::GroupClose => 5,
            ConsensusType::MemberInfo => 6,
            ConsensusType::MemberJoin => 7,
            ConsensusType::MemberLeave => 8,
            ConsensusType::MessageCreate => 9,
        }
    }

    fn from_i16(a: i16) -> Self {
        match a {
            1 => ConsensusType::GroupInfo,
            2 => ConsensusType::GroupTransfer,
            3 => ConsensusType::GroupManagerAdd,
            4 => ConsensusType::GroupManagerDel,
            5 => ConsensusType::GroupClose,
            6 => ConsensusType::MemberInfo,
            7 => ConsensusType::MemberJoin,
            8 => ConsensusType::MemberLeave,
            9 => ConsensusType::MessageCreate,
            _ => ConsensusType::None,
        }
    }
}

/// Group Chat Consensus.
pub(crate) struct Consensus {
    /// db auto-increment id.
    id: i64,
    /// group's db id.
    fid: i64,
    /// group's height.
    height: i64,
    /// consensus type.
    ctype: ConsensusType,
    /// consensus point value db id.
    cid: i64,
}

impl Consensus {
    pub async fn pack(
        base: &PathBuf,
        gcd: &GroupId,
        fid: &i64,
        from: &i64,
        to: &i64,
    ) -> Result<Vec<PackedEvent>> {
        let recs =
            sqlx::query!("SELECT id, fid, height, ctype, cid FROM consensus WHERE fid = $1 AND height BETWEEN $2 AND $3", fid, from, to)
            .fetch_all(get_pool()?)
            .await
            .map_err(|_| anyhow!("database failure."))?;

        let mut packed = vec![];

        for res in recs {
            match ConsensusType::from_i16(res.ctype) {
                ConsensusType::GroupInfo => {
                    //
                }
                ConsensusType::GroupTransfer => {
                    //
                }
                ConsensusType::GroupManagerAdd => {
                    //
                }
                ConsensusType::GroupManagerDel => {
                    //
                }
                ConsensusType::GroupClose => {
                    //
                }
                ConsensusType::MemberInfo => {
                    //
                }
                ConsensusType::MemberJoin => {
                    let m = Member::get_id(&res.cid).await?;
                    // TODO load member avatar.
                    let mavatar = vec![];
                    packed.push(PackedEvent::MemberJoin(
                        m.m_id, m.m_addr, m.m_name, mavatar, m.datetime,
                    ))
                }
                ConsensusType::MemberLeave => {
                    //
                }
                ConsensusType::MessageCreate => {
                    let m = Message::get_id(&res.cid).await?;
                    let datetime = m.datetime;
                    let mem = Member::get_id(&m.mid).await?;
                    let nmsg = m.to_network_message(base, gcd).await?;
                    packed.push(PackedEvent::MessageCreate(mem.m_id, nmsg, datetime))
                }
                ConsensusType::None => {
                    // None
                }
            }
        }

        Ok(packed)
    }

    pub async fn insert(fid: &i64, height: &i64, cid: &i64, ctype: &ConsensusType) -> Result<()> {
        let unique_check = sqlx::query!(
            "SELECT id from consensus WHERE fid = $1 AND height = $2",
            fid,
            height
        )
        .fetch_optional(get_pool()?)
        .await
        .map_err(|_| anyhow!("database failure."))?;

        if let Some(rec) = unique_check {
            let _ = sqlx::query!(
                "UPDATE consensus SET ctype = $1, cid = $2 WHERE id = $3",
                ctype.to_i16(),
                cid,
                rec.id
            )
            .execute(get_pool()?)
            .await
            .map_err(|_| anyhow!("database failure."))?;
        } else {
            let _ = sqlx::query!(
                "INSERT INTO consensus ( fid, height, ctype, cid ) VALUES ( $1, $2, $3, $4 )",
                fid,
                height,
                ctype.to_i16(),
                cid
            )
            .execute(get_pool()?)
            .await
            .map_err(|_| anyhow!("database failure."))?;
        }

        Ok(())
    }
}
