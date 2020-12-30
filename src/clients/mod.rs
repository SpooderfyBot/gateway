use hashbrown::hash_map::HashMap;
use serde::{Serialize, Deserialize};

use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::RwLock;
use warp::ws::Message;

pub mod routes;

pub type ClientMap = RwLock<HashMap<usize, UnboundedSender<Message>>>;
pub type ClientInfo = RwLock<HashMap<usize, User>>;
pub type SessionsMap = RwLock<HashMap<String, usize>>;


#[derive(Serialize)]
pub struct Event<T> where T: Serialize {
    pub op: usize,
    pub payload: T
}

#[derive(Serialize ,Deserialize, Clone)]
pub struct User {
    pub id: usize,
    pub name: String,
    pub avatar_url: String,
}


pub struct Clients {
    clients: ClientMap,
}

impl Clients {
    pub fn new() -> Self {
        Self {
            clients: ClientMap::default(),
        }
    }

    pub async fn emit<T>(&self, event: Event<T>) where T: Serialize {
        let mut remove: Vec<usize> = Vec::new();
        let data = serde_json::to_string(&event).unwrap();
        for (id, tx) in self.clients.read().await.iter() {
            if let Err(_e) = tx.send(Message::text(&data)) {
                // This can be ignored because the error will be handled by
                // the sink itself if its not a disconnect.
                remove.push(*id);
            }
        }

        let mut lock = self.clients.write().await;
        for id in remove {
            &lock.remove(&id);
        }

    }

    pub async fn add_client(&self, id: usize, sender: UnboundedSender<Message>) {
        let mut lock = self.clients.write().await;
        lock.insert(id, sender);
    }

    pub async fn remove_client(&self, id: usize) {
        let mut lock = self.clients.write().await;
        lock.remove(&id);
    }

    pub async fn has_client(&self, id: usize) -> bool {
        let lock = self.clients.read().await;
        lock.contains_key(&id)
    }
}


pub struct Sessions {
    clients: ClientInfo,
    sessions: SessionsMap,
}

impl Sessions {
    pub fn new() -> Self {
        Self {
            clients: ClientInfo::default(),
            sessions: SessionsMap::default(),
        }
    }

    pub async fn add_user(&self, session_id: String, user: User) {
        let uid = user.id;
        {
            let mut lock = self.clients.write().await;
            lock.insert(user.id, user);
        }

        {
            let mut lock = self.sessions.write().await;
            lock.insert(session_id, uid);
        }
    }

    pub async fn remove_user(&self, id: usize) {
        let mut lock = self.clients.write().await;
        lock.remove(&id);
    }

    pub async fn get_user_by_id(&self, id: usize) -> Option<User> {
        let lock = self.clients.read().await;
        let val = lock.get(&id);
        return if val.is_some() {
            Some(val.unwrap().clone())
        } else {
            None
        }
    }

    pub async fn get_user_by_session(&self, sess: &str) -> Option<User> {
        let id = {
            let lock = self.sessions.read().await;
            let id = match lock.get(sess) {
                None => return None,
                Some(id) => id,
            };

            *id
        };


        let lock = self.clients.read().await;
        let val = lock.get(&id);
        return if val.is_some() {
            Some(val.unwrap().clone())
        } else {
            None
        }
    }
}

