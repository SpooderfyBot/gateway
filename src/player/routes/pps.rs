use rocket::{Route, State, Response};
use rocket::http::{Status, CookieJar};

use serde::Serialize;

use crate::Rooms;
use crate::clients::{Event, Sessions};
use crate::utils::responses;
use crate::opcodes;


#[derive(Serialize)]
struct EventSeek {
    position: usize
}

#[derive(Serialize)]
struct PlayerResponse {
    message: String
}


#[put("/<room_id>/play")]
async fn play<'a>(
    room_id: String,
    rooms: State<'_, Rooms>,
    sessions: State<'_, Sessions>,
    cookies: &'a CookieJar<'_>
) -> Response<'a> {
    let lock = rooms.read().await;
    let maybe_room = lock.get(&room_id);

    let room = match maybe_room {
        None => return not_found(),
        Some(r) => r,
    };

    let crumb = match cookies.get("session") {
        Some(c) => c,
        None => return unauthorized()
    };

    let session_id = crumb.value();
    if let Some(_) = sessions.get_user_by_session(session_id).await {
        let data = Event {
            op: opcodes::OP_PLAY,
            payload: ()
        };

        room.clients.emit(data).await;

        ok()
    } else {
        unauthorized()
    }
}

#[put("/<room_id>/pause")]
async fn pause<'a>(
    room_id: String,
    rooms: State<'_, Rooms>,
    sessions: State<'_, Sessions>,
    cookies: &'a CookieJar<'_>
) -> Response<'a> {
let lock = rooms.read().await;
    let maybe_room = lock.get(&room_id);

    let room = match maybe_room {
        None => return not_found(),
        Some(r) => r,
    };

    let crumb = match cookies.get("session") {
        Some(c) => c,
        None => return unauthorized()
    };

    let session_id = crumb.value();
    if let Some(_) = sessions.get_user_by_session(session_id).await {
        let data = Event {
            op: opcodes::OP_PAUSE,
            payload: ()
        };

        room.clients.emit(data).await;

        ok()
    } else {
        unauthorized()
    }
}

#[put("/<room_id>/seek?<position>")]
async fn seek<'a>(
    room_id: String,
    position: usize,
    rooms: State<'_, Rooms>,
    sessions: State<'_, Sessions>,
    cookies: &'a CookieJar<'_>
) -> Response<'a> {
    let lock = rooms.read().await;
    let maybe_room = lock.get(&room_id);

    let room = match maybe_room {
        None => return not_found(),
        Some(r) => r,
    };

    let crumb = match cookies.get("session") {
        Some(c) => c,
        None => return unauthorized()
    };

    let session_id = crumb.value();
    if let Some(_) = sessions.get_user_by_session(session_id).await {
        let data = Event {
            op: opcodes::OP_SEEK,
            payload: EventSeek { position }
        };

        room.clients.emit(data).await;

        ok()
    } else {
        unauthorized()
    }
}


pub fn get_routes() -> Vec<Route> {
    routes![play, pause, seek]
}

fn not_found() -> Response<'static> {
    let resp = PlayerResponse {
        message: "Room not found".to_string(),
    };
    responses::json_response(Status::NotFound, &resp).unwrap()
}

fn ok() -> Response<'static> {
    let resp = PlayerResponse {
        message: "Operation Complete".to_string(),
    };
    responses::json_response(Status::Ok, &resp).unwrap()
}

fn unauthorized() -> Response<'static> {
    let resp = PlayerResponse {
        message: "Unauthorized Request".to_string(),
    };
    responses::json_response(Status::Unauthorized, &resp).unwrap()
}