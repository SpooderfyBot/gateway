#![allow(unused)]

use crossbeam::channel::{
    Sender as CrossSender,
    Receiver as CrossReceiver,
    TryRecvError,
    TrySendError,
    RecvError,
    SendError,
    bounded as cross_bounded,
};
use tokio::time::{delay_for, Duration};


pub fn bounded<T>(n: usize) -> (Sender<T>, Receiver<T>) {
    let (tx, rx) = cross_bounded(n);
    let tx = Sender{tx};
    let rx = Receiver{rx};

    (tx, rx)
}


/// The sending side of the channel this just mimics what cross beam
/// does but yields while waiting for a result giving it the async vibes.
pub struct Sender<T> {
    tx: CrossSender<T>
}

impl <T> Sender<T> {
    pub async fn send(&self, item: T) -> Result<(), SendError<T>> {
        let mut v = item;
        loop {
            let e = match self.tx.try_send(v) {
                Ok(_) => return Ok(()),
                Err(e) => e,
            };

            match e {
                TrySendError::Disconnected(v) => return Err(SendError(v)),
                TrySendError::Full(val) => {
                    v = val;
                    delay_for(Duration::from_micros(1)).await;
                }
            };
        }
    }
}

/// The receiving side of the channel this just mimics what cross beam
/// does but yields while waiting for a result giving it the async vibes.
pub struct Receiver<T> {
    rx: CrossReceiver<T>
}

impl <T> Receiver<T> {
    pub async fn recv(&self) -> Result<T, RecvError> {
        loop {
            let e = match self.rx.try_recv() {
                Ok(v) => return Ok(v),
                Err(e) => e,
            };

            match e {
                TryRecvError::Disconnected => return Err(RecvError),
                TryRecvError::Empty => {
                    delay_for(Duration::from_micros(1)).await;
                }
            };
        }
    }
}
