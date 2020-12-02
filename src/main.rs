mod emitters;
mod rooms;
mod redis_client;

// #![deny(warnings)]
use std::collections::HashMap;
use std::sync::Arc;
use std::error;
use std::net::SocketAddr;
use std::fs;

use tokio::sync::RwLock;

use warp::Filter;

use redis_client::AioRedisPool;
use serde::{Serialize, Deserialize};

use crate::rooms::room::Room;



/// Our state of currently connected users known as the rooms.
///
/// - Key is their id
/// - Value is a sender of `warp::ws::Message`
type Rooms = Arc<RwLock<HashMap<String, Room>>>;

#[tokio::main]
async fn main() -> Result<(), Box<dyn error::Error>> {
    let config = get_config();

    let pool = AioRedisPool::create_clients(&config.redis_url, 5).await?;

    let consumers_base = Rooms::default();

    // Emitter
    let consumers_lock = consumers_base.clone();
    let consumers = warp::any().map(move || consumers_lock.clone());

    let emitter = warp::path("emitters")
        .and(warp::ws())
        .and(consumers)
        .map(|ws: warp::ws::Ws, consumers: Rooms| {
            // This will call our function if the handshake succeeds.
            ws.on_upgrade(move |socket| emitters::on_emitter_connect(
                socket,
                consumers,
            ))
        });

    // Consumer
    let consumers_lock = consumers_base.clone();
    let consumers = warp::any().map(move || consumers_lock.clone());
    let pool = warp::any().map(move || pool.clone());

    let consumer = warp::path("ws")
        .and(warp::ws())
        .and(warp::query::<HashMap<String, String>>())
        .and(consumers)
        .and(pool)
        .map(|ws: warp::ws::Ws, query, consumers, pool| {
            // This will call our function if the handshake succeeds.
            ws.on_upgrade(move |socket| rooms::on_consumer_connect(
                socket,
                query,
                consumers,
                pool,
            ))
        });


    println!("Running @ ws://{}", &config.server_host);

    let server: SocketAddr = config.server_host
        .parse()
        .expect("Unable to parse socket address");
    warp::serve(consumer.or(emitter)).run(server).await;
    Ok(())
}


fn get_config() -> ServerConfig {
    let config = fs::read_to_string("./config.json")
        .expect("could not load config");
    let config: ServerConfig = serde_json::from_str(&config)
        .expect("could not parse json");


    config
}


#[derive(Serialize, Deserialize)]
struct ServerConfig {
    redis_url: String,
    server_host: String,

}