use rocket::{Route, State, Response};
use rocket::http::Status;

use serde::Serialize;

use crate::Rooms;
use crate::utils::responses;
use crate::json::Json;
use crate::player::player::Track;


#[derive(Serialize)]
struct PlayerResponse {
    message: String,
}


#[put("/<room_id>/track/next")]
async fn next_track(room_id: String, rooms: State<'_, Rooms>) -> Response<'_> {
    let lock = rooms.read().await;
    let maybe_room = lock.get(&room_id);

    let room = match maybe_room {
        None => return not_found(),
        Some(r) => r,
    };

    let maybe_track = room.player.next_track().await;

    let track = match maybe_track {
        Some(t) => t,
        None => return queue_empty(),
    };

    responses::json_response(Status::Ok, &track).unwrap()
}

#[put("/<room_id>/track/previous")]
async fn previous_track(room_id: String, rooms: State<'_, Rooms>) -> Response<'_> {
    let lock = rooms.read().await;
    let maybe_room = lock.get(&room_id);

    let room = match maybe_room {
        None => return not_found(),
        Some(r) => r,
    };

    let maybe_track = room.player.previous_track().await;

    let track = match maybe_track {
        Some(t) => t,
        None => return queue_empty(),
    };

    responses::json_response(Status::Ok, &track).unwrap()
}

#[post("/<room_id>/track/add", data="<track>")]
async fn add_track(room_id: String, track: Json<Track>, rooms: State<'_, Rooms>) -> Response<'_> {
    let lock = rooms.read().await;
    let maybe_room = lock.get(&room_id);

    let room = match maybe_room {
        None => return not_found(),
        Some(r) => r,
    };

    room.player.add_track(track.into_inner()).await;

    ok()
}

#[delete("/<room_id>/track/remove?<index>")]
async fn remove_track(room_id: String, index: usize, rooms: State<'_, Rooms>) -> Response<'_> {
    let lock = rooms.read().await;
    let maybe_room = lock.get(&room_id);

    let room = match maybe_room {
        None => return not_found(),
        Some(r) => r,
    };

    let _ = room.player.remove_track(index).await;

    ok()
}

pub fn get_routes() -> Vec<Route> {
    routes![next_track, previous_track, add_track, remove_track]
}

fn not_found() -> Response<'static> {
    let resp = PlayerResponse {
        message: format!("Room not found"),
    };
    responses::json_response(Status::NotFound, &resp).unwrap()
}

fn queue_empty() -> Response<'static> {
    let resp = PlayerResponse {
        message: "Queue empty".to_string(),
    };
    responses::json_response(Status::NoContent, &resp).unwrap()
}

fn ok() -> Response<'static> {
    let resp = PlayerResponse {
        message: "Action success".to_string(),
    };
    responses::json_response(Status::Ok, &resp).unwrap()
}