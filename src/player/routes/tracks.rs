use rocket::{Route, State, Response};
use rocket::http::{Status, CookieJar};

use serde::Serialize;

use crate::{Rooms, opcodes};
use crate::utils::responses;
use crate::json::Json;
use crate::player::player::Track;
use crate::clients::{Sessions, Event};
use crate::webhook::UserMessage;
use crate::SPOODERFY_LOGO;
use crate::rooms::room::Room;


#[derive(Serialize)]
struct PlayerResponse {
    message: String,
}

#[derive(Serialize)]
struct PlayerCurrentTrack {
    status: usize,
    message: String,
    track: Option<Track>,
}



#[get("/<room_id>/track/current")]
async fn get_current_track<'a>(
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
        let maybe_track = room.player.current_track().await;

        let msg = PlayerCurrentTrack {
            status: if maybe_track.is_some() { 1 } else { 0 },
            message: "Track loaded".to_string(),
            track: maybe_track
        };

        responses::json_response(Status::Ok, &msg).unwrap()
    } else {
        unauthorized()
    }
}


#[put("/<room_id>/track/next")]
async fn next_track<'a>(
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
        send_system_webhook(&room,format!(
            "\\🚀 [**Moving to next video**](https://spooderfy.com/room/{})",
            room_id
        )).await;

        let maybe_track = room.player.next_track().await;

        let track = match maybe_track {
            Some(t) => t,
            None => return queue_empty(),
        };

        let data = Event {
            op: opcodes::OP_NEXT,
            payload: track
        };

        room.clients.emit(data).await;

        ok()
    } else {
        unauthorized()
    }
}

#[put("/<room_id>/track/previous")]
async fn previous_track<'a>(
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
        send_system_webhook(&room, format!(
            "\\🚀 [**Moving to previous video**](https://spooderfy.com/room/{})",
            room_id
        )).await;

        let maybe_track = room.player.previous_track().await;

        let track = match maybe_track {
            Some(t) => t,
            None => return queue_empty(),
        };

        let data = Event {
            op: opcodes::OP_NEXT,
            payload: track
        };

        room.clients.emit(data).await;

        ok()
    } else {
        unauthorized()
    }
}

#[post("/<room_id>/track/add", data="<track>")]
async fn add_track<'a>(
    room_id: String,
    track: Json<Track>,
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
        let track = track.into_inner();
        send_system_webhook(&room,format!(
            "\\🚀 [**Added video `{}`**](https://spooderfy.com/room/{})",
            &track.title,
            room_id
        )).await;

        room.player.add_track(track).await;
        ok()
    } else {
        unauthorized()
    }
}

#[delete("/<room_id>/track/remove?<index>")]
async fn remove_track<'a>(
    room_id: String,
    index: usize,
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
        send_system_webhook(&room, format!(
            "\\🚀 [**Removed track at index `{}`**](https://spooderfy.com/room/{})",
            index,
            room_id
        )).await;
        let _ = room.player.remove_track(index).await;

        ok()
    } else {
        unauthorized()
    }
}

pub fn get_routes() -> Vec<Route> {
    routes![
        next_track,
        previous_track,
        add_track,
        remove_track,
        get_current_track
    ]
}

async fn send_system_webhook(room: &Room, msg: String) {
    let msg = UserMessage {
        content: msg,
        embeds: (),
        username: "Spooderfy".to_string(),
        avatar_url: SPOODERFY_LOGO.to_string()
    };
    match room.webhook.send(msg).await {
        Ok(is_ok) => {
            if !is_ok {
                eprintln!("Webhook responded with non 2xx or 3xx code");
            }
        },
        Err(e) => eprintln!("{:?}", e)
    };
}

fn not_found() -> Response<'static> {
    let resp = PlayerResponse {
        message: format!("Room not found"),
    };
    responses::json_response(Status::NotFound, &resp).unwrap()
}

fn queue_empty() -> Response<'static> {
    let resp = PlayerResponse {
        message: "EMPTY".to_string(),
    };
    responses::json_response(Status::Ok, &resp).unwrap()
}

fn ok() -> Response<'static> {
    let resp = PlayerResponse {
        message: "Action success".to_string(),
    };
    responses::json_response(Status::Ok, &resp).unwrap()
}

fn unauthorized() -> Response<'static> {
    let resp = PlayerResponse {
        message: "Unauthorized Request".to_string(),
    };
    responses::json_response(Status::Unauthorized, &resp).unwrap()
}