use crate::clients::Clients;
use crate::player::player::MediaPlayer;

use crate::webhook;

pub struct Room {
    pub clients: Clients,
    pub player: MediaPlayer,
    pub webhook: webhook::Webhook,
}

impl Room {
    pub fn new(url: String) -> Self {
        Self {
            clients: Clients::new(),
            player: MediaPlayer::new(),
            webhook: webhook::Webhook::new(url)
        }
    }
}