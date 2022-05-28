use std::sync::Arc;
use tdn::types::{
    group::GroupId,
    primitive::{HandleResult, PeerAddr},
    rpc::{json, RpcError, RpcHandler, RpcParam},
};
use tokio::sync::RwLock;

use crate::layer::Layer;
use crate::manager::Manager;

pub(crate) struct RpcState {
    pub layer: Arc<RwLock<Layer>>,
}

pub(crate) fn new_rpc_handler(addr: PeerAddr, layer: Arc<RwLock<Layer>>) -> RpcHandler<RpcState> {
    let mut handler = RpcHandler::new(RpcState { layer });

    handler.add_method("echo", |_, params, _| async move {
        Ok(HandleResult::rpc(json!(params)))
    });

    // MOCK
    handler.add_method(
        "list-managers",
        |_params: Vec<RpcParam>, state: Arc<RpcState>| async move {
            let managers = Manager::all().await?;
            let mut vecs = vec![];
            for manager in managers {
                vecs.push([
                    json!(manager.id),
                    json!(manager.gid.to_hex()),
                    json!(manager.times),
                    json!(manager.is_closed),
                ]);
            }
            Ok(HandleResult::rpc(json!(vecs)))
        },
    );

    // MOCK
    handler.add_method(
        "add-manager",
        |params: Vec<RpcParam>, _state: Arc<RpcState>| async move {
            let gid = GroupId::from_hex(params[0].as_str().ok_or(RpcError::ParseError)?)?;

            let mut results = HandleResult::rpc(json!(params));

            let mut manager = Manager::new(gid);
            manager.insert().await?;

            Ok(results)
        },
    );

    // MOCK
    handler.add_method(
        "remove-manager",
        |params: Vec<RpcParam>, _state: Arc<RpcState>| async move {
            let gid = GroupId::from_hex(params[0].as_str().ok_or(RpcError::ParseError)?)?;

            let mut results = HandleResult::rpc(json!(params));

            Ok(results)
        },
    );

    handler
}
