use tokio::sync::{mpsc, RwLock};
use warp::ws::Message;
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicUsize, Ordering};
use chrono::Utc;
use tokio::time::Duration;


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

    pub async fn add_client(
        &self,
        client: mpsc::UnboundedSender<Result<Message, warp::Error>>
    ) -> usize {
        let id_ = self.counter.fetch_add(1, Ordering::Relaxed);
        self.clients.write().await.insert(id_, client);

        id_
    }

    pub async fn remove_client(&self, id_: &usize) {
        self.clients.write().await.remove(&id_);
        println!("[ {} ] Client Removed", Utc::now().format("%D | %T"));
    }

    pub async fn send_message(&self, msg: String) {
        for (_, client) in self.clients.read().await.iter() {
            let _ = client.send(Ok(Message::text(&msg)));
        }
    }

    pub async fn ping_clients(&self) {
        loop {
            for (id_, client) in self.clients.read().await.iter() {
                if let Err(_) = client.send(Ok(Message::ping(Vec::with_capacity(0)))) {
                    self.remove_client(id_).await;
                }
            }
            tokio::time::delay_for(Duration::from_secs(5)).await;
        }
    }
}