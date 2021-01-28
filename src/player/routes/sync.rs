use rocket::{Route, State, Response};
use rocket::http::{Status, CookieJar};
use tokio::time::{delay_for, Duration};
use serde::Serialize;

use crate::{Rooms, opcodes};
use crate::clients::{Sessions, Event};
use crate::utils::responses;


#[derive(Serialize)]
struct DefaultResponse {
    message: String,
}

#[derive(Serialize)]
struct TimeResponse {
    message: String,
    time: usize,
}

#[get("/<room_id>/time")]
async fn get_time<'a>(
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
            op: opcodes::OP_TIME_CHECK,
            payload: (),
        };

        room.clients.emit(data).await;
        delay_for(Duration::from_secs(1)).await;
        let time = room.player.time();
        if time != 0 {
            room.player.set_time(time + 1);
        }

        send_time(time + 1)
    } else {
        unauthorized()
    }
}

#[post("/<room_id>/time?<time>")]
async fn set_time<'a>(
    room_id: String,
    time: usize,
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
        room.player.set_time(time);

        ok()
    } else {
        unauthorized()
    }
}

pub fn get_routes() -> Vec<Route> {
    routes![get_time, set_time]
}

fn not_found() -> Response<'static> {
    let resp = DefaultResponse {
        message: "Room not found".to_string(),
    };
    responses::json_response(Status::NotFound, &resp).unwrap()
}

fn ok() -> Response<'static> {
    let resp = DefaultResponse {
        message: "Action success".to_string(),
    };
    responses::json_response(Status::Ok, &resp).unwrap()
}

fn send_time(time: usize) -> Response<'static> {
    let resp = TimeResponse {
        message: "Time found".to_string(),
        time
    };
    responses::json_response(Status::Ok, &resp).unwrap()
}

fn unauthorized() -> Response<'static> {
    let resp = DefaultResponse {
        message: "Unauthorized Request".to_string(),
    };
    responses::json_response(Status::Unauthorized, &resp).unwrap()
}