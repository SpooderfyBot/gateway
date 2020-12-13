use warp::ws::{WebSocket, Message};
use tokio::sync::mpsc;
use futures::{FutureExt, StreamExt};
use serde::{Serialize, Deserialize};
use serde_json::Result as ParseResult;
use chrono::Utc;

use crate::Rooms;


pub async fn on_emitter_connect(ws: WebSocket, rooms: Rooms) {
    println!("[ {} ] Emitter Connected", Utc::now().format("%D | %T"));

    let (user_ws_tx, mut user_ws_rx) = ws.split();

    let (_, rx) = mpsc::unbounded_channel();
    tokio::task::spawn(rx.forward(user_ws_tx).map(|result| {
        if let Err(_) = result { }
    }));

    while let Some(result) = user_ws_rx.next().await {
        let msg = match result {
            Ok(msg) => msg,
            Err(e) => {
                eprintln!("receiving websocket error: {}", e);
                break;
            }
        };
        if let Err(e) = on_emitter_message(msg, &rooms).await {
            eprintln!("Failed to parse emitter message: {:?}", e);
        };
    }

    println!("[ {} ] Emitter Disconnected", Utc::now().format("%D | %T"));
}

async fn on_emitter_message(msg: Message, rooms: &Rooms) -> ParseResult<()> {
    let msg: EmitterMessage = serde_json::from_slice(msg.as_bytes())?;
    let rooms = rooms.read().await;
    let room = match rooms.get(&msg.room_id) {
        Some(r) => r,
        None => return Ok(())
    };

    let msg = serde_json::to_string(&msg.message)?;
    room.send_message(msg).await;

    Ok(())
}

#[derive(Serialize, Deserialize, Debug)]
struct EmitterMessage {
    room_id: String,
    message: serde_json::Value,
}
