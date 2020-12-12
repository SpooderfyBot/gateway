pub mod room;

use warp::ws::WebSocket;
use futures::StreamExt;
use tokio::sync::mpsc;

use std::collections::HashMap;
use std::error;
use chrono::Utc;
use std::convert::Infallible;

use crate::Rooms;


const CREATE: &'static str = "create";
const DELETE: &'static str = "delete";
const ADD_SESSION: &'static str = "add_session";
const REMOVE_SESSION: &'static str = "remove_session";


pub async fn create_or_delete_room(
    query: HashMap<String, String>,
    rooms: Rooms
)  -> Result<impl warp::Reply, Infallible>  {

    if query.get("room_id").is_none() {
        return Ok(
            warp::http::Response::builder()
                .status(400)
                .body("[ 400 ] Missing room_id query")
                .unwrap()
        )
    } else if query.get("op").is_none() {
        return Ok(
            warp::http::Response::builder()
                .status(400)
                .body("[ 400 ] Missing op query")
                .unwrap()
        )
    }

    let op = query.get("op").unwrap();
    let room_id = query.get("room_id").unwrap();
    let exists = match op.as_str() {
        CREATE => {
            create(room_id.clone(), rooms).await;

            true  // always will exist
        },
        DELETE => delete(room_id.clone(), rooms).await,
        ADD_SESSION => {
            let exists = if let Some(valid) = query.get("session_id") {
                add_session(
                    room_id.clone(),
                    rooms,
                    valid.clone()
                ).await
            } else {
                return Ok(
                    warp::http::Response::builder()
                        .status(400)
                        .body("[ 400 ] session_id op query")
                        .unwrap()
                )
            };

            exists
        },
        REMOVE_SESSION => {
            let exists = if let Some(valid) = query.get("session_id") {
                remove_session(
                    room_id.clone(),
                    rooms,
                    valid.clone()
                ).await
            } else {
                return Ok(
                    warp::http::Response::builder()
                        .status(400)
                        .body("[ 400 ] session_id op query")
                        .unwrap()
                )
            };

            exists
        },

        _ => {
            return Ok(
                warp::http::Response::builder()
                    .status(404)
                    .body("[ 404 ] Unknown operation")
                    .unwrap()
            )
        }
    };

    let resp = if exists {
       warp::http::Response::builder()
            .status(200)
            .body("[ OK ] Room operation complete")
            .unwrap()
    } else {
        warp::http::Response::builder()
            .status(404)
            .body("[ 404 ] Item does not exist")
            .unwrap()
    } ;

    Ok(resp)
}

async fn create(room_id: String, rooms: Rooms) {
    println!(
        "[ {} ] Creating Room with ID: {}",
        Utc::now().format("%D | %T"),
        &room_id
    );
    let new_room = room::Room::new();
    rooms.write().await.insert(room_id, new_room);
}

async fn delete(room_id: String, rooms: Rooms) -> bool {
    println!(
        "[ {} ] Deleting Room with ID: {}",
        Utc::now().format("%D | %T"),
        &room_id
    );

    rooms.write().await.remove(&room_id).is_none()
}

async fn add_session(room_id: String, rooms: Rooms, valid_id: String) -> bool {
    println!(
        "[ {} ] Adding session to room ID: {}",
        Utc::now().format("%D | %T"),
        &room_id
    );
    if let Some(room) = rooms.read().await.get(&room_id) {
        room.add_session(valid_id).await;
        true
    } else {
        false
    }

}

async fn remove_session(room_id: String, rooms: Rooms, valid_id: String) -> bool {
    println!(
        "[ {} ] Removing session from room ID: {}",
        Utc::now().format("%D | %T"),
        &room_id
    );
    if let Some(room) = rooms.read().await.get(&room_id) {
        room.remove_session(valid_id).await;
        true
    } else {
        false
    }

}

pub async fn on_consumer_connect(
    ws: WebSocket,
    query: HashMap<String, String>,
    rooms: Rooms,
) {
    if let Err(e) = handle_connection(ws, query, rooms).await {
        eprintln!("Connection error: {:?}", e);
    }
}

async fn handle_connection(
    ws: WebSocket,
    query: HashMap<String, String>,
    rooms: Rooms,
) -> Result<(), Box<dyn error::Error>> {
    println!("[ {} ] Client Connected", Utc::now().format("%D | %T"));

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

    if !room.is_valid(session_id).await {
        let _ = ws.close().await;
        return Ok(())
    }

    let (user_ws_tx, _) = ws.split();

    let (tx, rx) = mpsc::unbounded_channel();

    room.add_client(tx).await;
    let _ = rx.forward(user_ws_tx).await;
    println!("closing luil");
    Ok(())
}
