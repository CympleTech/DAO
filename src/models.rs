use serde::{Deserialize, Serialize};
use tdn::types::group::GroupId;

#[derive(Serialize, Deserialize)]
pub(crate) enum GroupType {
    /// common group type, data not encrypted, and need group manager agree.
    Common,
    /// encrypted group type, data is encrypted, and it can need manager
    /// or take manager's zero-knowledge-proof.
    Encrypted,
    /// opened group type, data not encrypted, anyone can join this group.
    Open,
}

/// GroupInfo transfer in the network.
#[derive(Serialize, Deserialize)]
pub(crate) enum GroupInfo {
    /// params: Group_ID, group_type, is_must_agree_by_manager,
    /// group_name, group_bio, group_avatar.
    Common(GroupId, GroupType, bool, String, String, Vec<u8>),
    /// params: Group_ID, is_must_agree_by_manager, key_hash,
    /// group_name(bytes), group_bio(bytes), group_avatar(bytes).
    Encrypted(GroupId, bool, Vec<u8>, Vec<u8>, Vec<u8>, Vec<u8>),
}

pub(crate) struct GroupChat {
    id: i64,
    hash: GroupId,
    g_type: GroupType,
    g_name: String,
    g_bio: String,
    is_closed: bool,
    key_hash: String,
    is_must_agree: bool,
    datetime: i64,
}

pub(crate) struct Member {
    id: i64,
    group_id: i64,
    user_id: GroupId,
    datetime: i64,
}

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

pub(crate) struct Message {
    id: i64,
    group_id: i64,
    content: String,
    m_type: MessageType,
}
