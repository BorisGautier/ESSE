use std::path::PathBuf;
use std::sync::Arc;
use tdn::{
    smol::lock::RwLock,
    types::{
        group::GroupId,
        message::{RecvType, SendType},
        primitive::{new_io_error, HandleResult, PeerAddr, Result},
    },
};

use group_chat_types::{Event, GroupConnect, GroupResult, JoinProof, LayerEvent, PackedEvent};
use tdn_did::Proof;
use tdn_storage::local::DStorage;

use crate::layer::{Layer, Online};
use crate::rpc::{session_connect, session_lost, session_suspend};
use crate::storage::{group_chat_db, write_avatar_sync};

use super::models::{from_network_message, GroupChat, Member, Request};
use super::{add_layer, rpc};

pub(crate) async fn handle(
    layer: &Arc<RwLock<Layer>>,
    mgid: GroupId,
    msg: RecvType,
) -> Result<HandleResult> {
    let mut results = HandleResult::new();

    match msg {
        RecvType::Connect(..) => {} // Never to here.
        RecvType::Leave(..) => {}   // Never to here. handled in chat.
        RecvType::Result(addr, _is_ok, data) => {
            let res: GroupResult = postcard::from_bytes(&data)
                .map_err(|_e| new_io_error("Deseralize result failure"))?;
            match res {
                GroupResult::Check(ct, supported) => {
                    println!("check: {:?}, supported: {:?}", ct, supported);
                    results.rpcs.push(rpc::create_check(mgid, ct, supported))
                }
                GroupResult::Create(gcd, ok) => {
                    println!("Create result: {}", ok);
                    if ok {
                        // get gc by gcd.
                        let db = group_chat_db(layer.read().await.base(), &mgid)?;
                        if let Some(mut gc) = GroupChat::get(&db, &gcd)? {
                            gc.ok(&db)?;
                            results.rpcs.push(rpc::create_result(mgid, gc.id, ok));

                            // 0. get session. TODO
                            let sid = 0;

                            // online this group.
                            layer.write().await.running_mut(&mgid)?.check_add_online(
                                gcd,
                                Online::Direct(addr),
                                sid,
                                gc.id,
                            )?;
                        }
                    }
                }
                GroupResult::Join(gcd, ok, height) => {
                    println!("Got join result: {}", ok);
                    if ok {
                        let base = layer.read().await.base.clone();
                        if let Some(group) = load_group(&base, &mgid, &gcd)? {
                            let mut layer_lock = layer.write().await;
                            // 1. check address.
                            if group.g_addr != addr {
                                return Ok(results);
                            }

                            // 2. get group session.
                            let sid = 0; // TODO

                            // 2. online this group.
                            layer_lock.running_mut(&mgid)?.check_add_online(
                                gcd,
                                Online::Direct(addr),
                                sid,
                                group.id,
                            )?;
                            // 3. online to UI.
                            results.rpcs.push(rpc::group_online(mgid, group.id));

                            // 4. online ping.
                            add_layer(&mut results, mgid, ping(gcd, addr));

                            println!("will sync remote: {}, my: {}", height, group.height);

                            // 5. sync group height.
                            if group.height < height {
                                add_layer(&mut results, mgid, sync(gcd, addr, group.height));
                            }
                        } else {
                            let msg = SendType::Result(0, addr, false, false, vec![]);
                            add_layer(&mut results, mgid, msg);
                            return Ok(results);
                        }
                    }
                }
                GroupResult::Waiting(_gcd) => {
                    // TODO waiting
                }
                GroupResult::Agree(gcd, info) => {
                    println!("Agree..........");
                    let base = layer.read().await.base.clone();
                    let db = group_chat_db(&base, &mgid)?;
                    let (rid, key) = Request::over(&db, &gcd, true)?;

                    // 1. add group chat.
                    let mut group = GroupChat::from_info(key, info, 0, addr, base, &mgid)?;
                    group.insert(&db)?;

                    // 2. update UI.
                    results.rpcs.push(rpc::group_agree(mgid, rid, group));

                    // 3. online ping.
                    add_layer(&mut results, mgid, ping(gcd, addr));

                    // 4. sync group height.
                    add_layer(&mut results, mgid, sync(gcd, addr, 0));
                }
                GroupResult::Reject(gcd) => {
                    println!("Reject..........");
                    let db = group_chat_db(layer.read().await.base(), &mgid)?;
                    let (rid, _key) = Request::over(&db, &gcd, true)?;
                    results.rpcs.push(rpc::group_reject(mgid, rid));
                }
            }
        }
        RecvType::ResultConnect(_addr, data) => {
            let _res: GroupResult = postcard::from_bytes(&data)
                .map_err(|_e| new_io_error("Deseralize result failure"))?;
        }
        RecvType::Event(addr, bytes) => {
            let event: LayerEvent =
                postcard::from_bytes(&bytes).map_err(|_| new_io_error("serialize event error."))?;
            handle_event(mgid, addr, event, layer, &mut results).await?;
        }
        RecvType::Stream(_uid, _stream, _bytes) => {
            // TODO stream
        }
        RecvType::Delivery(_t, _tid, _is_ok) => {
            //
        }
    }

    Ok(results)
}

