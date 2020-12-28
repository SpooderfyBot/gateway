use std::collections::HashMap;
use std::error;

use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json;

use tokio::sync::mpsc;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio::time::{delay_for, Duration};

use futures::stream::SplitSink;
use futures::{SinkExt, StreamExt};

use warp::filters::ws::{Message, WebSocket};

use crate::opcodes;
use crate::Rooms;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering::Relaxed;

static NEXT_ID: AtomicUsize = AtomicUsize::new(0);

pub async fn handle(ws: WebSocket, query: HashMap<String, String>, rooms: Rooms) {
    let room_id = match query.get("id") {
        Some(r) => r,
        None => {
            println!(
                "[ {} ] No RoomID supplied closing...",
                Utc::now().format("%D | %T")
            );
            let _ = ws.close().await;
            return;
        }
    };

    if let Err(e) = handle_connection(ws, room_id.clone(), rooms).await {
        eprintln!("Connection error: {:?}", e);
    }
}

#[derive(Serialize)]
struct PingPayload {
    op: usize,
}

#[derive(Deserialize)]
struct IncomingPayload {
    op: usize,
}

async fn handle_connection(
    ws: WebSocket,
    room_id: String,
    rooms: Rooms,
) -> Result<(), Box<dyn error::Error>> {
    println!("[ {} ] Client Connected", Utc::now().format("%D | %T"));

    let id = NEXT_ID.fetch_add(1, Relaxed);

    let (tx, rx) = {
        let lock = rooms.read().await;
        let maybe_room = lock.get(&room_id);

        let room = if maybe_room.is_none() {
            println!(
                "[ {} ] Unknown Room closing...",
                Utc::now().format("%D | %T")
            );
            let _ = ws.close().await;
            return Ok(());
        } else {
            maybe_room.unwrap()
        };

        let (tx, rx) = mpsc::unbounded_channel();

        room.clients.add_client(id, tx.clone()).await;

        (tx, rx)
    };

    let (user_ws_tx, mut user_ws_rx) = ws.split();

    tokio::spawn(poll_client(tx));
    tokio::spawn(forward_events(user_ws_tx, rx));

    while let Some(msg) = user_ws_rx.next().await {
        let msg = match msg {
            Ok(msg) => msg,
            Err(_) => break,
        };

        let resp: IncomingPayload = match serde_json::from_slice(msg.as_bytes()) {
            Err(_) => break,
            Ok(r) => r,
        };

        if resp.op != opcodes::OP_PING {
            break;
        }
    }

    println!("[ {} ] Client Disconnected", Utc::now().format("%D | %T"));

    let lock = rooms.read().await;
    let room = lock.get(&room_id).unwrap();
    room.clients.remove_client(id).await;

    Ok(())
}

async fn poll_client(client: UnboundedSender<Message>) {
    let payload = serde_json::to_string(&PingPayload {
        op: opcodes::OP_PING,
    })
    .unwrap();

    loop {
        if let Err(_) = client.send(Message::text(&payload)) {
            return;
        }
        delay_for(Duration::from_secs(5)).await;
    }
}

async fn forward_events(mut ws: SplitSink<WebSocket, Message>, mut rx: UnboundedReceiver<Message>) {
    while let Some(msg) = rx.recv().await {
        if let Err(_) = ws.send(msg).await {
            break;
        }

        if let Err(_) = ws.flush().await {
            break;
        }
    }

    let _ = ws.close();
}
