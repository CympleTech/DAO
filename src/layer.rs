use std::collections::HashMap;
use std::path::PathBuf;
use tdn::types::{
    group::GroupId,
    message::{RecvType, SendType},
    primitive::{HandleResult, PeerAddr, Result},
};

use group_chat_types::{
    CheckType, ConnectProof, Event, GroupInfo, GroupType, JoinProof, LayerConnect, LayerEvent,
    LayerResult, GROUP_CHAT_ID,
};

use crate::manager::Manager;
use crate::models::{Consensus, ConsensusType, GroupChat, Member, Message, Request};
use crate::storage::{delete_avatar, init_local_files, read_avatar, write_avatar};
use crate::{DEFAULT_REMAIN, NAME, PERMISSIONLESS, SUPPORTED};

/// Group chat server to ESSE.
#[inline]
pub fn add_layer(results: &mut HandleResult, gid: GroupId, msg: SendType) {
    results.layers.push((GROUP_CHAT_ID, gid, msg));
}

pub(crate) struct Layer {
    base: PathBuf,
    /// running groups, with members info.
    /// params: online members (member id, member address, is manager), current height, db id.
    groups: HashMap<GroupId, (Vec<(GroupId, PeerAddr, bool)>, i64, i64)>,
}

impl Layer {
    pub(crate) async fn new(base: PathBuf) -> Result<Layer> {
        // load groups
        let gs = GroupChat::all().await?;
        let mut groups = HashMap::new();
        for group in gs {
            groups.insert(group.g_id, (vec![], group.height, group.id));
        }

        Ok(Layer { base, groups })
    }

