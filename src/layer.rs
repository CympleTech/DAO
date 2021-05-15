use std::collections::HashMap;
use tdn::types::{
    group::GroupId,
    message::{RecvType, SendType},
    primitive::{new_io_error, HandleResult, PeerAddr, Result},
};

use group_chat_types::{
    CheckType, Event, GroupConnect, GroupInfo, GroupResult, GroupType, JoinProof, LayerEvent,
    GROUP_CHAT_ID,
};

use crate::models::{GroupChat, Manager, Member};
use crate::storage;

/// Group chat server to ESSE.
#[inline]
pub fn add_layer(results: &mut HandleResult, gid: GroupId, msg: SendType) {
    results.layers.push((GROUP_CHAT_ID, gid, msg));
}

pub(crate) struct Layer {
    managers: HashMap<GroupId, (bool, i32)>,
    groups: HashMap<GroupId, (Vec<(GroupId, PeerAddr)>, i64, i64)>,
}

impl Layer {
    pub(crate) async fn new() -> Result<Layer> {
        let db = storage::INSTANCE.get().unwrap();

        // load managers
        let ms = Manager::all(&db.pool).await?;
        let mut managers = HashMap::new();
        for manager in ms {
            managers.insert(manager.gid, (manager.is_closed, manager.times));
        }

        // load groups
        let gs = GroupChat::all(&db.pool).await?;
        let mut groups = HashMap::new();
        for group in gs {
            groups.insert(group.g_id, (vec![], group.height, group.id));
        }

        Ok(Layer { managers, groups })
    }

