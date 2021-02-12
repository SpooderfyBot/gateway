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
use tokio::time::Instant;

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

    /// Gets a room with a given id as a immutable referance.
    pub fn get(&self, room_id: &String) -> Option<Ref<String, Room, RandomState>> {
        self.rooms.get(room_id)
    }
}


/// A message to the websocket
#[derive(Serialize)]
pub struct WsMessage {
    /// The websocket opcode
    opcode: usize,

    /// The payload of the message, if the opcode does not require
    /// a body this can be left blank.
    payload: Option<Value>,
}


/// The room statistics.
#[derive(Serialize)]
pub struct Stats {
    /// The amount of members in the room.
    members: usize,

    /// The multiplier in n.nx format e.g. '1.1x'
    multiplier: String,
}


/// A room manager tha keeps track of the room's state.
pub struct Room {
    /// The message broadcasting channel.
    sender: RoomSender,

    /// The amount of members in the room.
    members: AtomicUsize,

    /// The xp multiplier for the room.
    multiplier: AtomicUsize,
}

impl Room {
    /// Sends a message to the broadcast channel.
    pub fn send(&self, msg: String) {
        let _ = self.sender.send(msg);
    }

    /// Subscribes to the broadcasting channel/
    pub fn subscribe(&self) -> RoomReceiver {
        self.sender.subscribe()
    }

    /// The amount of members in the room
    pub fn member_count(&self) -> usize {
        self.members.load(Relaxed)
    }

    /// Increments the counter on members atomically by 1 and then sends
    /// the stats to all members in the room.
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


    /// Lowers the counter on members atomically and then sends the updated
    /// stats to all other members in the room.
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

    /// Get the room statistics.
    ///
    /// Loads the multiplier and calculated the floating point version of the
    /// modifier, this is then wrapped with member count and constructed into
    /// a Stats struct.
    pub fn get_stats(&self) -> Stats {
        let multiplier = self.multiplier.load(Relaxed) as f32 / 10f32;
        Stats {
            members: self.member_count(),
            multiplier: format!("{:.1}x", multiplier),
        }
    }
}

