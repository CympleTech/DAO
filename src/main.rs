#[macro_use]
extern crate log;

mod group;
mod rpc;
mod storage;

use simplelog::{CombinedLogger, Config as LogConfig, LevelFilter};
use std::env::args;
use std::path::PathBuf;
use std::sync::Arc;
use tdn::{
    prelude::*,
    smol::{channel::Sender, io::Result, lock::RwLock},
};

pub const DEFAULT_P2P_ADDR: &'static str = "127.0.0.1:7366"; // DEBUG CODE
pub const DEFAULT_HTTP_ADDR: &'static str = "127.0.0.1:8002"; // DEBUG CODE
pub const DEFAULT_LOG_FILE: &'static str = "esse.log.txt";

fn main() {
    let db_path = args().nth(1).unwrap_or("./.tdn".to_owned());

    if std::fs::metadata(&db_path).is_err() {
        std::fs::create_dir(&db_path).unwrap();
    }

    tdn::smol::block_on(start(db_path)).unwrap();
}

pub async fn start(db_path: String) -> Result<()> {
    let db_path = PathBuf::from(db_path);
    if !db_path.exists() {
        tdn::smol::fs::create_dir_all(&db_path).await?;
    }

    init_log(db_path.clone());
    info!("Core storage path {:?}", db_path);

    // let _client = storage::connect_database()?;

    let mut config = Config::load_save(db_path.clone()).await;
    config.db_path = Some(db_path.clone());
    // // use self sign to bootstrap peer.
    // if config.rpc_ws.is_none() {
    //     // set default ws addr.
    //     config.rpc_ws = Some(DEFAULT_WS_ADDR.parse().unwrap());
    // }
    config.rpc_ws = None;
    config.rpc_addr = DEFAULT_HTTP_ADDR.parse().unwrap();
    config.p2p_addr = DEFAULT_P2P_ADDR.parse().unwrap();
    config
        .p2p_allowlist
        .append(&mut vec!["1.15.156.199:7364".parse().unwrap()]);

    info!("Config RPC HTTP : {:?}", config.rpc_addr);
    info!("Config P2P      : {:?}", config.p2p_addr);

    let _rand_secret = config.secret.clone();

    let (peer_id, sender, recver) = start_with_config(config).await.unwrap();
    info!("Network Peer id : {}", peer_id.to_hex());

    let group = Arc::new(RwLock::new(group::Group::new()));

    let rpc_handler = rpc::new_rpc_handler(peer_id, group.clone());

    while let Ok(message) = recver.recv().await {
        match message {
            ReceiveMessage::Group(fgid, g_msg) => {
                if let Ok(results) = group.write().await.handle(fgid, g_msg) {
                    handle(results, 0, &sender).await;
                }
            }
            ReceiveMessage::Rpc(uid, params, _is_ws) => {
                if let Ok(results) = rpc_handler.handle(params).await {
                    handle(results, uid, &sender).await;
                }
            }
            ReceiveMessage::NetworkLost => {
                //
            }
        }
    }

    Ok(())
}

#[inline]
async fn handle(handle_result: HandleResult, uid: u64, sender: &Sender<SendMessage>) {
    let HandleResult {
        mut rpcs,
        mut groups,
        mut networks,
    } = handle_result;

    loop {
        if rpcs.len() != 0 {
            let msg = rpcs.remove(0);
            sender
                .send(SendMessage::Rpc(uid, msg, false))
                .await
                .expect("TDN channel closed");
        } else {
            break;
        }
    }

    loop {
        if networks.len() != 0 {
            let msg = networks.remove(0);
            sender
                .send(SendMessage::Network(msg))
                .await
                .expect("TDN channel closed");
        } else {
            break;
        }
    }

    loop {
        if groups.len() != 0 {
            let (gid, msg) = groups.remove(0);
            sender
                .send(SendMessage::Group(gid, msg))
                .await
                .expect("TDN channel closed");
        } else {
            break;
        }
    }
}

#[inline]
pub fn init_log(mut db_path: PathBuf) {
    db_path.push(DEFAULT_LOG_FILE);

    #[cfg(debug_assertions)]
    CombinedLogger::init(vec![simplelog::TermLogger::new(
        LevelFilter::Debug,
        LogConfig::default(),
        simplelog::TerminalMode::Mixed,
    )])
    .unwrap();

    #[cfg(not(debug_assertions))]
    CombinedLogger::init(vec![simplelog::WriteLogger::new(
        LevelFilter::Debug,
        LogConfig::default(),
        std::fs::File::create(db_path).unwrap(),
    )])
    .unwrap();
}
