use warp::ws::{WebSocket, Message};
use tokio::sync::mpsc;
use futures::{FutureExt, StreamExt};
use serde::{Serialize, Deserialize};
use serde_json::Result as ParseResult;

use crate::Rooms;



pub async fn on_emitter_connect(ws: WebSocket, rooms: Rooms) {
    println!("Emitter connected!");

    let (user_ws_tx, mut user_ws_rx) = ws.split();

    let (_, rx) = mpsc::unbounded_channel();
    tokio::task::spawn(rx.forward(user_ws_tx).map(|result| {
        if let Err(e) = result {
            eprintln!("websocket send error: {}", e);
        }
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
}

async fn on_emitter_message(msg: Message, rooms: &Rooms) -> ParseResult<()> {
    let msg = if let Ok(s) = msg.to_str() {
        let msg: EmitterMessage = serde_json::from_str(s)?;
        msg
    } else {
        return Ok(());
    };

    let rooms = rooms.read().await;
    let room = match rooms.get(&msg.room_id) {
        Some(r) => r,
        None => return Ok(())
    };

    room.send_message(msg.message).await;

    Ok(())
}

#[derive(Serialize, Deserialize)]
struct EmitterMessage {
    room_id: String,
    message: String,
}
