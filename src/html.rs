use rocket::{Route, State, Response};
use rocket::http::{Status, CookieJar};

use std::fs;
use serde::Serialize;

use crate::Rooms;
use crate::clients::Sessions;
use crate::utils::responses;

lazy_static! {
    static ref ROOM_HTML: String = {
        fs::read_to_string("./templates/room.html").unwrap()
    };
}

#[get("/<room_id>")]
async fn get_room_html<'a>(
    room_id: String,
    rooms: State<'_, Rooms>,
    sessions: State<'_, Sessions>,
    cookies: &'a CookieJar<'_>
) -> Response<'a> {

    // Cleanup the lock as soon as possible because deadlocks are
    // annoying and this seemed like the best way to avoid them.
    {
        let lock = rooms.read().await;
        let maybe_room = lock.get(&room_id);

        if maybe_room.is_none() {
            return not_found();
        }
    }

    let crumb = match cookies.get("session") {
        Some(c) => c,
        None => return unauthorized()
    };

    let session_id = crumb.value();
    if let Some(_) = sessions.get_user_by_session(session_id).await {
        ok()
    } else {
        unauthorized()
    }
}


#[derive(Serialize)]
struct PlayerResponse {
    message: String
}

fn ok() -> Response<'static> {
    responses::html_response(Status::Ok, &ROOM_HTML).unwrap()
}

fn not_found() -> Response<'static> {
    let resp = PlayerResponse {
        message: "Room not found".to_string(),
    };
    responses::json_response(Status::NotFound, &resp).unwrap()
}

fn unauthorized() -> Response<'static> {
    let resp = PlayerResponse {
        message: "Unauthorized Request".to_string(),
    };
    responses::json_response(Status::Unauthorized, &resp).unwrap()
}

pub fn get_routes() -> Vec<Route> {
    routes![get_room_html]
}
