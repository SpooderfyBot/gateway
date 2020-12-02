use tokio::sync::{mpsc, RwLock};
use warp::ws::Message;
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};


pub struct Room {
    counter: AtomicUsize,
    clients: RwLock<HashMap<usize, mpsc::UnboundedSender<Result<Message, warp::Error>>>>
}

impl Room {
    pub async fn add_client(&self, client: mpsc::UnboundedSender<Result<Message, warp::Error>>) {
        let id_ = self.counter.fetch_add(1, Ordering::Relaxed);
        self.clients.write().await.insert(id_, client);
    }

    pub async fn send_message(&self, msg: String) {
        for (_, client) in self.clients.read().await.iter() {
            if let Err(_disconnected) = client.send(Ok(Message::text(&msg))) {
                // The tx is disconnected, our `user_disconnected` code
                // should be happening in another task, nothing more to
                // do here.
            }
        }
    }
}