    pub(crate) async fn handle(&mut self, gid: GroupId, msg: RecvType) -> Result<HandleResult> {
        let mut results = HandleResult::new();

        match msg {
            RecvType::Connect(addr, data) => {
                let connect = postcard::from_bytes(&data)
                    .map_err(|_e| new_io_error("deserialize group chat connect failure"))?;

                match connect {
                    GroupConnect::Check => {
                        let supported =
                            vec![GroupType::Encrypted, GroupType::Common, GroupType::Open];
                        let res = if let Some((is_closed, limit)) = self.managers.get(&gid) {
                            if !*is_closed && *limit > 0 {
                                GroupResult::Check(CheckType::Allow, supported)
                            } else {
                                GroupResult::Check(CheckType::None, supported)
                            }
                        } else {
                            GroupResult::Check(CheckType::Deny, supported)
                        };
                        let data = postcard::to_allocvec(&res).unwrap_or(vec![]);
                        let s = SendType::Result(0, addr, false, false, data);
                        add_layer(&mut results, gid, s);
                    }
                    GroupConnect::Create(info, _proof) => {
                        let supported =
                            vec![GroupType::Encrypted, GroupType::Common, GroupType::Open];
                        let (res, ok) = if let Some((is_closed, limit)) = self.managers.get(&gid) {
                            if !*is_closed && *limit > 0 {
                                // TODO check proof.
                                let gcd = match info {
                                    GroupInfo::Common(
                                        owner,
                                        m_name,
                                        gcd,
                                        gt,
                                        need_agree,
                                        name,
                                        bio,
                                        _avatar,
                                    ) => {
                                        let mut gc = GroupChat::new(
                                            owner,
                                            gcd,
                                            gt,
                                            name,
                                            bio,
                                            need_agree,
                                            vec![],
                                        );

                                        // save to db.
                                        let db = storage::INSTANCE.get().unwrap();
                                        gc.insert(&db.pool).await?;
                                        Member::new(gc.id, gid, addr, m_name, true)
                                            .insert(&db.pool)
                                            .await?;

                                        // TODO save avatar.

                                        self.create_group(gc.id, gcd, gid, addr);
                                        gcd
                                    }
                                    GroupInfo::Encrypted(gcd, ..) => gcd,
                                };
                                (GroupResult::Create(gcd, true), true)
                            } else {
                                (GroupResult::Check(CheckType::None, supported), false)
                            }
                        } else {
                            (GroupResult::Check(CheckType::Deny, supported), false)
                        };

                        let data = postcard::to_allocvec(&res).unwrap_or(vec![]);
                        let s = SendType::Result(0, addr, ok, false, data);
                        add_layer(&mut results, gid, s);
                    }
                    GroupConnect::Join(gcd, join_proof) => {
                        // 1. check account is online, if not online, nothing.
                        if let Some((_members, height, fid)) = self.groups.get_mut(&gcd) {
                            match join_proof {
                                JoinProof::Had(proof) => {
                                    // check is member.
                                    let db = storage::INSTANCE.get().unwrap();
                                    if Member::exist(&db.pool, &fid, &gid).await? {
                                        let res = GroupResult::Join(gcd, true, *height as u64);
                                        let data = postcard::to_allocvec(&res).unwrap_or(vec![]);
                                        let s = SendType::Result(0, addr, true, false, data);
                                        add_layer(&mut results, gid, s);
                                        self.add_member(&gcd, gid, addr);
                                    } else {
                                        let s = SendType::Result(0, addr, false, false, vec![]);
                                        add_layer(&mut results, gid, s);
                                    }
                                }
                                JoinProof::Link(_link_gid) => {
                                    // TODO
                                }
                                JoinProof::Invite(_invate_gid, _proof) => {
                                    // TODO
                                }
                                JoinProof::Zkp(_proof) => {
                                    // TOOD
                                }
                            }

                            //if members.contains(&addr) {
                            // TODO return OK.
                            // } else {
                            // TODO check proof.

                            // TODO boradcast online event.

                            // TODO sync events.
                            //}
                        }
                    }
                }
            }
            RecvType::Leave(addr) => {
                for (g, (members, _, _)) in self.groups.iter_mut() {
                    if let Some(pos) = members.iter().position(|(_, x)| x == &addr) {
                        let (mid, addr) = members.remove(pos);
                        let data = postcard::to_allocvec(&LayerEvent::MemberOffline(*g, mid, addr))
                            .map_err(|_| new_io_error("serialize event error."))?;
                        for (mid, maddr) in members {
                            let s = SendType::Event(0, *maddr, data.clone());
                            add_layer(&mut results, *mid, s);
                        }
                    }
                }
            }
            RecvType::Event(addr, bytes) => {
                println!("Got Event");
                let event: LayerEvent = postcard::from_bytes(&bytes)
                    .map_err(|_| new_io_error("deserialize event error."))?;
                self.handle_event(gid, addr, event, &mut results)?;
            }
            RecvType::Stream(_uid, _stream, _bytes) => {
                // TODO stream
            }
            RecvType::Result(..) => {}        // no-reach here.
            RecvType::ResultConnect(..) => {} // no-reach here.
            RecvType::Delivery(..) => {}      // no-reach here.
        }

        Ok(results)
    }

