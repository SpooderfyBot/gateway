use crate::clients::Clients;
use crate::player::player::MediaPlayer;


pub struct Room {
    pub clients: Clients,
    pub player: MediaPlayer,
}

impl Room {
    pub fn new() -> Self {
        Self {
            clients: Clients::new(),
            player: MediaPlayer::new(),
        }
    }
}