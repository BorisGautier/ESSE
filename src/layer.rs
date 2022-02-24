use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tdn::types::{
    group::GroupId,
    message::SendType,
    primitives::{HandleResult, Peer, PeerId, Result},
};
use tokio::sync::RwLock;

use crate::account::User;
//use crate::apps::chat::{chat_conn, LayerEvent as ChatLayerEvent};
//use crate::apps::group::{group_conn, GROUP_ID};
use crate::group::Group;
//use crate::session::{Session, SessionType};

/// ESSE app's `BaseLayerEvent`.
/// EVERY LAYER APP MUST EQUAL THE FIRST THREE FIELDS.
#[derive(Serialize, Deserialize)]
pub(crate) enum LayerEvent {
    /// Offline. params: remote_id.
    Offline(GroupId),
    /// Suspend. params: remote_id.
    Suspend(GroupId),
    /// Actived. params: remote_id.
    Actived(GroupId),
}

/// ESSE layers.
pub(crate) struct Layer {
    /// running layers: (Layer_gid, layer sessions)
    pub sessions: HashMap<GroupId, HashMap<PeerId, LayerSession>>,
}

impl Layer {
    pub fn init() -> Layer {
        // add all inner-service layers
        // add all third-service layers
        let mut sessions = HashMap::new();

        // runnings.insert(CHAT_GROUP_ID, RunningLayer::init());

        Layer { sessions }
    }

    pub fn clear(&mut self) {
        let _ = self.sessions.iter_mut().map(|(_, s)| s.clear());
    }

    // pub fn remove_running(&mut self, gid: &GroupId) -> HashMap<PeerId, GroupId> {
    //     // check close the stable connection.
    //     let mut addrs: HashMap<PeerId, GroupId> = HashMap::new();
    //     if let Some(running) = self.runnings.remove(gid) {
    //         for (addr, fgid) in running.remove_onlines() {
    //             addrs.insert(addr, fgid);
    //         }
    //     }

    //     let mut need_keep = vec![];
    //     for (_, running) in &self.runnings {
    //         for addr in addrs.keys() {
    //             if running.check_addr_online(addr) {
    //                 need_keep.push(*addr);
    //             }
    //         }
    //     }
    //     for i in need_keep {
    //         addrs.remove(&i);
    //     }

    //     addrs
    // }

    // pub fn remove_all_running(&mut self) -> HashMap<PeerId, GroupId> {
    //     let mut addrs: HashMap<PeerId, GroupId> = HashMap::new();
    //     for (_, running) in self.runnings.drain() {
    //         for (addr, fgid) in running.remove_onlines() {
    //             addrs.insert(addr, fgid);
    //         }
    //     }
    //     addrs
    // }

    // pub fn get_running_remote_id(&self, mgid: &GroupId, fgid: &GroupId) -> Result<(i64, i64)> {
    //     debug!("onlines: {:?}, find: {:?}", self.runnings.keys(), mgid);
    //     self.running(mgid)?.get_online_id(fgid)
    // }

    // pub fn remove_online(&mut self, gid: &GroupId, fgid: &GroupId) -> Option<PeerId> {
    //     self.running_mut(gid).ok()?.remove_online(fgid)
    // }

    // pub async fn all_layer_conns(&self) -> Result<HashMap<GroupId, Vec<(GroupId, SendType)>>> {
    //     let mut conns = HashMap::new();
    //     let group_lock = self.group.read().await;
    //     for mgid in self.runnings.keys() {
    //         let mut vecs = vec![];

    //         let db = group_lock.session_db(&mgid)?;
    //         let sessions = Session::list(&db)?;
    //         drop(db);

    //         for s in sessions {
    //             match s.s_type {
    //                 SessionType::Chat => {
    //                     let proof = group_lock.prove_addr(mgid, &s.addr)?;
    //                     vecs.push((s.gid, chat_conn(proof, Peer::peer(s.addr))));
    //                 }
    //                 SessionType::Group => {
    //                     let proof = group_lock.prove_addr(mgid, &s.addr)?;
    //                     vecs.push((GROUP_ID, group_conn(proof, Peer::peer(s.addr), s.gid)));
    //                 }
    //                 _ => {}
    //             }
    //         }

    //         conns.insert(*mgid, vecs);
    //     }

