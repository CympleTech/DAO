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
    /// params: Owner, Group_ID, group_type, is_must_agree_by_manager,
    /// group_name, group_bio, group_avatar.
    Common(GroupId, GroupId, GroupType, bool, String, String, Vec<u8>),
    /// params: Owner, Group_ID, is_must_agree_by_manager, key_hash,
    /// group_name(bytes), group_bio(bytes), group_avatar(bytes).
    Encrypted(GroupId, GroupId, bool, Vec<u8>, Vec<u8>, Vec<u8>, Vec<u8>),
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
    /// params: Group_ID, join_proof, group_event_height.
    Join(GroupId, JoinProof, u64),
}

/// Group chat join proof.
#[derive(Serialize, Deserialize)]
pub enum JoinProof {
    /// when is joined in group chat, can only use had to join (connect).
    /// params: proof.
    Had(Proof),
    /// when is join by a link/qrcode, it has not proof. it will check group_type.
    /// params: link_by_account.
    Link(GroupId),
    /// when is invate, it will take group_manager's proof for invate.
    /// params: invite_by_account, invite_proof.
    Invite(GroupId, Proof),
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
    /// cannot created, no permission.
    Deny,
}

impl CheckType {
    pub fn to_u32(&self) -> u32 {
        match self {
            CheckType::Allow => 0,
            CheckType::None => 1,
            CheckType::Deny => 2,
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
    /// params: account, Group_ID
    Create(GroupId, bool),
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

/// ESSE app's layer Event.
#[derive(Serialize, Deserialize)]
pub(crate) enum LayerEvent {
    /// receiver gid, sender gid. as BaseLayerEvent.
    OnlinePing,
    /// receiver gid, sender gid. as BaseLayerEvent.
    OnlinePong,
    /// receiver gid, sender gid. as BaseLayerEvent.
    Offline,
    /// online group member.
    MemberOnline(PeerAddr),
    /// offline group member.
    MemberOffline(PeerAddr),
    /// sync group message.
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
