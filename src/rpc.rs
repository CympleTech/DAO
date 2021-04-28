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

use crate::group::Group;
use crate::layer::Layer;

pub(crate) struct RpcState {
    pub group: Arc<RwLock<Group>>,
    pub layer: Arc<RwLock<Layer>>,
}

pub(crate) fn new_rpc_handler(
    addr: PeerAddr,
    group: Arc<RwLock<Group>>,
    layer: Arc<RwLock<Layer>>,
) -> RpcHandler<RpcState> {
    let mut handler = RpcHandler::new(RpcState { group, layer });

    handler.add_method("echo", |_, params, _| async move {
        Ok(HandleResult::rpc(json!(params)))
    });

    handler
}
