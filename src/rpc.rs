use async_lock::RwLock;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tdn::{
    smol::channel::{SendError, Sender},
    types::{
        group::GroupId,
        message::{NetworkType, SendMessage, SendType, StateRequest, StateResponse},
        primitive::{new_io_error, HandleResult, PeerAddr, Result},
        rpc::{json, rpc_response, RpcError, RpcHandler, RpcParam},
    },
};

use crate::layer::Layer;

pub(crate) struct RpcState {
    pub group: Arc<RwLock<Layer>>,
}

pub(crate) fn new_rpc_handler(addr: PeerAddr, group: Arc<RwLock<Layer>>) -> RpcHandler<RpcState> {
    let mut handler = RpcHandler::new(RpcState { group });

    handler.add_method("echo", |_, params, _| async move {
        Ok(HandleResult::rpc(json!(params)))
    });

    // MOCK
    handler.add_method(
        "add-manager",
        |_gid: GroupId, params: Vec<RpcParam>, state: Arc<RpcState>| async move {
            let gid = GroupId::from_hex(params[0].as_str()?)?;

            let mut results = HandleResult::rpc(json!(params));

            state.group.write().await.add_manager(gid, 5);
            results.networks.push(NetworkType::AddGroup(gid));

            Ok(results)
        },
    );

    // MOCK
    handler.add_method(
        "remove-manager",
        |_gid: GroupId, params: Vec<RpcParam>, state: Arc<RpcState>| async move {
            let gid = GroupId::from_hex(params[0].as_str()?)?;

            let mut results = HandleResult::rpc(json!(params));

            state.group.write().await.remove_manager(&gid);
            results.networks.push(NetworkType::DelGroup(gid));

            Ok(results)
        },
    );

    handler
}
