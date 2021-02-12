mod ws;
mod managers;
mod opcodes;

use managers::RoomManager;
use ws::connect_client;

use warp::Filter;
use warp::ws::Ws;
use warp::reply::Response;
use warp::http::header::ACCESS_CONTROL_ALLOW_ORIGIN;
use warp::http::header::ACCESS_CONTROL_ALLOW_METHODS;
use warp::http::header::ACCESS_CONTROL_ALLOW_HEADERS;
use warp::hyper::header::HeaderValue;

use bytes::Bytes;
use warp::http::StatusCode;


#[tokio::main]
async fn main() {
    let room_manager1 = RoomManager::new();
    let room_manager = move || {
        let inst = room_manager1.clone();
        warp::any().map(move || inst.clone())
    };

    // GET /ws/<room_id> -> websocket upgrade
    let gateway = warp::path!("ws" / String)
        // The `ws()` filter will prepare Websocket handshake...
        .and(warp::ws())
        .and(room_manager())
        .map(|room_id: String, ws: Ws, rooms: RoomManager| {
            ws.on_upgrade(move |socket| {
                connect_client(socket, room_id, rooms)
            })
        });

    // GET /add/<room_id> -> Makes a room
    let add_room = warp::path!("add" / String)
        // The `ws()` filter will prepare Websocket handshake...
        .and(room_manager())
        .map(|room_id: String, rooms: RoomManager| {
            rooms.create_room(room_id);

            "Made room!"
        });

    // GET /remove/<room_id> -> Removes a room
    let remove_room = warp::path!("remove" / String)
        // The `ws()` filter will prepare Websocket handshake...
        .and(room_manager())
        .map(|room_id: String, rooms: RoomManager| {
            rooms.delete_room(room_id);

            "Removed room!"
        });

    // GET emit/<room_id>/ -> emits a message to a room
    let emit = warp::path!("emit"/ String)
        // The `ws()` filter will prepare Websocket handshake...
        .and(room_manager())
        .and(warp::body::bytes())
        .map(|room_id: String, rooms: RoomManager, body: Bytes| {
            let msg = if let Some(room) = rooms.get(&room_id) {
                let msg = String::from_utf8_lossy(body.as_ref());
                room.send(msg.to_string());

                "Operation complete!"
            } else {
                "Unknown room"
            };

            let mut resp = Response::new(msg.into());
            let inst = resp.headers_mut();
            inst.insert(ACCESS_CONTROL_ALLOW_HEADERS,HeaderValue::from_static("*"));
            inst.insert(ACCESS_CONTROL_ALLOW_ORIGIN,HeaderValue::from_static("*"));
            inst.insert(ACCESS_CONTROL_ALLOW_METHODS,HeaderValue::from_static("PUT"));

            resp
        });

    // GET emit/<room_id>/ -> emits a message to a room
    let exists = warp::path!("exists"/ String)
        // The `ws()` filter will prepare Websocket handshake...
        .and(room_manager())
        .map(|room_id: String, rooms: RoomManager| {
            if let Some(_) = rooms.get(&room_id) {
                StatusCode::OK
            } else {
                StatusCode::NOT_FOUND
            }
        });

    let routes = gateway
        .or(remove_room)
        .or(add_room)
        .or(emit)
        .or(exists);


    warp::serve(routes).run(([0, 0, 0, 0], 3030)).await;
}


