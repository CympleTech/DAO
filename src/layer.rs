use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tdn::types::{
    group::GroupId,
    message::{RecvType, SendType},
    primitive::{new_io_error, HandleResult, PeerAddr, Result},
};
use tdn_did::Proof;

use group_chat_types::{
    Event, GroupConnect, GroupEvent, GroupInfo, GroupResult, GroupType, GROUP_CHAT_ID,
};

/// Group chat server to ESSE.
#[inline]
pub fn add_layer(results: &mut HandleResult, gid: GroupId, msg: SendType) {
    results.layers.push((GROUP_CHAT_ID, gid, msg));
}

pub(crate) struct Layer {
    managers: HashMap<GroupId, u32>,
    groups: HashMap<GroupId, (Vec<PeerAddr>, u64)>,
}

impl Layer {
    pub(crate) fn new() -> Layer {
        Layer {
            managers: HashMap::new(),
            groups: HashMap::new(),
        }
    }

    pub(crate) fn handle(&mut self, gid: GroupId, msg: RecvType) -> Result<HandleResult> {
        println!("Group ID connect {:?}", gid.to_hex());
        let mut results = HandleResult::new();

        match msg {
            RecvType::Connect(addr, data) => {
                println!("===== Connect ======= ");
                let connect = postcard::from_bytes(&data)
                    .map_err(|_e| new_io_error("deserialize group chat connect failure"))?;

                match connect {
                    GroupConnect::Check => {
                        println!("===== Check Start ======= ");
                        let supported =
                            vec![GroupType::Common, GroupType::Encrypted, GroupType::Open];
                        let res = if let Some(limit) = self.managers.get(&gid) {
                            if *limit > 0 {
                                GroupResult::Check(true, supported)
                            } else {
                                GroupResult::Check(false, supported)
                            }
                        } else {
                            GroupResult::Check(false, supported)
                        };
                        let data = postcard::to_allocvec(&res).unwrap_or(vec![]);
                        let s = SendType::Result(0, addr, false, false, data);
                        add_layer(&mut results, gid, s);
                        println!("===== Check Over ======= ");
                    }
                    GroupConnect::Create(account, info, proof) => {
                        if let Some(limit) = self.managers.get(&gid) {
                            if *limit > 0 {
                                // TODO return OK.
                            } else {
                                // TODO return Err.
                            }
                        }
                    }
                    GroupConnect::Join(gid, proof, remote_height) => {
                        // 1. check account is online, if not online, nothing.
                        if let Some((members, _height)) = self.groups.get_mut(&gid) {
                            if members.contains(&addr) {
                                // TODO return OK.
                            } else {
                                // TODO check proof.

                                // TODO boradcast online event.

                                // TODO sync events.
                            }
                        }
                    }
                }
            }
            RecvType::Leave(addr) => {
                for (_g, (members, _)) in self.groups.iter_mut() {
                    if let Some(pos) = members.iter().position(|x| x == &addr) {
                        members.remove(pos);
                        let data = postcard::to_allocvec(&GroupEvent::Offline(addr))
                            .map_err(|_| new_io_error("serialize event error."))?;
                        for member in members {
                            let s = SendType::Event(0, *member, data.clone());
                            add_layer(&mut results, gid, s);
                        }
                    }
                }
            }
            RecvType::Result(_addr, _is_ok, _data) => {
                // no-reach here. here must be user's peer.
            }
            RecvType::ResultConnect(_addr, _data) => {
                // no-reach here. here must be user's peer.
            }
            RecvType::Event(addr, bytes) => {
                let event: GroupEvent = postcard::from_bytes(&bytes)
                    .map_err(|_| new_io_error("deserialize event error."))?;

                if let Some(true) = self
                    .groups
                    .get(&gid)
                    .map(|(members, _)| members.contains(&addr))
                {
                    self.handle_event(&gid, event, &mut results)?;
                }
            }
            RecvType::Stream(_uid, _stream, _bytes) => {
                // TODO stream
            }
            RecvType::Delivery(_t, _tid, _is_ok) => {
                // TODO or not.
            }
        }

        Ok(results)
    }

    fn handle_event(
        &mut self,
        gid: &GroupId,
        gevent: GroupEvent,
        results: &mut HandleResult,
    ) -> Result<()> {
        let (members, height) = self.groups.get_mut(gid).ok_or(new_io_error("missing"))?;

        match gevent {
            GroupEvent::Online(addr) => {
                let new_data = postcard::to_allocvec(&GroupEvent::Online(addr))
                    .map_err(|_| new_io_error("serialize event error."))?;
                for member in members {
                    let s = SendType::Event(0, *member, new_data.clone());
                    add_layer(results, *gid, s);
                }
            }
            GroupEvent::Offline(addr) => {
                let new_data = postcard::to_allocvec(&GroupEvent::Offline(addr))
                    .map_err(|_| new_io_error("serialize event error."))?;
                for member in members {
                    let s = SendType::Event(0, *member, new_data.clone());
                    add_layer(results, *gid, s);
                }
            }
            GroupEvent::Sync(_, event) => {
                match event {
                    Event::Message => {
                        //
                    }
                    Event::GroupUpdate => {
                        //
                    }
                    Event::GroupTransfer => {
                        //
                    }
                    Event::UserInfo => {
                        //
                    }
                    Event::Close => {
                        //
                    }
                }

                let new_data = postcard::to_allocvec(&GroupEvent::Sync(*height + 1, event))
                    .map_err(|_| new_io_error("serialize event error."))?;
                *height += 1;
                for member in members {
                    let s = SendType::Event(0, *member, new_data.clone());
                    add_layer(results, *gid, s);
                }
            }
        }

        Ok(())
    }

    pub fn add_manager(&mut self, gid: GroupId, limit: u32) {
        self.managers.insert(gid, limit);
    }

    pub fn remove_manager(&mut self, gid: &GroupId) {
        self.managers.remove(gid);
    }
}