async fn handle_event(
    mgid: GroupId,
    addr: PeerAddr,
    event: LayerEvent,
    layer: &Arc<RwLock<Layer>>,
    results: &mut HandleResult,
) -> Result<()> {
    println!("Got event.......");
    let (sid, gid) = match event {
        LayerEvent::Offline(gcd)
        | LayerEvent::Suspend(gcd)
        | LayerEvent::Actived(gcd)
        | LayerEvent::OnlinePing(gcd)
        | LayerEvent::OnlinePong(gcd)
        | LayerEvent::MemberOnline(gcd, ..)
        | LayerEvent::MemberOffline(gcd, ..)
        | LayerEvent::Sync(gcd, ..)
        | LayerEvent::SyncReq(gcd, ..)
        | LayerEvent::Packed(gcd, ..) => layer.read().await.get_running_remote_id(&mgid, &gcd)?,
    };

    match event {
        LayerEvent::Offline(gcd) => {
            let mut layer_lock = layer.write().await;
            layer_lock.running_mut(&mgid)?.check_offline(&gcd, &addr);
            drop(layer_lock);
            results.rpcs.push(session_lost(mgid, &sid));
        }
        LayerEvent::Suspend(gcd) => {
            let mut layer_lock = layer.write().await;
            if layer_lock.running_mut(&mgid)?.suspend(&gcd, false)? {
                results.rpcs.push(session_suspend(mgid, &sid));
            }
            drop(layer_lock);
        }
        LayerEvent::Actived(gcd) => {
            let mut layer_lock = layer.write().await;
            let _ = layer_lock.running_mut(&mgid)?.active(&gcd, false);
            drop(layer_lock);
            results.rpcs.push(session_connect(mgid, &sid, &addr));
        }
        LayerEvent::OnlinePing(gcd) => {
            results.rpcs.push(rpc::group_online(mgid, gid));
            let data = postcard::to_allocvec(&LayerEvent::OnlinePong(gcd)).unwrap_or(vec![]);
            let msg = SendType::Event(0, addr, data);
            add_layer(results, mgid, msg);
        }

        LayerEvent::OnlinePong(_) => {
            results.rpcs.push(rpc::group_online(mgid, gid));
        }
        LayerEvent::MemberOnline(_, mid, maddr) => {
            results.rpcs.push(rpc::member_online(mgid, gid, mid, maddr));
        }
        LayerEvent::MemberOffline(_, mid, ma) => {
            results.rpcs.push(rpc::member_offline(mgid, gid, mid, ma));
        }
        LayerEvent::Sync(_, height, event) => {
            println!("Sync: height: {}", height);
            let base = layer.read().await.base().clone();
            let db = group_chat_db(&base, &mgid)?;

            match event {
                Event::GroupInfo => {}
                Event::GroupTransfer => {}
                Event::GroupManagerAdd => {}
                Event::GroupManagerDel => {}
                Event::GroupClose => {}
                Event::MemberInfo(mid, maddr, mname, mavatar) => {
                    let id = Member::get_id(&db, &gid, &mid)?;
                    Member::update(&db, &id, &maddr, &mname)?;
                    if mavatar.len() > 0 {
                        write_avatar_sync(&base, &mgid, &mid, mavatar)?;
                    }
                    results.rpcs.push(rpc::member_info(mgid, id, maddr, mname));
                }
                Event::MemberJoin(mid, maddr, mname, mavatar, mtime) => {
                    let mut member = Member::new(gid, mid, maddr, mname, false, mtime);
                    member.insert(&db)?;
                    if mavatar.len() > 0 {
                        write_avatar_sync(&base, &mgid, &mid, mavatar)?;
                    }
                    results.rpcs.push(rpc::member_join(mgid, member));
                }
                Event::MemberLeave(_mid) => {}
                Event::MessageCreate(mid, nmsg, mtime) => {
                    println!("Sync: create message start");
                    let base = layer.read().await.base.clone();
                    let msg = from_network_message(height, gid, mid, &mgid, nmsg, mtime, &base)?;
                    results.rpcs.push(rpc::message_create(mgid, msg));
                    println!("Sync: create message ok");
                }
            }

            // save event.
            GroupChat::add_height(&db, gid, height)?;
        }
        LayerEvent::Packed(gcd, height, from, to, events) => {
            println!("Start handle sync packed... {}, {}, {}", height, from, to);
            let base = layer.read().await.base().clone();
            handle_sync(
                mgid, gid, gcd, addr, height, from, to, events, base, results,
            )?;
        }
        LayerEvent::SyncReq(..) => {} // Never here.
    }

    Ok(())
}

