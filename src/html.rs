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
        None => return unauthorized(&room_id)
    };

    let session_id = crumb.value();
    if let Some(_) = sessions.get_user_by_session(session_id).await {
        ok()
    } else {
        unauthorized(&room_id)
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

fn unauthorized(room_id: &str) -> Response<'static> {
    let login = format!(
        "https://spooderfy.com/login?redirect_to=/room/{}",
        room_id,
    );

    responses::redirect_to(login).unwrap()
}

pub fn get_routes() -> Vec<Route> {
    routes![get_room_html]
}
