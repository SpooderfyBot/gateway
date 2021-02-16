use warp::http::StatusCode;
use tokio::sync::broadcast;
use tokio::time::{self, Duration};
use tokio::task::JoinHandle;

use dashmap::DashMap;
use dashmap::mapref::one::Ref;

use serde_json::Value;
use serde::{Serialize, Deserialize};

use std::sync::Arc;
use std::collections::hash_map::RandomState;
use std::sync::atomic::{AtomicUsize, AtomicBool};
use std::sync::atomic::Ordering::Relaxed;
use std::env;

use crate::opcodes;
use crate::utils;

pub type RoomSender = broadcast::Sender<String>;
pub type RoomReceiver = broadcast::Receiver<String>;


lazy_static! {
    static ref API_KEY: String = {
        env::var("API_KEY").unwrap_or_else(|_| "".to_string())
    };
}


/// A controller actor that manages room creation and deletion for
/// clients to communicate with one another.
#[derive(Clone)]
pub struct RoomManager {
    rooms: Arc<DashMap<String, Room>>,
    room_watchers: Arc<DashMap<String, JoinHandle<()>>>
}

impl RoomManager {
    /// Creates and starts the actor returning a handle to
    /// communicate with the actor.
    pub fn new() -> Self {
        Self {
            rooms: Arc::new(DashMap::new()),
            room_watchers: Arc::new(DashMap::new()),
        }
    }

    /// Creates a room with a given ID.
    pub fn create_room(&self, room_id: String, live_server: String) {
        if self.rooms.get(&room_id).is_some() {
            return
        }

        let (tx, _) = broadcast::channel(50);
        let room = Room {
            room_id: Arc::new(room_id.clone()),
            live_server: Arc::new(live_server),
            sender: tx,
            members: Arc::from(AtomicUsize::new(0)),
            multiplier: Arc::from(AtomicUsize::new(0)),
            avg_byte_rate: Arc::from(AtomicUsize::new(0)),
            data_streamed: Arc::from(AtomicUsize::new(0)),
            stream_time: Arc::new(AtomicUsize::new(0)),
            is_live: Arc::new(AtomicBool::new(false))
        };

        // why are you doing this??
        let room2 = room.clone();
        let handle = tokio::spawn(room2.watch_stats());

        self.rooms.insert(room_id.clone(), room);
        self.room_watchers.insert(room_id, handle);
    }