    //     Ok(conns)
    // }

    // pub fn is_addr_online(&self, faddr: &PeerId) -> bool {
    //     for (_, running) in &self.runnings {
    //         if running.check_addr_online(faddr) {
    //             return true;
    //         }
    //     }
    //     return false;
    // }

    // pub fn is_online(&self, gid: &GroupId, fgid: &GroupId) -> bool {
    //     if let Some(running) = self.runnings.get(gid) {
    //         running.is_online(fgid)
    //     } else {
    //         false
    //     }
    // }

    // pub fn broadcast(&self, user: User, results: &mut HandleResult) {
    //     let gid = user.id;
    //     let info = ChatLayerEvent::InfoRes(user);
    //     let data = bincode::serialize(&info).unwrap_or(vec![]);
    //     if let Some(running) = self.runnings.get(&gid) {
    //         for (fgid, online) in &running.sessions {
    //             let msg = SendType::Event(0, *online.online.addr(), data.clone());
    //             results.layers.push((gid, *fgid, msg));
    //         }
    //     }
    // }
}

/// online info.
#[derive(Eq, PartialEq)]
pub(crate) enum Online {
    /// connected to this device.
    Direct(PeerId),
    /// connected to other device.
    Relay(PeerId),
}

impl Online {
    fn addr(&self) -> &PeerId {
        match self {
            Online::Direct(ref addr) | Online::Relay(ref addr) => addr,
        }
    }
}

pub(crate) struct OnlineSession {
    pub online: Online,
    /// session database id.
    pub db_id: i64,
    /// session ref's service(friend/group) database id.
    pub db_fid: i64,
    pub suspend_me: bool,
    pub suspend_remote: bool,
    pub remain: u16, // keep-alive remain minutes
}

// impl OnlineSession {
//     fn new(online: Online, db_id: i64, db_fid: i64) -> Self {
//         Self {
//             online,
//             db_id,
//             db_fid,
//             suspend_me: false,
//             suspend_remote: false,
//             remain: 0,
//         }
//     }

//     fn close_suspend(&mut self) -> bool {
//         if self.suspend_me && self.suspend_remote {
//             if self.remain == 0 {
//                 true
//             } else {
//                 self.remain -= 1;
//                 false
//             }
//         } else {
//             false
//         }
//     }
// }

/// online connected layer session.
pub(crate) struct LayerSession {
    /// session online type.
    pub online: Online,
    /// current layer consensus(height).
    pub consensus: i64,
    /// session database id.
    pub s_id: i64,
    /// layer database id.
    pub db_id: i64,
    /// if session is suspend by me.
    pub suspend_me: bool,
    /// if session is suspend by remote.
    pub suspend_remote: bool,
    /// keep alive remain minutes.
    pub remain: u16,
}

impl LayerSession {
    // pub fn increased(&mut self) -> i64 {
    //     self.consensus += 1;
    //     self.consensus
    // }

    // pub fn active(&mut self, gid: &GroupId, is_me: bool) -> Option<PeerId> {
    //     if let Some(online) = self.sessions.get_mut(gid) {
    //         if is_me {
    //             online.suspend_me = false;
    //         } else {
    //             online.suspend_remote = false;
    //         }

    //         online.remain = 0;
    //         Some(*online.online.addr())
    //     } else {
    //         None
    //     }
    // }

    // pub fn suspend(&mut self, gid: &GroupId, is_me: bool, must: bool) -> Result<bool> {
    //     if let Some(online) = self.sessions.get_mut(gid) {
    //         if must {
    //             online.suspend_me = true;
    //             online.suspend_remote = true;
    //         }

    //         if is_me {
    //             online.suspend_me = true;
    //         } else {
    //             online.suspend_remote = true;
    //         }

    //         if online.suspend_remote && online.suspend_me {
    //             online.remain = 6; // keep-alive 10~11 minutes 120s/time
    //             Ok(true)
    //         } else {
    //             Ok(false)
    //         }
    //     } else {
    //         Err(anyhow!("remote not online"))
    //     }
    // }

    // pub fn get_online_id(&self, gid: &GroupId) -> Result<(i64, i64)> {
    //     debug!("onlines: {:?}, find: {:?}", self.sessions.keys(), gid);
    //     self.sessions
    //         .get(gid)
    //         .map(|online| (online.db_id, online.db_fid))
    //         .ok_or(anyhow!("remote not online"))
    // }

