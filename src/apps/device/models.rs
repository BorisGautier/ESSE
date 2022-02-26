use std::time::{SystemTime, UNIX_EPOCH};
use tdn::types::primitives::{Peer, Result};
use tdn::types::rpc::{json, RpcParam};
use tdn_storage::local::{DStorage, DsValue};

pub(crate) struct Device {
    pub id: i64,
    pub name: String,
    pub info: String,
    pub peer: Peer,
    pub lasttime: i64,
    pub online: bool,
}

impl Device {
    pub fn new(name: String, info: String, peer: Peer) -> Self {
        let start = SystemTime::now();
        let lasttime = start
            .duration_since(UNIX_EPOCH)
            .map(|s| s.as_secs())
            .unwrap_or(0) as i64; // safe for all life.

        Self {
            lasttime,
            info,
            name,
            peer,
            id: 0,
            online: true,
        }
    }

    /// here is zero-copy and unwrap is safe. checked.
    fn from_values(mut v: Vec<DsValue>) -> Device {
        Device {
            lasttime: v.pop().unwrap().as_i64(),
            peer: Peer::from_string(v.pop().unwrap().as_str()).unwrap_or(Peer::default()),
            info: v.pop().unwrap().as_string(),
            name: v.pop().unwrap().as_string(),
            id: v.pop().unwrap().as_i64(),
            online: false,
        }
    }

    pub fn to_rpc(&self) -> RpcParam {
        json!([
            self.id,
            self.name,
            self.info,
            self.peer.to_string(),
            self.lasttime,
            if self.online { "1" } else { "0" },
        ])
    }

    /// load account devices.
    pub fn list(db: &DStorage) -> Result<Vec<Device>> {
        let matrix = db.query("SELECT id, name, info, peer, lasttime FROM devices")?;
        let mut devices = vec![];
        for values in matrix {
            devices.push(Device::from_values(values));
        }
        Ok(devices)
    }

    pub fn distributes(db: &DStorage) -> Result<Vec<(Peer, i64, bool)>> {
        let matrix = db.query("SELECT id, peer FROM devices")?;
        let mut devices = vec![];
        for mut v in matrix {
            if v.len() == 3 {
                let peer = Peer::from_string(v.pop().unwrap().as_str()).unwrap_or(Peer::default());
                let id = v.pop().unwrap().as_i64();
                devices.push((peer, id, false));
            }
        }
        Ok(devices)
    }

    pub fn device_info(db: &DStorage) -> Result<(String, String)> {
        let mut matrix = db.query("SELECT name, info FROM devices ORDER BY id LIMIT 1")?;
        if matrix.len() > 0 {
            let mut values = matrix.pop().unwrap(); // safe unwrap()
            if values.len() == 2 {
                let info = values.pop().unwrap().as_string();
                let name = values.pop().unwrap().as_string();
                return Ok((name, info));
            }
        }
        Ok((String::new(), String::new()))
    }

    pub fn insert(&mut self, db: &DStorage) -> Result<()> {
        let sql = format!(
            "INSERT INTO devices (name, info, peer, lasttime) VALUES ('{}', '{}', '{}', {})",
            self.name,
            self.info,
            self.peer.to_string(),
            self.lasttime,
        );
        let id = db.insert(&sql)?;
        self.id = id;
        Ok(())
    }

    pub fn _update(db: &DStorage, id: i64, name: &str) -> Result<usize> {
        let sql = format!("UPDATE devices SET name='{}' WHERE id = {}", name, id);
        db.update(&sql)
    }

    /// used in rpc, when what to delete a friend.
    pub fn _delete(&self, db: &DStorage) -> Result<usize> {
        let sql = format!("DELETE FROM devices WHERE id = {}", self.id);
        db.update(&sql)
    }
}
