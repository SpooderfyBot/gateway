pub mod room;

use serde::{Serialize, Deserialize};
use warp::ws::WebSocket;
use futures::{FutureExt, StreamExt};
use tokio::sync::mpsc;

use std::collections::HashMap;
use std::error;

use redis::AsyncCommands;

use crate::Rooms;
use crate::redis_client::RedisPool;



pub async fn on_consumer_connect(
    ws: WebSocket,
    query: HashMap<String, String>,
    rooms: Rooms,
    pool: RedisPool
) {
    if let Err(e) = handle_connection(ws, query, rooms, pool).await {
        eprintln!("Connection error: {:?}", e);
    }
}

async fn handle_connection(
    ws: WebSocket,
    query: HashMap<String, String>,
    rooms: Rooms,
    pool: RedisPool
) -> Result<(), Box<dyn error::Error>> {

    let room_id = match query.get("id") {
        Some(r) => r,
        None => {
            let _ = ws.close().await;
            return Ok(())
        }
    };

    let session_id =  match query.get("session") {
        Some(r) => r,
        None => {
            let _ = ws.close().await;
            return Ok(())
        }
    };

    let rooms = rooms.read().await;
    let room = match rooms.get(room_id) {
        Some(r) => r,
        None => {
            let _ = ws.close().await;
            return Ok(())
        }
    };

    // Get a redis lock
    let mut con = pool.acquire().await;
    let data: String = (*con).get(session_id).await?;

    // Get the session
    let session: RoomData = serde_json::from_str(&data)?;

    if room_id != &session.room_id {
        let _ = ws.close().await;
        return Ok(())
    }


    let (user_ws_tx, _) = ws.split();

    let (tx, rx) = mpsc::unbounded_channel();

    room.add_client(tx).await;

    rx.forward(user_ws_tx).map(|result| {
        if let Err(e) = result {
            eprintln!("websocket send error: {}", e);
        }
    }).await;

    Ok(())
}


#[derive(Serialize, Deserialize)]
struct RoomData {
    room_id: String,
    user_id: usize,
    username: String,
    avatar: String,
}