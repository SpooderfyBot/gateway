use tokio::sync::broadcast;

use dashmap::DashMap;
use dashmap::mapref::one::Ref;

use serde_json::Value;
use serde::Serialize;

use std::sync::Arc;
use std::collections::hash_map::RandomState;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering::Relaxed;

use crate::opcodes;

pub type RoomSender = broadcast::Sender<String>;
pub type RoomReceiver = broadcast::Receiver<String>;



/// A controller actor that manages room creation and deletion for
/// clients to communicate with one another.
#[derive(Clone)]
pub struct RoomManager {
    rooms: Arc<DashMap<String, Room>>,
}

impl RoomManager {
    /// Creates and starts the actor returning a handle to
    /// communicate with the actor.
    pub fn new() -> Self {
        Self {
            rooms: Arc::new(DashMap::new()),
        }
    }

    /// Creates a room with a given ID.
    pub fn create_room(&self, room_id: String) {
        let (tx, _) = broadcast::channel(50);
        let room = Room {
            sender: tx,
            members: AtomicUsize::new(0),
            multiplier: AtomicUsize::new(10),
        };

        self.rooms.insert(room_id, room);
    }

    /// Deletes a room with a given ID.
    pub fn delete_room(&self, room_id: String) {
        self.rooms.remove(&room_id);
    }

    pub fn get(&self, room_id: &String) -> Option<Ref<String, Room, RandomState>> {
        self.rooms.get(room_id)
    }
}



#[derive(Serialize)]
pub struct WsMessage {
    opcode: usize,
    payload: Option<Value>,
}


#[derive(Serialize)]
pub struct Stats {
    members: usize,
    multiplier: String,
}

pub struct Room {
    sender: RoomSender,
    members: AtomicUsize,
    multiplier: AtomicUsize,
}

impl Room {
    pub fn send(&self, msg: String) {
        let _ = self.sender.send(msg);
    }

    pub fn subscribe(&self) -> RoomReceiver {
        self.sender.subscribe()
    }

    pub fn member_count(&self) -> usize {
        self.members.load(Relaxed)
    }

    pub fn member_join(&self) {
        let old = self.members.fetch_add(1, Relaxed);

        let multiplier_maybe = ((old + 1) as f32).log10() * 4f32;
        self.adjust_modifier(multiplier_maybe.round() as usize);

        let stats = self.get_stats();

        // This will never error, i think.
        let val = serde_json::to_value(stats).unwrap();
        let wrapped = WsMessage {
            opcode: opcodes::OP_STATS_UPDATE,
            payload: Some(val),
        };

        let msg = serde_json::to_string(&wrapped).unwrap();

        self.send(msg);
    }

    pub fn member_leave(&self) {
        let old = self.members.fetch_sub(1, Relaxed);

        let multiplier_maybe = ((old - 1) as f32).log10() * 4f32;
        self.adjust_modifier(multiplier_maybe.round() as usize);


        let stats = self.get_stats();

        // This will never error, i think.
        let val = serde_json::to_value(stats).unwrap();
        let wrapped = WsMessage {
            opcode: opcodes::OP_STATS_UPDATE,
            payload: Some(val),
        };

        let msg = serde_json::to_string(&wrapped).unwrap();

        self.send(msg);
    }

    /// Changes the multiplier, uses a percentile to represent the floating
    /// value.
    ///
    /// 10 -> 1x
    /// 15 -> 1.5x
    pub fn adjust_modifier(&self, modifier: usize) {
        self.multiplier.store(10 + modifier, Relaxed);

        let stats = self.get_stats();

        // This will never error, i think.
        let val = serde_json::to_value(stats).unwrap();
        let wrapped = WsMessage {
            opcode: opcodes::OP_STATS_UPDATE,
            payload: Some(val),
        };

        let msg = serde_json::to_string(&wrapped).unwrap();

        self.send(msg);
    }

    pub fn get_stats(&self) -> Stats {
        let multiplier = self.multiplier.load(Relaxed) as f32 / 10f32;
        Stats {
            members: self.member_count(),
            multiplier: format!("{:.1}x", multiplier),
        }
    }
}

