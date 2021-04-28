use std::collections::HashMap;
use tdn::types::{
    group::GroupId,
    message::RecvType,
    primitive::{HandleResult, PeerAddr, Result},
};
use tdn_did::Proof;

use crate::models::{GroupInfo, GroupType};

pub(crate) struct Group {
    managers: HashMap<GroupId, u32>,
    groups: HashMap<GroupId, Vec<PeerAddr>>,
}

/// Group chat connect data.
enum GroupConnect {
    /// check if account has permission to create group, and supported group types.
    Check(GroupId),
    /// create a Group Chat.
    /// params: account, group_info, proof.
    Create(GroupId, GroupInfo, Proof),
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
    /// result check.
    /// params: account, is_ok, supported_group_types.
    Check(GroupId, bool, Vec<GroupType>),
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
enum GroupEvent {
    Message,
    GroupUpdate,
    GroupTransfer,
    Remove,
    UserInfo,
    Close,
}

impl Group {
    pub(crate) fn handle(&mut self, gid: GroupId, msg: RecvType) -> Result<HandleResult> {
        let mut results = HandleResult::new();

        // 1. check account is online, if not online, nothing.
        if !self.contains(&gid) {
            return Ok(results);
        }

        match msg {
            RecvType::Connect(addr, data) => {
                //
            }
            RecvType::Leave(addr) => {
                //
            }
            RecvType::Result(addr, is_ok, data) => {
                //
            }
            RecvType::ResultConnect(addr, data) => {
                //
            }
            RecvType::Event(addr, bytes) => {
                //
            }
            RecvType::Stream(_uid, _stream, _bytes) => {
                // TODO stream
            }
            RecvType::Delivery(_t, _tid, _is_ok) => {
                //
            }
        }

        Ok(results)
    }

    pub(crate) fn new() -> Group {
        Group {
            managers: HashMap::new(),
            groups: HashMap::new(),
        }
    }

    fn contains(&self, gid: &GroupId) -> bool {
        self.managers.contains_key(gid) || self.groups.contains_key(gid)
    }
}
