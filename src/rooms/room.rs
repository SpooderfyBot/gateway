use tokio::sync::{mpsc, RwLock};
use warp::ws::Message;
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};


type Clients = RwLock<HashMap<usize, mpsc::UnboundedSender<Result<Message, warp::Error>>>>;


pub struct Room {
    counter: AtomicUsize,
    clients: Clients
}

impl Room {
    pub fn new() -> Self {
        Room {
            counter: AtomicUsize::new(0),
            clients: Clients::default(),
        }
    }

    pub async fn add_client(&self, client: mpsc::UnboundedSender<Result<Message, warp::Error>>) {
        let id_ = self.counter.fetch_add(1, Ordering::Relaxed);
        self.clients.write().await.insert(id_, client);
    }

    pub async fn send_message(&self, msg: String) {
        for (_, client) in self.clients.read().await.iter() {
            let _ = client.send(Ok(Message::text(&msg)));
        }
    }
}