    // /// get online peer's addr.
    // pub fn online(&self, gid: &GroupId) -> Result<PeerId> {
    //     self.sessions
    //         .get(gid)
    //         .map(|online| *online.online.addr())
    //         .ok_or(anyhow!("remote not online"))
    // }

    // pub fn online_direct(&self, gid: &GroupId) -> Result<PeerId> {
    //     if let Some(online) = self.sessions.get(gid) {
    //         match online.online {
    //             Online::Direct(addr) => return Ok(addr),
    //             _ => {}
    //         }
    //     }
    //     Err(anyhow!("no direct online"))
    // }

    // /// get all online peer.
    // pub fn onlines(&self) -> Vec<(&GroupId, &PeerId)> {
    //     self.sessions
    //         .iter()
    //         .map(|(fgid, online)| (fgid, online.online.addr()))
    //         .collect()
    // }

    // pub fn is_online(&self, gid: &GroupId) -> bool {
    //     self.sessions.contains_key(gid)
    // }

    // /// check add online.
    // pub fn check_add_online(
    //     &mut self,
    //     gid: GroupId,
    //     online: Online,
    //     id: i64,
    //     fid: i64,
    // ) -> Result<()> {
    //     if let Some(o) = self.sessions.get(&gid) {
    //         match (&o.online, &online) {
    //             (Online::_Relay(..), Online::Direct(..)) => {
    //                 self.sessions
    //                     .insert(gid, OnlineSession::new(online, id, fid));
    //                 Ok(())
    //             }
    //             _ => Err(anyhow!("remote had online")),
    //         }
    //     } else {
    //         self.sessions
    //             .insert(gid, OnlineSession::new(online, id, fid));
    //         Ok(())
    //     }
    // }

    // /// check offline, and return is direct.
    // pub fn check_offline(&mut self, gid: &GroupId, addr: &PeerId) -> bool {
    //     if let Some(online) = self.sessions.remove(gid) {
    //         if online.online.addr() != addr {
    //             return false;
    //         }

    //         match online.online {
    //             Online::Direct(..) => {
    //                 return true;
    //             }
    //             _ => {}
    //         }
    //     }
    //     false
    // }

    // pub fn remove_online(&mut self, gid: &GroupId) -> Option<PeerId> {
    //     self.sessions
    //         .remove(gid)
    //         .map(|online| *online.online.addr())
    // }

    // /// remove all onlines peer.
    // pub fn remove_onlines(self) -> Vec<(PeerId, GroupId)> {
    //     let mut peers = vec![];
    //     for (fgid, online) in self.sessions {
    //         match online.online {
    //             Online::Direct(addr) => peers.push((addr, fgid)),
    //             _ => {}
    //         }
    //     }
    //     peers
    // }

    // /// check if addr is online.
    // pub fn check_addr_online(&self, addr: &PeerId) -> bool {
    //     for (_, online) in &self.sessions {
    //         if online.online.addr() == addr {
    //             return true;
    //         }
    //     }
    //     false
    // }

    // /// peer leave, remove online peer.
    // pub fn peer_leave(&mut self, addr: &PeerId) -> Vec<i64> {
    //     let mut peers = vec![];
    //     let mut deletes = vec![];
    //     for (fgid, online) in &self.sessions {
    //         if online.online.addr() == addr {
    //             peers.push(online.db_id);
    //             deletes.push(*fgid);
    //         }
    //     }
    //     for i in &deletes {
    //         self.sessions.remove(&i);
    //     }

    //     peers
    // }

    // /// list all onlines groups.
    // pub fn close_suspend(&mut self, self_addr: &PeerId) -> Vec<(GroupId, PeerId, i64)> {
    //     let mut needed = vec![];
    //     for (fgid, online) in &mut self.sessions {
    //         // when online is self. skip.
    //         if online.online == Online::Direct(*self_addr) {
    //             continue;
    //         }

    //         if online.close_suspend() {
    //             needed.push((*fgid, *online.online.addr(), online.db_id));
    //         }
    //     }

    //     for (gid, _, _) in needed.iter() {
    //         self.sessions.remove(gid);
    //     }
    //     needed
    // }
}
