use std::collections::HashMap;
use tdn::types::{
    group::GroupId,
    message::{RecvType, SendType},
    primitive::{new_io_error, HandleResult, PeerAddr, Result},
};

use group_chat_types::{
    CheckType, ConnectProof, Event, GroupInfo, GroupType, JoinProof, LayerConnect, LayerEvent,
    LayerResult, GROUP_CHAT_ID,
};

use crate::models::{Consensus, ConsensusType, GroupChat, Manager, Member, Message};

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
        // load managers
        let ms = Manager::all().await?;
        let mut managers = HashMap::new();
        for manager in ms {
            managers.insert(manager.gid, (manager.is_closed, manager.times));
        }

        // load groups
        let gs = GroupChat::all().await?;
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
                let LayerConnect(gcd, connect) = postcard::from_bytes(&data)
                    .map_err(|_e| new_io_error("deserialize group chat connect failure"))?;

                match connect {
                    ConnectProof::Common(proof) => {
                        let (height, fid) = self.height_and_fid(&gcd)?;
                        // check is member.

                        if Member::exist(&fid, &gid).await? {
                            self.add_member(&gcd, gid, addr);
                            Self::had_join(height, gcd, gid, addr, &mut results);

                            let new_data =
                                postcard::to_allocvec(&LayerEvent::MemberOnline(gcd, gid, addr))
                                    .map_err(|_| new_io_error("serialize event error."))?;
                            for (mid, maddr) in self.groups(&gcd)? {
                                let s = SendType::Event(0, *maddr, new_data.clone());
                                add_layer(&mut results, *mid, s);
                            }
                        } else {
                            let s = SendType::Result(0, addr, false, false, vec![]);
                            add_layer(&mut results, gid, s);
                        }
                    }
                    ConnectProof::Zkp(_proof) => {
                        //
                    }
                }
            }
            RecvType::Leave(addr) => {
                for (g, (members, _, _)) in self.groups.iter_mut() {
                    if let Some(pos) = members.iter().position(|(_, x)| x == &addr) {
                        let (mid, addr) = members.remove(pos);
                        let data = postcard::to_allocvec(&LayerEvent::MemberOffline(*g, mid))
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
                self.handle_event(gid, addr, event, &mut results).await?;
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

    async fn handle_event(
        &mut self,
        fmid: GroupId,
        addr: PeerAddr,
        gevent: LayerEvent,
        results: &mut HandleResult,
    ) -> Result<()> {
        match gevent {
            LayerEvent::Offline(gcd) => {
                if !self.is_online_addr(&gcd, &addr) {
                    return Ok(());
                }
                self.del_member(&gcd, &fmid);

                let new_data = postcard::to_allocvec(&LayerEvent::MemberOffline(gcd, fmid))
                    .map_err(|_| new_io_error("serialize event error."))?;

                for (mid, maddr) in self.groups(&gcd)? {
                    let s = SendType::Event(0, *maddr, new_data.clone());
                    add_layer(results, *mid, s);
                }
            }
            LayerEvent::Suspend(gcd) => {
                // TODO
            }
            LayerEvent::Actived(gcd) => {
                // TODO
            }
            LayerEvent::Check => {
                let supported = vec![GroupType::Encrypted, GroupType::Private, GroupType::Open];
                let res = if let Some((is_closed, limit)) = self.managers.get(&fmid) {
                    if *is_closed {
                        LayerEvent::CheckResult(CheckType::Suspend, supported)
                    } else if *limit > 0 {
                        LayerEvent::CheckResult(CheckType::Allow, supported)
                    } else {
                        LayerEvent::CheckResult(CheckType::None, supported)
                    }
                } else {
                    LayerEvent::CheckResult(CheckType::Deny, supported)
                };
                let data = postcard::to_allocvec(&res).unwrap_or(vec![]);
                let s = SendType::Event(0, addr, data);
                add_layer(results, fmid, s);
            }
            LayerEvent::Create(info, _proof) => {
                let supported = vec![GroupType::Encrypted, GroupType::Private, GroupType::Open];
                let (res, ok) = if let Some((is_closed, limit)) = self.managers.get(&fmid) {
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
                                let mut gc =
                                    GroupChat::new(owner, gcd, gt, name, bio, need_agree, vec![]);

                                gc.insert().await?;

                                // TODO save avatar.

                                // add frist member.
                                let mut mem = Member::new(gc.id, fmid, addr, m_name, true);
                                mem.insert().await?;
                                println!("add member ok");

                                self.create_group(gc.id, gcd, fmid, addr);
                                println!("add group ok");

                                self.add_height(&gcd, &mem.id, ConsensusType::MemberJoin)
                                    .await?;
                                println!("add consensus ok");
                                gcd
                            }
                            GroupInfo::Encrypted(gcd, ..) => gcd,
                        };
                        (LayerEvent::CreateResult(gcd, true), true)
                    } else {
                        (LayerEvent::CheckResult(CheckType::None, supported), false)
                    }
                } else {
                    (LayerEvent::CheckResult(CheckType::Deny, supported), false)
                };

                let data = postcard::to_allocvec(&res).unwrap_or(vec![]);
                let s = SendType::Event(0, addr, data);
                add_layer(results, fmid, s);
            }
            LayerEvent::Request(gcd, join_proof) => {
                // 1. check account is online, if not online, nothing.
                match join_proof {
                    JoinProof::Open(mname, mavatar) => {
                        let fid = self.fid(&gcd)?;

                        let group = GroupChat::get_id(&fid).await?;
                        // check is member.
                        if Member::exist(fid, &fmid).await? {
                            self.add_member(&gcd, fmid, addr);
                            self.agree_join(gcd, fmid, addr, group, results).await?;
                            return Ok(());
                        }

                        if group.g_type == GroupType::Open {
                            let mut m = Member::new(*fid, fmid, addr, mname, false);
                            m.insert().await?;

                            // TOOD save avatar.

                            self.add_member(&gcd, fmid, addr);
                            self.broadcast_join(&gcd, m, mavatar, results).await?;

                            // return join result.
                            self.agree_join(gcd, fmid, addr, group, results).await?;
                        } else {
                            // TODO add member request.
                        }
                    }
                    JoinProof::Link(link_gid, mname, mavatar) => {
                        let fid = self.fid(&gcd)?;

                        let group = GroupChat::get_id(&fid).await?;
                        // check is member.
                        if Member::exist(fid, &fmid).await? {
                            self.add_member(&gcd, fmid, addr);
                            self.agree_join(gcd, fmid, addr, group, results).await?;
                            return Ok(());
                        }

                        if !Member::exist(fid, &link_gid).await? {
                            // TODO add join result invite url lose efficacy.
                            return Ok(());
                        }

                        if group.is_need_agree {
                            // TODO add member request.
                        } else {
                            let mut m = Member::new(*fid, fmid, addr, mname, false);
                            m.insert().await?;

                            // TOOD save avatar.

                            self.add_member(&gcd, fmid, addr);
                            self.broadcast_join(&gcd, m, mavatar, results).await?;

                            // return join result.
                            self.agree_join(gcd, fmid, addr, group, results).await?;
                        }
                    }
                    JoinProof::Invite(invite_gid, _proof, mname, mavatar) => {
                        let fid = self.fid(&gcd)?;

                        let group = GroupChat::get_id(fid).await?;
                        // check is member.
                        if Member::exist(fid, &fmid).await? {
                            self.add_member(&gcd, fmid, addr);
                            self.agree_join(gcd, fmid, addr, group, results).await?;
                            return Ok(());
                        }

                        if !Member::is_manager(fid, &invite_gid).await? {
                            // TODO add join result invite url lose efficacy.
                            return Ok(());
                        }

                        // TODO check proof.

                        let mut m = Member::new(*fid, fmid, addr, mname, false);
                        m.insert().await?;

                        // TOOD save avatar.

                        self.add_member(&gcd, fmid, addr);
                        self.broadcast_join(&gcd, m, mavatar, results).await?;

                        // return join result.
                        self.agree_join(gcd, fmid, addr, group, results).await?;
                    }
                    JoinProof::Zkp(_proof) => {
                        // TOOD zkp join.
                    }
                }
            }
            LayerEvent::RequestResult(_gcd, _ok) => {
                // TODO
            }
            LayerEvent::Sync(gcd, _, event) => {
                println!("Start handle Event.");

                let fid = self.fid(&gcd)?;

                let (cid, ctype) = match &event {
                    Event::GroupInfo => {
                        // TODO
                        (0, ConsensusType::GroupInfo)
                    }
                    Event::GroupTransfer => {
                        // TODO
                        (0, ConsensusType::GroupTransfer)
                    }
                    Event::GroupManagerAdd => {
                        // TODO
                        (0, ConsensusType::GroupManagerAdd)
                    }
                    Event::GroupManagerDel => {
                        // TODO
                        (0, ConsensusType::GroupManagerDel)
                    }
                    Event::GroupClose => {
                        // TODO
                        (0, ConsensusType::GroupClose)
                    }
                    Event::MemberInfo(mid, maddr, mname, mavatar) => {
                        // TODO
                        (0, ConsensusType::MemberInfo)
                    }
                    Event::MemberLeave(mid) => {
                        // TODO
                        (0, ConsensusType::MemberLeave)
                    }
                    Event::MessageCreate(mid, nmsg, _) => {
                        let id = Message::from_network_message(&gcd, fid, mid, nmsg).await?;
                        (id, ConsensusType::MessageCreate)
                    }
                    Event::MemberJoin(..) => return Ok(()), // Never here.
                };

                let height = self.add_height(&gcd, &cid, ctype).await?;
                println!("Event broadcast");
                let new_data = postcard::to_allocvec(&LayerEvent::Sync(gcd, height, event))
                    .map_err(|_| new_io_error("serialize event error."))?;
                for (mid, maddr) in self.groups(&gcd)? {
                    let s = SendType::Event(0, *maddr, new_data.clone());
                    add_layer(results, *mid, s);
                }
            }
            LayerEvent::SyncReq(gcd, from) => {
                let (height, fid) = self.height_and_fid(&gcd)?;
                println!("Got sync request. height: {} from: {}", height, from);
                if height > from {
                    let to = if height - from > 100 {
                        from + 100
                    } else {
                        height
                    };
                    let packed = Consensus::pack(&fid, &from, &to).await?;
                    let event = LayerEvent::Packed(gcd, height, from, to, packed);
                    let data = postcard::to_allocvec(&event).unwrap_or(vec![]);
                    let s = SendType::Event(0, addr, data);
                    add_layer(results, fmid, s);
                    println!("Sended sync request results. from: {}, to: {}", from, to);
                }
            }
            LayerEvent::CheckResult(..) => {}   // Nerver here.
            LayerEvent::CreateResult(..) => {}  // Nerver here.
            LayerEvent::Agree(..) => {}         // Nerver here.
            LayerEvent::Reject(..) => {}        // Nerver here.
            LayerEvent::Packed(..) => {}        // Nerver here.
            LayerEvent::MemberOnline(..) => {}  // Nerver here.
            LayerEvent::MemberOffline(..) => {} // Never here.
        }

        Ok(())
    }

    fn fid(&self, gid: &GroupId) -> Result<&i64> {
        self.groups
            .get(gid)
            .map(|v| &v.2)
            .ok_or(new_io_error("Group missing"))
    }

    fn height(&self, gid: &GroupId) -> Result<i64> {
        self.groups
            .get(gid)
            .map(|v| v.1)
            .ok_or(new_io_error("Group missing"))
    }

    fn height_and_fid(&self, gid: &GroupId) -> Result<(i64, i64)> {
        self.groups
            .get(gid)
            .map(|v| (v.1, v.2))
            .ok_or(new_io_error("Group missing"))
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

    pub async fn add_height(
        &mut self,
        gid: &GroupId,
        cid: &i64,
        ctype: ConsensusType,
    ) -> Result<i64> {
        if let Some((_, height, fid)) = self.groups.get_mut(gid) {
            *height += 1;

            // save.
            Consensus::insert(fid, height, cid, &ctype).await?;
            GroupChat::add_height(fid, height).await?;

            Ok(*height)
        } else {
            Err(new_io_error("Group missing"))
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
            for (_, maddr) in members {
                if maddr == addr {
                    return true;
                }
            }
        }
        return false;
    }

    pub async fn broadcast_join(
        &mut self,
        gcd: &GroupId,
        member: Member,
        avatar: Vec<u8>,
        results: &mut HandleResult,
    ) -> Result<()> {
        println!("start broadcast join...");
        let height = self
            .add_height(gcd, &member.id, ConsensusType::MemberJoin)
            .await?;

        let datetime = member.datetime;
        let event = Event::MemberJoin(
            member.m_id,
            member.m_addr,
            member.m_name,
            avatar,
            member.datetime,
        );

        let new_data =
            postcard::to_allocvec(&LayerEvent::Sync(*gcd, height, event)).unwrap_or(vec![]);

        if let Some((members, _, _)) = self.groups.get(gcd) {
            for (mid, maddr) in members {
                let s = SendType::Event(0, *maddr, new_data.clone());
                add_layer(results, *mid, s);
            }
        }
        println!("over broadcast join...");

        Ok(())
    }

    fn had_join(
        height: i64,
        gcd: GroupId,
        gid: GroupId,
        addr: PeerAddr,
        results: &mut HandleResult,
    ) {
        let res = LayerResult(gcd, height);
        let data = postcard::to_allocvec(&res).unwrap_or(vec![]);
        let s = SendType::Result(0, addr, true, false, data);
        add_layer(results, gid, s);
    }

    async fn agree_join(
        &self,
        gcd: GroupId,
        gid: GroupId,
        addr: PeerAddr,
        group: GroupChat,
        results: &mut HandleResult,
    ) -> Result<()> {
        let gavatar = vec![]; // TOOD load group avatar.
        let group_info = group.to_group_info(gavatar);
        let res = LayerEvent::Agree(gcd, group_info);
        let d = postcard::to_allocvec(&res).unwrap_or(vec![]);
        let s = SendType::Event(0, addr, d);
        add_layer(results, gid, s);
        Ok(())
    }
}
