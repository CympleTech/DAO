use serde::{Deserialize, Serialize};
use tdn_did::Proof;
use tdn_types::{group::GroupId, primitive::PeerAddr};

#[rustfmt::skip]
pub const GROUP_CHAT_ID: GroupId = GroupId([
    0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 2,
]);

#[derive(Serialize, Deserialize, Debug)]
pub enum GroupType {
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
pub enum GroupInfo {
    /// params: Group_ID, group_type, is_must_agree_by_manager,
    /// group_name, group_bio, group_avatar.
    Common(GroupId, GroupType, bool, String, String, Vec<u8>),
    /// params: Group_ID, is_must_agree_by_manager, key_hash,
    /// group_name(bytes), group_bio(bytes), group_avatar(bytes).
    Encrypted(GroupId, bool, Vec<u8>, Vec<u8>, Vec<u8>, Vec<u8>),
}

pub struct GroupChat {
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

pub struct Member {
    id: i64,
    group_id: i64,
    user_id: GroupId,
    datetime: i64,
}

pub enum MessageType {
    String,
    Image,
    File,
    Contact,
    Emoji,
    Record,
    Phone,
    Video,
}

pub struct Message {
    id: i64,
    group_id: i64,
    content: String,
    m_type: MessageType,
}

/// Group chat connect data.
#[derive(Serialize, Deserialize)]
pub enum GroupConnect {
    /// check if account has permission to create group, and supported group types.
    Check,
    /// create a Group Chat.
    /// params: account, group_info, proof.
    Create(GroupId, GroupInfo, Proof),
    /// join a Group Chat.
    /// params: Group_ID, join_proof, group_event_height.
    Join(GroupId, JoinProof, u64),
}

/// Group chat join proof.
#[derive(Serialize, Deserialize)]
pub enum JoinProof {
    /// when is had in group chat, can only use had to join.
    /// params: account.
    Had(GroupId),
    /// when is join by a link/qrcode, it has not proof. it will check group_type.
    /// params: account.
    Link(GroupId),
    /// when is invate, it will take group_manager's proof for invate.
    /// params: account, invite_proof.
    Invite(GroupId, Proof),
    /// zero-knowledge-proof. not has account id.
    /// verify(proof, key_hash, current_peer_addr).
    Zkp(Proof), // TODO MOCK-PROOF
}

/// Group chat connect result data.
#[derive(Serialize, Deserialize)]
pub enum GroupResult {
    /// result check.
    /// params: account, is_ok, supported_group_types.
    Check(bool, Vec<GroupType>),
    /// result create group success.
    /// params: account, Group_ID
    Create(GroupId, GroupId, bool),
    /// connect result.
    /// params: GroupId, account, is_ok.
    Join(GroupId, GroupId, bool),
    /// join result, need group manager agree.
    /// params: GroupId, account.
    Waiting(GroupId, GroupId),
    /// join result. agree to join.
    /// params: GroupId, account.
    Agree(GroupId, GroupId, GroupInfo),
    /// join result. reject to join.
    /// params: GroupId, account.
    Reject(GroupId, GroupId),
}

/// Group chat event.
#[derive(Serialize, Deserialize)]
pub enum GroupEvent {
    Online(PeerAddr),
    Offline(PeerAddr),
    Sync(u64, Event),
}

#[derive(Serialize, Deserialize, Clone)]
pub enum Event {
    Message,
    GroupUpdate,
    GroupTransfer,
    UserInfo,
    Close,
}
