use tdn::types::{
    group::GroupId,
    message::RecvType,
    primitive::{HandleResult, Result},
};
use tdn_did::Proof;

use crate::models::GroupInfo;

pub(crate) struct Group {
    //
}

/// Group chat connect data.
enum GroupConnect {
    /// join a Group Chat.
    /// params: Group_ID, join_proof.
    Join(GroupId, JoinProof),
}

/// Group chat join proof.
enum JoinProof {
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
enum GroupResult {
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
enum GroupEvent {
    Message,
    GroupUpdate,
    GroupTransfer,
    Remove,
    UserInfo,
    Close,
}

impl Group {
    pub(crate) fn new() -> Group {
        Group {}
    }

    pub(crate) fn handle(&mut self, _gid: GroupId, _msg: RecvType) -> Result<HandleResult> {
        todo!()
    }
}