    pub(crate) async fn handle(&mut self, gid: GroupId, msg: RecvType) -> Result<HandleResult> {
        let mut results = HandleResult::new();

        match msg {
            RecvType::Connect(addr, data) => {
                let LayerConnect(gcd, connect) = bincode::deserialize(&data)
                    .map_err(|_e| anyhow!("deserialize group chat connect failure"))?;

                match connect {
                    ConnectProof::Common(proof) => {
                        let (height, fid) = self.height_and_fid(&gcd)?;
                        // check is member.

                        if Member::exist(&fid, &gid).await? {
                            self.add_member(&gcd, gid, addr);
                            Self::had_join(height, gcd, gid, addr, &mut results);

                            let new_data =
                                bincode::serialize(&LayerEvent::MemberOnline(gcd, gid, addr))
                                    .map_err(|_| anyhow!("serialize event error."))?;
                            for (mid, maddr, _) in self.groups(&gcd)? {
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
                    if let Some(pos) = members.iter().position(|(_, x, _)| x == &addr) {
                        let (mid, addr, _) = members.remove(pos);
                        let data = bincode::serialize(&LayerEvent::MemberOffline(*g, mid))
                            .map_err(|_| anyhow!("serialize event error."))?;
                        for (mid, maddr, _) in members {
                            let s = SendType::Event(0, *maddr, data.clone());
                            add_layer(&mut results, *mid, s);
                        }
                    }
                }
            }
            RecvType::Event(addr, bytes) => {
                println!("Got Event");
                let event: LayerEvent = bincode::deserialize(&bytes)
                    .map_err(|_| anyhow!("deserialize event error."))?;
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
                if !self.is_online_member(&gcd, &fmid) {
                    return Ok(());
                }
                self.del_member(&gcd, &fmid);

                let new_data = bincode::serialize(&LayerEvent::MemberOffline(gcd, fmid))
                    .map_err(|_| anyhow!("serialize event error."))?;

                for (mid, maddr, _) in self.groups(&gcd)? {
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
                let (t, r) = if let Ok(manager) = Manager::get(&fmid).await {
                    if manager.is_closed {
                        (CheckType::Suspend, manager.times)
                    } else if manager.times > 0 {
                        (CheckType::Allow, manager.times)
                    } else {
                        (CheckType::None, 0)
                    }
                } else {
                    if PERMISSIONLESS {
                        (CheckType::Allow, DEFAULT_REMAIN)
                    } else {
                        (CheckType::Deny, 0)
                    }
                };
                let res = LayerEvent::CheckResult(t, NAME.to_owned(), r as i64, SUPPORTED.to_vec());
                let data = bincode::serialize(&res).unwrap_or(vec![]);
                let s = SendType::Event(0, addr, data);
                add_layer(results, fmid, s);
            }
            LayerEvent::Create(info, _proof) => {
                let manager = if let Ok(manager) = Manager::get(&fmid).await {
                    manager
                } else {
                    if PERMISSIONLESS {
                        // add manager.
                        let mut manager = Manager::new(fmid);
                        manager.insert().await?;
                        manager
                    } else {
                        // return Deny to outside.
                        let res =
                            LayerEvent::CheckResult(CheckType::Deny, "".to_owned(), 0, vec![]);
                        let data = bincode::serialize(&res).unwrap_or(vec![]);
                        let s = SendType::Event(0, addr, data);
                        add_layer(results, fmid, s);
                        return Ok(());
                    }
                };

                if manager.is_closed {
                    let res = LayerEvent::CheckResult(
                        CheckType::Suspend,
                        NAME.to_owned(),
                        manager.times as i64,
                        SUPPORTED.to_vec(),
                    );
                    let data = bincode::serialize(&res).unwrap_or(vec![]);
                    let s = SendType::Event(0, addr, data);
                    add_layer(results, fmid, s);
                    return Ok(());
                }

                if manager.times == 0 {
                    let res = LayerEvent::CheckResult(
                        CheckType::None,
                        NAME.to_owned(),
                        0,
                        SUPPORTED.to_vec(),
                    );
                    let data = bincode::serialize(&res).unwrap_or(vec![]);
                    let s = SendType::Event(0, addr, data);
                    add_layer(results, fmid, s);
                    return Ok(());
                }

                // TODO check proof.
                let gcd = match info {
                    GroupInfo::Common(
                        owner,
                        owner_name,
                        owner_avatar,
                        gcd,
                        gt,
                        need_agree,
                        name,
                        bio,
                        avatar,
                    ) => {
                        let mut gc = GroupChat::new(owner, gcd, gt, name, bio, need_agree, vec![]);

                        gc.insert().await?;

                        let _ = init_local_files(&self.base, &gc.g_id).await;
                        let _ = write_avatar(&self.base, &gc.g_id, &gc.g_id, &avatar).await;

                        // add frist member.
                        let mut mem = Member::new(gc.id, owner, addr, owner_name, true);
                        mem.insert().await?;
                        // save member avatar.
                        let _ = write_avatar(&self.base, &gc.g_id, &mem.m_id, &owner_avatar).await;
                        println!("add member ok");

                        // reduce manager remain.
                        let _ = manager.reduce().await;

                        self.create_group(gc.id, gcd, fmid, addr);
                        println!("add group ok");

                        self.add_height(&gcd, &mem.id, ConsensusType::MemberJoin)
                            .await?;
                        println!("add consensus ok");
                        gcd
                    }
                    GroupInfo::Encrypted(gcd, ..) => gcd,
                };

                let res = LayerEvent::CreateResult(gcd, true);
                let data = bincode::serialize(&res).unwrap_or(vec![]);
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
                            self.agree(gcd, fmid, addr, group, results).await?;
                            return Ok(());
                        }

                        if group.g_type == GroupType::Open {
                            let mut m = Member::new(*fid, fmid, addr, mname, false);
                            m.insert().await?;

                            // save avatar.
                            let _ = write_avatar(&self.base, &gcd, &m.m_id, &mavatar).await;

                            self.add_member(&gcd, fmid, addr);
                            self.broadcast_join(&gcd, m, mavatar, results).await?;

                            // return join result.
                            self.agree(gcd, fmid, addr, group, results).await?;
                        } else {
                            Self::reject(gcd, fmid, addr, false, results);
                        }
                    }
                    JoinProof::Invite(invite_gid, proof, mname, mavatar) => {
                        let fid = self.fid(&gcd)?;
                        let group = GroupChat::get_id(fid).await?;

                        // check is member.
                        if Member::exist(fid, &fmid).await? {
                            self.add_member(&gcd, fmid, addr);
                            self.agree(gcd, fmid, addr, group, results).await?;
                            return Ok(());
                        }

                        // TODO check if request had or is blocked by manager.

                        // check if inviter is member.
                        if !Member::exist(fid, &invite_gid).await? {
                            Self::reject(gcd, fmid, addr, true, results);
                            return Ok(());
                        }

                        // TODO check proof.
                        // proof.verify(&invite_gid, &addr, &layer.addr)?;

                        if group.is_need_agree {
                            if !Member::is_manager(fid, &invite_gid).await? {
                                let mut request = Request::new();
                                request.insert().await?;
                                self.broadcast_request(
                                    &gcd,
                                    request,
                                    JoinProof::Invite(invite_gid, proof, mname, mavatar),
                                    results,
                                );
                                return Ok(());
                            }
                        }

                        let mut m = Member::new(*fid, fmid, addr, mname, false);
                        m.insert().await?;

                        // save avatar.
                        let _ = write_avatar(&self.base, &gcd, &m.m_id, &mavatar).await;

                        self.add_member(&gcd, fmid, addr);
                        self.broadcast_join(&gcd, m, mavatar, results).await?;

                        // return join result.
                        self.agree(gcd, fmid, addr, group, results).await?;
                    }
                    JoinProof::Zkp(_proof) => {
                        // TOOD zkp join.
                    }
                }
            }
            LayerEvent::RequestResult(gcd, rid, ok) => {
                let fid = self.fid(&gcd)?;

                if Member::is_manager(fid, &fmid).await? {
                    let request = Request::get(&rid).await?;
                    if &request.fid == fid {
                        if ok {
                            let group = GroupChat::get_id(fid).await?;

                            let mut m = request.to_member();
                            m.insert().await?;

                            self.add_member(&gcd, m.m_id, m.m_addr);
                            self.agree(gcd, m.m_id, m.m_addr, group, results).await?;

                            let mavatar = read_avatar(&self.base, &gcd, &fmid).await?;
                            self.broadcast_join(&gcd, m, mavatar, results).await?;
                        } else {
                            Self::reject(gcd, request.m_id, request.m_addr, true, results);
                        }
                    }
                    self.broadcast_request_result(&gcd, rid, ok, results);
                }
            }
            LayerEvent::Sync(gcd, _, event) => {
                println!("Start handle Event.");

                if !self.is_online_member(&gcd, &fmid) {
                    return Ok(());
                }

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
                        let member = Member::get(fid, mid).await?;
                        // TODO
                        (member.id, ConsensusType::MemberInfo)
                    }
                    Event::MemberLeave(mid) => {
                        let member = Member::get(fid, mid).await?;
                        member.leave().await?;
                        let _ = delete_avatar(&self.base, &gcd, &mid).await;
                        (member.id, ConsensusType::MemberLeave)
                    }
                    Event::MessageCreate(mid, nmsg, _) => {
                        let id =
                            Message::from_network_message(&self.base, &gcd, fid, mid, nmsg).await?;
                        (id, ConsensusType::MessageCreate)
                    }
                    Event::MemberJoin(..) => return Ok(()), // Never here.
                };

                let height = self.add_height(&gcd, &cid, ctype).await?;
                println!("Event broadcast");
                let new_data = bincode::serialize(&LayerEvent::Sync(gcd, height, event))
                    .map_err(|_| anyhow!("serialize event error."))?;
                for (mid, maddr, _) in self.groups(&gcd)? {
                    let s = SendType::Event(0, *maddr, new_data.clone());
                    add_layer(results, *mid, s);
                }
            }
            LayerEvent::SyncReq(gcd, from) => {
                if !self.is_online_member(&gcd, &fmid) {
                    return Ok(());
                }

                let (height, fid) = self.height_and_fid(&gcd)?;
                println!("Got sync request. height: {} from: {}", height, from);
                if height >= from {
                    let to = if height - from > 100 {
                        from + 100
                    } else {
                        height
                    };
                    let packed = Consensus::pack(&self.base, &gcd, &fid, &from, &to).await?;
                    let event = LayerEvent::Packed(gcd, height, from, to, packed);
                    let data = bincode::serialize(&event).unwrap_or(vec![]);
                    let s = SendType::Event(0, addr, data);
                    add_layer(results, fmid, s);
                    println!("Sended sync request results. from: {}, to: {}", from, to);
                }
            }
            LayerEvent::MemberOnlineSync(gcd) => {
                if !self.is_online_member(&gcd, &fmid) {
                    return Ok(());
                }

                let onlines = self.onlines(&gcd)?;
                let event = LayerEvent::MemberOnlineSyncResult(gcd, onlines);
                let data = bincode::serialize(&event).unwrap_or(vec![]);
                let s = SendType::Event(0, addr, data);
                add_layer(results, fmid, s);
            }
            LayerEvent::MemberOnlineSyncResult(..) => {} // Nerver here.
            LayerEvent::CheckResult(..) => {}            // Nerver here.
            LayerEvent::CreateResult(..) => {}           // Nerver here.
            LayerEvent::RequestHandle(..) => {}          // Nerver here.
            LayerEvent::Agree(..) => {}                  // Nerver here.
            LayerEvent::Reject(..) => {}                 // Nerver here.
            LayerEvent::Packed(..) => {}                 // Nerver here.
            LayerEvent::MemberOnline(..) => {}           // Nerver here.
            LayerEvent::MemberOffline(..) => {}          // Never here.
        }