    fn handle_event(
        &mut self,
        fmid: GroupId,
        addr: PeerAddr,
        gevent: LayerEvent,
        results: &mut HandleResult,
    ) -> Result<()> {
        let gcd = match gevent {
            LayerEvent::Offline(gcd)
            | LayerEvent::OnlinePing(gcd)
            | LayerEvent::OnlinePong(gcd)
            | LayerEvent::MemberOnline(gcd, ..)
            | LayerEvent::MemberOffline(gcd, ..)
            | LayerEvent::Sync(gcd, ..) => gcd,
        };

        println!("Check online.");
        if !self.is_online_addr(&gcd, &addr) {
            return Ok(());
        }
        println!("Check online ok.");

        match gevent {
            LayerEvent::Offline(gcd) => {
                self.del_member(&gcd, &fmid);

                let new_data = postcard::to_allocvec(&LayerEvent::MemberOffline(gcd, fmid, addr))
                    .map_err(|_| new_io_error("serialize event error."))?;

                for (mid, maddr) in self.groups(&gcd)? {
                    let s = SendType::Event(0, *maddr, new_data.clone());
                    add_layer(results, *mid, s);
                }
            }
            LayerEvent::OnlinePing(gcd) => {
                self.add_member(&gcd, fmid, addr);

                let new_data = postcard::to_allocvec(&LayerEvent::OnlinePong(gcd))
                    .map_err(|_| new_io_error("serialize event error."))?;
                let s = SendType::Event(0, addr, new_data.clone());
                add_layer(results, fmid, s);

                let new_data = postcard::to_allocvec(&LayerEvent::MemberOnline(gcd, fmid, addr))
                    .map_err(|_| new_io_error("serialize event error."))?;
                for (mid, maddr) in self.groups(&gcd)? {
                    let s = SendType::Event(0, *maddr, new_data.clone());
                    add_layer(results, *mid, s);
                }
            }

            LayerEvent::OnlinePong(gcd) => {
                self.add_member(&gcd, fmid, addr);

                let new_data = postcard::to_allocvec(&LayerEvent::MemberOnline(gcd, fmid, addr))
                    .map_err(|_| new_io_error("serialize event error."))?;
                for (mid, maddr) in self.groups(&gcd)? {
                    let s = SendType::Event(0, *maddr, new_data.clone());
                    add_layer(results, *mid, s);
                }
            }
            LayerEvent::Sync(gcd, _, event) => {
                println!("Start handle Event.");
                let height = self.add_height(&gcd);

                match &event {
                    Event::Message(..) => {
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

                println!("Event broadcast");
                let new_data = postcard::to_allocvec(&LayerEvent::Sync(gcd, height as u64, event))
                    .map_err(|_| new_io_error("serialize event error."))?;
                for (mid, maddr) in self.groups(&gcd)? {
                    let s = SendType::Event(0, *maddr, new_data.clone());
                    add_layer(results, *mid, s);
                }
            }
            LayerEvent::MemberOnline(..) => {}  // Nerver here.
            LayerEvent::MemberOffline(..) => {} // Never here.
        }

        Ok(())
    }

    fn groups(&self, gid: &GroupId) -> Result<&Vec<(GroupId, PeerAddr)>> {
        self.groups
            .get(gid)
            .map(|v| &v.0)
            .ok_or(new_io_error("Group missing"))
    }

    pub fn add_manager(&mut self, gid: GroupId, limit: i32) {
        self.managers.insert(gid, (false, limit));
    }

    pub fn remove_manager(&mut self, gid: &GroupId) {
        self.managers.remove(gid);
    }

    pub fn create_group(&mut self, id: i64, gid: GroupId, rid: GroupId, raddr: PeerAddr) {
        self.groups.insert(gid, (vec![(rid, raddr)], 0, id));
    }

    pub fn add_height(&mut self, gid: &GroupId) -> i64 {
        if let Some((_, height, _)) = self.groups.get_mut(gid) {
            *height += 1;
            *height
        } else {
            0
        }
    }

    pub fn add_member(&mut self, gid: &GroupId, rid: GroupId, raddr: PeerAddr) {
        if let Some((members, _, _)) = self.groups.get_mut(gid) {
            for (mid, maddr) in members.iter_mut() {
                if *mid == rid {
                    *maddr = raddr;
                    return;
                }
            }
            members.push((rid, raddr));
        }
    }

    pub fn del_member(&mut self, gid: &GroupId, rid: &GroupId) {
        if let Some((members, _, _)) = self.groups.get_mut(gid) {
            if let Some(pos) = members.iter().position(|(mid, _)| mid == rid) {
                members.remove(pos);
            }
        }
    }

    pub fn is_online_addr(&self, gid: &GroupId, addr: &PeerAddr) -> bool {
        if let Some((members, _, _)) = self.groups.get(gid) {
            println!("{:?}", members);
            for (_, maddr) in members {
                if maddr == addr {
                    return true;
                }
            }
        }
        return false;
    }
}
