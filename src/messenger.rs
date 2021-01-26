use rocket::{Route, State, Response};
use rocket::http::{Status, CookieJar};

use serde::{Deserialize, Serialize};

use crate::{Rooms, opcodes};
use crate::clients::{Sessions, Event};
use crate::utils::responses;
use crate::json::Json;


#[derive(Deserialize, Serialize)]
struct MessageResp {
    content: String,
    user_id: usize,
    username: String,
    avatar: String,
}

#[derive(Deserialize)]
struct Message {
    content: String,
}

#[derive(Serialize)]
struct MessengerResponse {
    message: String
}


#[put("/<room_id>/message", data="<message>")]
async fn send_message<'a>(
    room_id: String,
    message: Json<MessageResp>,
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
        let message = message.into_inner();
        if let Err(e) = room.webhook.send_as_user(
          message.username.clone(),
          message.avatar.clone(),
          message.content.clone(),
        ).await {
            eprintln!("Error: {:?}", e);
        };

        let data = Event {
            op: opcodes::OP_MESSAGE,
            payload: message,
        };

        room.clients.emit(data).await;

        ok()
    } else {
        unauthorized()
    }
}


#[put("/<room_id>/botmsg", data="<message>")]
async fn bot_message<'a>(
    room_id: String,
    message: Json<Message>,
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
    if let Some(user) = sessions.get_user_by_session(session_id).await {
        let message = message.into_inner();
        if let Err(e) = room.webhook.send_as_user(
          user.name.clone(),
          user.avatar_url.clone(),
          message.content.clone(),
        ).await {
            eprintln!("Error: {:?}", e);
        };

        let data = Event {
            op: opcodes::OP_MESSAGE,
            payload: MessageResp {
                content: message.content,
                user_id: user.id,
                username: user.name,
                avatar: user.avatar_url
            }
        };

        room.clients.emit(data).await;

        ok()
    } else {
        unauthorized()
    }
}


pub fn get_routes() -> Vec<Route> {
    routes![send_message, bot_message]
}


fn not_found() -> Response<'static> {
    let resp = MessengerResponse {
        message: "Room not found".to_string(),
    };
    responses::json_response(Status::NotFound, &resp).unwrap()
}

fn ok() -> Response<'static> {
    let resp = MessengerResponse {
        message: "Operation Complete".to_string(),
    };
    responses::json_response(Status::Ok, &resp).unwrap()
}

fn unauthorized() -> Response<'static> {
    let resp = MessengerResponse {
        message: "Unauthorized Request".to_string(),
    };
    responses::json_response(Status::Unauthorized, &resp).unwrap()
}