        Ok(())
    }

    fn fid(&self, gid: &GroupId) -> Result<&i64> {
        self.groups
            .get(gid)
            .map(|v| &v.2)
            .ok_or(anyhow!("Group missing"))
    }

    fn height(&self, gid: &GroupId) -> Result<i64> {
        self.groups
            .get(gid)
            .map(|v| v.1)
            .ok_or(anyhow!("Group missing"))
    }

    fn height_and_fid(&self, gid: &GroupId) -> Result<(i64, i64)> {
        self.groups
            .get(gid)
            .map(|v| (v.1, v.2))
            .ok_or(anyhow!("Group missing"))
    }

    fn groups(&self, gid: &GroupId) -> Result<&Vec<(GroupId, PeerAddr, bool)>> {
        self.groups
            .get(gid)
            .map(|v| &v.0)
            .ok_or(anyhow!("Group missing"))
    }

    fn onlines(&self, gid: &GroupId) -> Result<Vec<(GroupId, PeerAddr)>> {
        self.groups
            .get(gid)
            .map(|v| v.0.iter().map(|(g, a, _)| (*g, *a)).collect())
            .ok_or(anyhow!("Group missing"))
    }

    pub fn create_group(&mut self, id: i64, gid: GroupId, rid: GroupId, raddr: PeerAddr) {
        self.groups.insert(gid, (vec![(rid, raddr, true)], 0, id));
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
            Err(anyhow!("Group missing"))
        }
    }

    pub fn add_member(&mut self, gid: &GroupId, rid: GroupId, raddr: PeerAddr) {
        if let Some((members, _, _)) = self.groups.get_mut(gid) {
            for (mid, maddr, is_m) in members.iter_mut() {
                if *mid == rid {
                    *maddr = raddr;
                    return;
                }
            }
            members.push((rid, raddr, false));
        }
    }

    pub fn del_member(&mut self, gid: &GroupId, rid: &GroupId) {
        if let Some((members, _, _)) = self.groups.get_mut(gid) {
            if let Some(pos) = members.iter().position(|(mid, _, _)| mid == rid) {
                members.remove(pos);
            }
        }
    }

    pub fn is_online_member(&self, gid: &GroupId, mid: &GroupId) -> bool {
        if let Some((members, _, _)) = self.groups.get(gid) {
            for (mmid, _, _) in members {
                if mmid == mid {
                    return true;
                }
            }
        }

        false
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

        let new_data = bincode::serialize(&LayerEvent::Sync(*gcd, height, event)).unwrap_or(vec![]);

        if let Some((members, _, _)) = self.groups.get(gcd) {
            for (mid, maddr, _) in members {
                let s = SendType::Event(0, *maddr, new_data.clone());
                add_layer(results, *mid, s);
            }
        }
        println!("over broadcast join...");

        Ok(())
    }

    fn broadcast_request(
        &self,
        gcd: &GroupId,
        request: Request,
        join: JoinProof,
        results: &mut HandleResult,
    ) {
        println!("start broadcast request...");
        let event = LayerEvent::RequestHandle(
            *gcd,
            request.m_id,
            request.m_addr,
            join,
            request.id,
            request.datetime,
        );
        let new_data = bincode::serialize(&event).unwrap_or(vec![]);

        if let Some((members, _, _)) = self.groups.get(gcd) {
            for (mid, maddr, is_m) in members {
                if *is_m {
                    let s = SendType::Event(0, *maddr, new_data.clone());
                    add_layer(results, *mid, s);
                }
            }
        }
    }

    fn broadcast_request_result(
        &self,
        gcd: &GroupId,
        rid: i64,
        ok: bool,
        results: &mut HandleResult,
    ) {
        println!("start broadcast request result...");
        let new_data =
            bincode::serialize(&LayerEvent::RequestResult(*gcd, rid, ok)).unwrap_or(vec![]);

        if let Some((members, _, _)) = self.groups.get(gcd) {
            for (mid, maddr, is_m) in members {
                if *is_m {
                    let s = SendType::Event(0, *maddr, new_data.clone());
                    add_layer(results, *mid, s);
                }
            }
        }
    }

    fn had_join(
        height: i64,
        gcd: GroupId,
        gid: GroupId,
        addr: PeerAddr,
        results: &mut HandleResult,
    ) {
        let res = LayerResult(gcd, height);
        let data = bincode::serialize(&res).unwrap_or(vec![]);
        let s = SendType::Result(0, addr, true, false, data);
        add_layer(results, gid, s);
    }

    async fn agree(
        &self,
        gcd: GroupId,
        gid: GroupId,
        addr: PeerAddr,
        group: GroupChat,
        results: &mut HandleResult,
    ) -> Result<()> {
        let gavatar = read_avatar(&self.base, &gcd, &gcd).await?;
        let group_info = group.to_group_info(gavatar);
        let res = LayerEvent::Agree(gcd, group_info);
        let d = bincode::serialize(&res).unwrap_or(vec![]);
        let s = SendType::Event(0, addr, d);
        add_layer(results, gid, s);
        Ok(())
    }

    fn reject(gcd: GroupId, gid: GroupId, addr: PeerAddr, lost: bool, res: &mut HandleResult) {
        let d = bincode::serialize(&LayerEvent::Reject(gcd, lost)).unwrap_or(vec![]);
        add_layer(res, gid, SendType::Event(0, addr, d));
    }
}
