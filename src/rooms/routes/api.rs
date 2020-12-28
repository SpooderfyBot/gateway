use rocket::response::Debug;
use rocket::tokio::io;
use rocket::{Route, State};
use rocket::http::Status;

use serde::Serialize;

use crate::utils::responses;
use crate::rooms::room;
use crate::Rooms;


#[derive(Serialize)]
struct RoomResponse {
    message: String,
    data: Option<RoomData>,
}

#[derive(Serialize)]
struct RoomData {
    id: String,
}

#[post("/<room>/create")]
async fn create_room(
    room: String,
    rooms: State<'_, Rooms>
) -> Result<rocket::Response<'_>, Debug<io::Error>> {

    // No deadlock ty
    let exist = {
        let lock = rooms.read().await;
        lock.contains_key(&room)
    };

    let resp = if exist {
        let resp = RoomResponse {
            message: format!("Room already exists with id: {}", &room),
            data: None,
        };
        responses::json_response(Status::NotAcceptable, &resp)
    } else {
        _create_room(room.clone(), rooms).await;
        let resp = RoomResponse {
            message: format!("Room made with id: {}", &room),
            data: Some(RoomData { id: room }),
        };
        responses::json_response(Status::Ok, &resp)
    };

    Ok(resp.unwrap())
}

#[delete("/<room>/delete")]
async fn delete_room(
    room: String,
    rooms: State<'_, Rooms>
) -> Result<rocket::Response<'_>, Debug<io::Error>> {

    let exist = {
        let lock = rooms.read().await;
        lock.contains_key(&room)
    };

    let resp = if exist {
        _delete_room(&room, rooms).await;

        let resp = RoomResponse {
            message: format!("Deleted room with id: {}", &room),
            data: Some(RoomData { id: room }),
        };
        responses::json_response(Status::Ok, &resp)
    } else {
        let resp = RoomResponse {
            message: format!("Room does not exist with id: {}", &room),
            data: None,
        };
        responses::json_response(Status::NotFound, &resp)
    };

    Ok(resp.unwrap())
}

pub fn get_routes() -> Vec<Route> {
    routes![create_room, delete_room]
}

async fn _create_room(id: String, rooms: State<'_, Rooms>) {
    let new_room = room::Room::new();
    let mut lock = rooms.write().await;
    lock.insert(id, new_room);
}

async fn _delete_room(id: &String, rooms: State<'_, Rooms>) {
    let mut lock = rooms.write().await;
    lock.remove(id);
}