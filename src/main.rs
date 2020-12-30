#[macro_use]
extern crate rocket;

#[macro_use]
extern crate lazy_static;


use hashbrown::HashMap;
use tokio::sync::RwLock;
use std::sync::Arc;
use warp::Filter;
use std::net::SocketAddr;

mod player;
mod clients;
mod rooms;
mod utils;
mod json;
mod opcodes;
mod security;
mod html;
mod messenger;
mod webhook;

pub static SPOODERFY_LOGO: &str = "https://cdn.discordapp.com/avatars/585225058683977750/73628acbb1304b05c718f22a380767bd.png?size=128";

pub type Rooms = Arc<RwLock<HashMap<String, rooms::room::Room>>>;


#[rocket::main]
async fn main() {
    // state
    let rooms = Rooms::default();
    let sessions = clients::Sessions::new();

    // routes
    let player_routes = player::routes::get_routes();
    let room_routes = rooms::routes::get_routes();
    let html_routes = html::get_routes();
    let session_routes = security::get_routes();
    let message_routes = messenger::get_routes();


    tokio::spawn(run_warp(rooms.clone()));

    let _res = rocket::ignite()
        .manage(rooms.clone())
        .manage(sessions)
        .mount("/api/player", player_routes)
        .mount("/api/room", room_routes)
        .mount("/api/room", message_routes)
        .mount("/api", session_routes)
        .mount("/room", html_routes)
        .launch()
        .await;
}


async fn run_warp(rooms: Rooms) {
    let rooms = warp::any().map(move || rooms.clone());

    let gateway = warp::path("ws")
        .and(warp::ws())
        .and(warp::query::<std::collections::HashMap<String, String>>())
        .and(rooms)
        .map(|ws: warp::ws::Ws, query, rooms| {
            // This will call our function if the handshake succeeds.
            ws.on_upgrade(move |socket| clients::routes::ws::handle(
                socket,
                query,
                rooms,
            ))
        });

    let server: SocketAddr = "0.0.0.0:8080"
        .parse()
        .expect("Unable to parse socket address");

    warp::serve(gateway).run(server).await;
}