    /// Deletes a room with a given ID.
    pub fn delete_room(&self, room_id: String) {
        if let Some((_, handle)) = self.room_watchers.remove(&room_id) {
            handle.abort();
        };
        self.rooms.remove(&room_id);
        println!("[ ROOM {} ] Room closing and terminating connections", &room_id);
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


/// The room basic set of statistics.
///
/// This is pretty much only for gateway clients when someone joins or
/// disconnects from the room. Most things want FullStats for a more
/// detailed incite into the room.
#[derive(Serialize)]
pub struct BasicStats {
    /// The amount of members in the room.
    members: usize,

    /// The multiplier in n.nx format e.g. '1.1x'
    multiplier: String,
}

#[derive(Serialize)]
pub struct FullStats {
    /// The amount of members in the room.
    members: usize,

    /// The multiplier in it's floating point form.
    multiplier: f32,

    /// The total amount of bytes streamed.
    ///
    /// This includes both Audio and Video in total.
    total_bytes_streamed: usize,

    /// The average rate of transfer in bytes.
    ///
    /// This is only in whole integers so any random bits wont be
    /// taken into account.
    /// This average is also made from the mean of the 50th percentile as to
    /// not be affected by random stops / peaks of data.
    avg_bytes_per_sec: usize,

    /// The average time the stream has been going on for based off the
    /// average bitrate and total amount of bytes streamed, this should
    /// generally not be used to determine the time the stream has existed
    /// for / limit the time of the stream but for calculation that can
    /// be more rough / random.
    avg_stream_time: usize,
}


/// The streaming server stats response
#[allow(unused)]
#[derive(Deserialize)]
struct StreamStatsResponse {
    /// The http status.
    status: u16,

    /// Any relevant info returned by the response.
    data: Value,
}


/// The stream statistics.
///
/// This just wraps the resulting JSON given by the live streaming server
/// and changes constantly.
#[allow(unused)]
#[derive(Deserialize)]
struct StreamStats {
    /// The key/room_name string that the server identifies a session with.
    key: String,

    /// The RTMP input url to publish data to the server.
    url: String,

    /// The uid of a stream publisher.
    stream_id: u32,

    /// The total amount of bytes sent to the server in video form.
    video_total_bytes: usize,

    /// The bitrate of the video in bytes/sec.
    video_speed: usize,  // todo maybe rename this???

    /// The total amount of bytes sent to the data in audio form.
    audio_total_bytes: usize,

    /// The bitrate of the audio in bytes/sec.
    audio_speed: usize,  // todo maybe rename this???
}


/// A room manager tha keeps track of the room's state.
#[derive(Clone)]
pub struct Room {
    /// The room id.
    pub(crate) room_id: Arc<String>,

    /// The live server url
    pub(crate) live_server: Arc<String>,

    /// The message broadcasting channel.
    sender: RoomSender,

    /// The amount of members in the room.
    members: Arc<AtomicUsize>,

    /// The xp multiplier for the room.
    multiplier: Arc<AtomicUsize>,

    /// The average bitrate beings sent to the server.
    avg_byte_rate: Arc<AtomicUsize>,

    /// The total amount of bytes streamed to the server.
    data_streamed: Arc<AtomicUsize>,

    /// Approx length of streaming time in seconds.
    stream_time: Arc<AtomicUsize>,

    /// A bool representing if the stream is live or not.
    pub(crate) is_live: Arc<AtomicBool>,
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

        let stats = self.get_basic_stats();

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

        let stats = self.get_basic_stats();

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

        let stats = self.get_basic_stats();

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
    pub fn get_basic_stats(&self) -> BasicStats {
        let members = self.member_count();
        let multiplier = if members > 0 {
            self.multiplier.load(Relaxed) as f32 / 10f32
        } else {
            0f32
        };
        BasicStats {
            members: self.member_count(),
            multiplier: format!("{:.1}x", multiplier),
        }
    }

    /// Loads and exports the current room stats including the streaming stats.
    pub fn get_full_stats(&self) -> FullStats {
        let members = self.member_count();
        let multiplier = if members > 0 {
            self.multiplier.load(Relaxed) as f32 / 10f32
        } else {
            0f32
        };
        let total_bytes_streamed = self.data_streamed.load(Relaxed);
        let avg_bytes_per_sec = self.avg_byte_rate.load(Relaxed);
        let avg_stream_time = total_bytes_streamed / avg_bytes_per_sec;

        FullStats {
            members,
            multiplier,
            total_bytes_streamed,
            avg_bytes_per_sec,
            avg_stream_time
        }
    }

    /// Watches the streaming server for stats and calculates the XP every
    /// minute, this is also used to work out the avg bitrate of the stream
    /// to apply a soft limit of N bytes per second as to not leave the servers
    /// munching a insane amount of bandwidth from one server.
    async fn watch_stats(self) {
        let client = reqwest::Client::new();

        let mut errors = 0usize;
        let mut rates = Vec::new();

        loop {
            let maybe_resp = client
                .get(&format!(
                    "{}/stats/livestat?room={}&authorization={}",
                    &self.live_server,
                    &self.room_id,
                    API_KEY.as_str(),
                ))
                .send()
                .await;

            let resp = match maybe_resp {
                Ok(resp) => {
                    resp
                },
                Err(e) => {
                    eprintln!(
                        "[ ROOM {} ] Error getting live stats: {:?}",
                        &self.room_id,
                        e
                    );

                    errors += 1;

                    if errors >= 3 {
                        eprintln!(
                            "[ ROOM {} ] Exiting stats watcher due to error overflow.",
                            &self.room_id
                        );
                        return
                    }
                    time::sleep(Duration::from_secs(60)).await;

                    continue
                }
            };


            let status = resp.status();

            let msg = resp
                    .text()
                    .await
                    .unwrap_or_else(|_| "".to_string());
            let maybe_data = serde_json::from_str::<'_, StreamStatsResponse>(&msg);

            if status == StatusCode::NOT_FOUND {
                println!(
                    "[ ROOM {} ] Room is not streaming... Aborting sampling.",
                    &self.room_id,
                );
                time::sleep(Duration::from_secs(10)).await;
                continue
            } else if status != StatusCode::OK {
                let msg = match maybe_data {
                    // Api responded with a custom error detail.
                    Ok(error_resp) => {
                        format!("{:?}", error_resp.data)
                    },

                    // The api crashed un-expectantly and was unable to produce
                    // a usable detail.
                    Err(_) => {
                        format!("Error has no details: {}", &msg)
                    }
                };

                eprintln!(
                    "[ ROOM {} ] Unexpected api response with status: {}, Msg: {}",
                    &self.room_id,
                    status.as_str(),
                    msg,
                );

                time::sleep(Duration::from_secs(60)).await;
                continue
            } else {
                self.is_live.store(true, Relaxed);

                let url = format!("{}/live/{}.flv", &self.live_server, &self.room_id);
                let payload = serde_json::json!({
                    "opcode":  opcodes::OP_LIVE_READY,
                    "payload": {
                        "stream_url": url,
                    }
                });
                self.send(serde_json::to_string(&payload).unwrap())
            }

            let data = maybe_data.unwrap();
            let stats = serde_json::from_value::<StreamStats>(data.data);
            match stats {
                Ok(stats) => {
                    let total_b = stats.video_total_bytes + stats.audio_total_bytes;
                    self.data_streamed.store(total_b, Relaxed);

                    rates.push(((stats.video_speed + stats.audio_speed) * 1000) / 8);
                    rates.sort();

                    let mid = rates.len() / 2;
                    let side_split = mid / 2;
                    let sliced = &rates[side_split..rates.len() - side_split];
                    let avg_rate = sliced.iter().sum::<usize>() / sliced.len();
                    self.avg_byte_rate.store(avg_rate, Relaxed);

                     let avg_time = if (total_b > 0) & (avg_rate > 0) {
                        total_b / avg_rate
                    } else {
                        0usize
                    };

                    self.stream_time.store(avg_time, Relaxed);

                    println!(
                        "[ ROOM {} ] Sampled steam, Total: {:}, Avg Rate: {}/Sec, Avg Time: {}",
                        &self.room_id,
                        utils::format_data(total_b as f64),
                        utils::format_data(avg_rate as f64),
                        utils::humanize(Duration::from_secs(avg_time as u64)),
                    )
                },
                Err(e) => {
                    eprintln!(
                        "[ ROOM {} ] Error de-serializing stream stats: {:?}, Origin: {}",
                        &self.room_id,
                        e,
                        &msg,
                    );

                    errors += 1;

                    if errors >= 3 {
                        eprintln!(
                            "[ ROOM {} ] Exiting stats watcher due to error overflow.",
                            &self.room_id
                        );
                        return
                    }
                }
            };


           time::sleep(Duration::from_secs(60)).await;
        }
    }
}




