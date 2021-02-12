use warp::ws::{WebSocket, Message};

use futures::stream::SplitSink;
use futures::{SinkExt, StreamExt};

use crate::managers::{RoomReceiver, RoomManager};


/// Handles a room client in the form of a websocket connection.
///
/// If a room does not exist the websocket is just immediately closed
/// and ignored.
pub async fn connect_client(
    ws: WebSocket,
    room_id: String,
    rooms: RoomManager,
) {
    let receiver = {
        if let Some(room) = rooms.get(&room_id) {
            room.subscribe()
        } else {
            println!(
               "[ ROOM {} ] Unknown room attempted join, \
                 terminating conn.",
              &room_id
            );
            let _ = ws.close().await;
            return;
        }
    };

    handle_client(
        ws,
        &rooms,
        room_id.clone(),
        receiver
    ).await;

    if let Some(room) = rooms.get(&room_id) {
        room.member_leave();
    };
}


/// Handles a client with a room that exists.
///
/// This spawns a worker task that emits messages from the broadcast
/// receiver to the websocket stream.
///
/// The websocket stays alive until the receiver half of the websocket
/// returns None resulting in a client disconnect.
async fn handle_client(
    ws: WebSocket,
    rooms: &RoomManager,
    room_id: String,
    receiver: RoomReceiver,
) {
    let (ws_tx, mut ws_rx) = ws.split();

    tokio::spawn(watch_messages(ws_tx, receiver));

    {
        if let Some(room) = rooms.get(&room_id) {
            room.member_join();
        };
    }


    while let Some(msg) = ws_rx.next().await {
        let msg = match msg {
            Ok(msg) => msg,
            Err(_) => break,
        };

        if !msg.is_text() {
            break;
        }

        if let Ok(msg) = msg.to_str() {
            if msg != "ping" {
                break;
            }
        } else {
            break;
        }
    };

    println!(
        "[ ROOM {} ] Client disconnected.",
        &room_id
    );



}

/// Watches for messages from the broadcast channel and sends them to the
/// websocket, this will end early if the websocket experiences and error.
async fn watch_messages(mut ws: SplitSink<WebSocket, Message>, mut rx: RoomReceiver) {
    while let Ok(msg) = rx.recv().await {
        if let Err(_) = ws.send(Message::text(msg)).await {
            break;
        }

        if let Err(_) = ws.flush().await {
            break;
        }
    }

    let _ = ws.close();
}