#[inline]
fn load_group(base: &PathBuf, mgid: &GroupId, gcd: &GroupId) -> Result<Option<GroupChat>> {
    let db = group_chat_db(base, mgid)?;
    GroupChat::get(&db, gcd)
}

pub(crate) fn group_chat_conn(proof: Proof, addr: PeerAddr, gid: GroupId) -> SendType {
    let data =
        postcard::to_allocvec(&GroupConnect::Join(gid, JoinProof::Had(proof))).unwrap_or(vec![]);
    SendType::Connect(0, addr, None, None, data)
}

fn ping(gcd: GroupId, addr: PeerAddr) -> SendType {
    let data = postcard::to_allocvec(&LayerEvent::OnlinePing(gcd)).unwrap_or(vec![]);
    SendType::Event(0, addr, data)
}

fn sync(gcd: GroupId, addr: PeerAddr, height: i64) -> SendType {
    println!("Send sync request...");
    let data = postcard::to_allocvec(&LayerEvent::SyncReq(gcd, height + 1)).unwrap_or(vec![]);
    SendType::Event(0, addr, data)
}

fn handle_sync(
    mgid: GroupId,
    fid: i64,
    gcd: GroupId,
    addr: PeerAddr,
    height: i64,
    mut from: i64,
    to: i64,
    events: Vec<PackedEvent>,
    base: PathBuf,
    results: &mut HandleResult,
) -> Result<()> {
    let db = group_chat_db(&base, &mgid)?;

    for event in events {
        let _ = handle_sync_event(&mgid, &fid, from, event, &base, &db, results);
        from += 1;
    }

    if to < height {
        add_layer(results, mgid, sync(gcd, addr, to + 1));
    }

    // update group chat height.
    GroupChat::add_height(&db, fid, to)?;

    Ok(())
}

fn handle_sync_event(
    mgid: &GroupId,
    fid: &i64,
    height: i64,
    event: PackedEvent,
    base: &PathBuf,
    db: &DStorage,
    results: &mut HandleResult,
) -> Result<()> {
    match event {
        PackedEvent::GroupInfo => {
            // TODO
        }
        PackedEvent::GroupTransfer => {
            // TODO
        }
        PackedEvent::GroupManagerAdd => {
            // TODO
        }
        PackedEvent::GroupManagerDel => {
            // TODO
        }
        PackedEvent::GroupClose => {
            // TOOD
        }
        PackedEvent::MemberInfo(mid, maddr, mname, mavatar) => {
            // TODO
        }
        PackedEvent::MemberJoin(mid, maddr, mname, mavatar, mtime) => {
            if mavatar.len() > 0 {
                write_avatar_sync(&base, &mgid, &mid, mavatar)?;
            }
            let mut member = Member::new(*fid, mid, maddr, mname, false, mtime);
            member.insert(&db)?;
            results.rpcs.push(rpc::member_join(*mgid, member));
        }
        PackedEvent::MemberLeave(mid) => {
            // TODO
        }
        PackedEvent::MessageCreate(mid, nmsg, time) => {
            let msg = from_network_message(height, *fid, mid, mgid, nmsg, time, base)?;
            results.rpcs.push(rpc::message_create(*mgid, msg));
        }
        PackedEvent::None => {}
    }

    Ok(())
}
