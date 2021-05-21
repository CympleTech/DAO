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

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
pub enum GroupType {
    /// encrypted group type, data is encrypted, and it can need manager
    /// or take manager's zero-knowledge-proof.
    Encrypted,
    /// common group type, data not encrypted, and need group manager agree.
    Common,
    /// opened group type, data not encrypted, anyone can join this group.
    Open,
}

impl GroupType {
    pub fn to_u32(&self) -> u32 {
        match self {
            GroupType::Encrypted => 0,
            GroupType::Common => 1,
            GroupType::Open => 2,
        }
    }

    pub fn from_u32(u: u32) -> Self {
        match u {
            0 => GroupType::Encrypted,
            1 => GroupType::Common,
            2 => GroupType::Open,
            _ => GroupType::Encrypted,
        }
    }
}

/// GroupInfo transfer in the network.
#[derive(Serialize, Deserialize)]
pub enum GroupInfo {
    /// params: owner, owner_name, Group_ID, group_type, is_must_agree_by_manager,
    /// group_name, group_bio, group_avatar.
    Common(
        GroupId,
        String,
        GroupId,
        GroupType,
        bool,
        String,
        String,
        Vec<u8>,
    ),
    /// params: owner, owner_name, Group_ID, is_must_agree_by_manager, key_hash,
    /// group_name(bytes), group_bio(bytes), group_avatar(bytes).
    Encrypted(
        GroupId,
        String,
        GroupId,
        bool,
        Vec<u8>,
        Vec<u8>,
        Vec<u8>,
        Vec<u8>,
    ),
}

/// Group chat connect data.
#[derive(Serialize, Deserialize)]
pub enum GroupConnect {
    /// check if account has permission to create group, and supported group types.
    Check,
    /// create a Group Chat.
    /// params: group_info, proof.
    Create(GroupInfo, Proof),
    /// join a Group Chat.
    /// params: Group_ID, join_proof.
    Join(GroupId, JoinProof),
}

/// Group chat join proof.
#[derive(Serialize, Deserialize)]
pub enum JoinProof {
    /// when is joined in group chat, can only use had to join (connect).
    /// params: proof.
    Had(Proof),
    /// when join the open group chat.
    /// params: member name, member avatar.
    Open(String, Vec<u8>),
    /// when is join by a link/qrcode, it has not proof. it will check group_type.
    /// params: link_by_account, member name, member avatar.
    Link(GroupId, String, Vec<u8>),
    /// when is invate, it will take group_manager's proof for invate.
    /// params: invite_by_account, invite_proof, member name, member avatar.
    Invite(GroupId, Proof, String, Vec<u8>),
    /// zero-knowledge-proof. not has account id.
    /// verify(proof, key_hash, current_peer_addr).
    Zkp(Proof), // TODO MOCK-PROOF
}

/// check result type.
#[derive(Serialize, Deserialize, Debug)]
pub enum CheckType {
    /// allow to create new group.
    Allow,
    /// cannot created, used all times.
    None,
    /// account is suspended.
    Suspend,
    /// cannot created, no permission.
    Deny,
}

impl CheckType {
    pub fn to_u32(&self) -> u32 {
        match self {
            CheckType::Allow => 0,
            CheckType::None => 1,
            CheckType::Suspend => 2,
            CheckType::Deny => 3,
        }
    }
}

/// Group chat connect result data.
#[derive(Serialize, Deserialize)]
pub enum GroupResult {
    /// result check.
    /// params: account, is_ok, supported_group_types.
    Check(CheckType, Vec<GroupType>),
    /// result create group success.
    /// params: Group_ID, is_ok.
    Create(GroupId, bool),
    /// connect result.
    /// params: GroupId, is_ok, group_event_height.
    Join(GroupId, bool, i64),
    /// join result, need group manager agree.
    /// params: GroupId.
    Waiting(GroupId),
    /// join result. agree to join.
    /// params: GroupId, Group info.
    Agree(GroupId, GroupInfo),
    /// join result. reject to join.
    /// params: GroupId.
    Reject(GroupId),
}

/// ESSE app's layer Event.
#[derive(Serialize, Deserialize)]
pub enum LayerEvent {
    /// offline GroupId. as BaseLayerEvent.
    Offline(GroupId),
    /// online ping GroupId.
    OnlinePing(GroupId),
    /// online pong GroupId
    OnlinePong(GroupId),
    /// online group member. GroupId, member, address.
    MemberOnline(GroupId, GroupId, PeerAddr),
    /// offline group member. GroupId, member, address.
    MemberOffline(GroupId, GroupId, PeerAddr),
    /// sync group event. GroupId, height, event.
    Sync(GroupId, i64, Event),
    /// packed sync event request. GroupId, from.
    SyncReq(GroupId, i64),
    /// packed sync event. GroupId, height, from, to, packed events.
    Packed(GroupId, i64, i64, i64, Vec<PackedEvent>),
}

#[derive(Serialize, Deserialize)]
pub enum PackedEvent {
    GroupInfo,
    GroupTransfer,
    GroupManagerAdd,
    GroupManagerDel,
    GroupClose,
    /// params: member id, member address, member name, member avatar.
    MemberInfo(GroupId, PeerAddr, String, Vec<u8>),
    /// params: member id, member address, member name, member avatar, member join time.
    MemberJoin(GroupId, PeerAddr, String, Vec<u8>, i64),
    /// params: member id,
    MemberLeave(GroupId),
    /// params: member id, message, message time.
    MessageCreate(GroupId, NetworkMessage, i64),
    /// had in before.
    None,
}

#[derive(Serialize, Deserialize, Clone)]
pub enum Event {
    GroupInfo,
    GroupTransfer,
    GroupManagerAdd,
    GroupManagerDel,
    GroupClose,
    /// params: member id, member address, member name, member avatar.
    MemberInfo(GroupId, PeerAddr, String, Vec<u8>),
    /// params: member id, member address, member name, member avatar, member join time.
    MemberJoin(GroupId, PeerAddr, String, Vec<u8>, i64),
    /// params: member id,
    MemberLeave(GroupId),
    /// params: member id, message, height.
    MessageCreate(GroupId, NetworkMessage, i64),
}

/// message type use in network.
#[derive(Serialize, Deserialize, Clone)]
pub enum NetworkMessage {
    String(String),                              // content
    Image(Vec<u8>),                              // image bytes.
    File(String, Vec<u8>),                       // filename, file bytes.
    Contact(String, GroupId, PeerAddr, Vec<u8>), // name, gid, addr, avatar bytes.
    Record(Vec<u8>, u32),                        // record audio bytes.
    Emoji,
    Phone,
    Video,
    None,
}
