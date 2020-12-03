use tokio::sync::{mpsc, RwLock};
use warp::ws::Message;
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicUsize, Ordering};


type Clients = RwLock<HashMap<usize, mpsc::UnboundedSender<Result<Message, warp::Error>>>>;
type Sessions = RwLock<HashSet<String>>;

pub struct Room {
    counter: AtomicUsize,
    clients: Clients,
    valid_sessions: Sessions,
}

impl Room {
    pub fn new() -> Self {
        Room {
            counter: AtomicUsize::new(0),
            clients: Clients::default(),
            valid_sessions: Sessions::default(),
        }
    }

    pub async fn is_valid(&self, session_id: &String) -> bool {
        let resp = self.valid_sessions.read().await;
        resp.get(session_id).is_some()
    }

    pub async fn add_session(&self, session_id: String) {
        self.valid_sessions.write().await.insert(session_id);
    }

    pub async fn remove_session(&self, session_id: String) {
        self.valid_sessions.write().await.remove(&session_id);
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