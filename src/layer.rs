use tdn::types::{
    group::GroupId,
    message::RecvType,
    primitive::{HandleResult, Result},
};
use tdn_did::Proof;

use crate::models::{GroupInfo, GroupType};

pub(crate) struct Layer {
    //
}

/// Group chat layer connect data.
enum LayerConnect {
    /// check if account has permission to create group, and supported group types.
    Check(GroupId),
    /// create a Group Chat.
    /// params: account, group_info, proof.
    Create(GroupId, GroupInfo, Proof),
}

/// Group chat layer connect result data.
enum LayerResult {
    /// result check.
    /// params: account, is_ok, supported_group_types.
    Check(GroupId, bool, Vec<GroupType>),
    /// result create group success.
    /// params: account, Group_ID
    Ok(GroupId, GroupId),
    /// result create group failure.
    /// params: account, Group_ID
    Err(GroupId, GroupId),
}

impl Layer {
    pub(crate) fn new() -> Layer {
        Layer {}
    }

    pub(crate) fn handle(
        &mut self,
        _fgid: GroupId,
        _tgid: GroupId,
        _msg: RecvType,
    ) -> Result<HandleResult> {
        todo!()
    }
}
