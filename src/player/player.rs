use serde::{Serialize, Deserialize};
use tokio::sync::RwLock;

use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering::Relaxed;


pub type Tracks = RwLock<AtomicDeQueue<Track>>;

pub struct MediaPlayer {
    current_time: AtomicUsize,
    tracks: Tracks,
}

impl MediaPlayer {
    pub fn new() -> Self {
        Self {
            current_time: AtomicUsize::new(0),
            tracks: Tracks::default(),
        }
    }

    /// Get the player time of all clients (or so it thinks.)
    pub fn time(&self) -> usize {
        self.current_time.load(Relaxed)
    }

    /// Sets the time of the players as a general measure.
    pub fn set_time(&self, time: usize) {
        self.current_time.store(time, Relaxed)
    }

    /// Gets the next track in the queue and rotates the queue by +1, this will
    /// return either None (No tracks left) or a Track struct, this is O(1) and
    /// will never panic because it's just a simple increment.
    pub async fn next_track(&self) -> Option<Track> {
        let lock = self.tracks.read().await;
        let res = match lock.get() {
            None => None,
            Some(v) => Some(v.clone()),
        };

        lock.rotate(1);

        res
    }

    /// Gets the previous track in the queue and rotates the queue by -1, this
    /// will return either None (No tracks left) or a Track struct, this is O(1)
    /// and will never panic because it's just a simple increment.
    pub async fn previous_track(&self) -> Option<Track> {
        let lock = self.tracks.read().await;
        let res = match lock.get() {
            None => None,
            Some(v) => Some(v.clone()),
        };

        lock.rotate(1);

        res
    }

    /// Adds a track to the end of the queue.
    pub async fn add_track(&self, track: Track) {
        let mut lock = self.tracks.write().await;
        lock.append(track);
    }

    /// Removes a track with a given index, this will automatically resize
    /// and re-order the queue to fit in the new bounds.
    pub async fn remove_track(&self, index: usize) -> Option<Track> {
        let mut lock = self.tracks.write().await;
        lock.delete(index)
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Track {
    title: String,
    url: String,
}


/// A semi-atomic dequeue implementation, this lets us immutably have queue
/// and only mutably do something when we add or a remove a Track instead
/// of mutating the whole list like the collection's impl.
///
/// **Example**:
/// ```
/// let mut queue = AtomicDeQueue::new();
/// queue.rotate(1);
/// let val = queue.get();
/// assert_eq!(val, None);
/// ```
pub struct AtomicDeQueue<T> {
    items: Vec<T>,
    index: AtomicUsize,
}

impl<T> AtomicDeQueue<T> {
    fn new() -> Self {

        Self {
            items: Vec::new(),
            index: AtomicUsize::new(0),
        }
    }

    #[inline]
    fn len(&self) -> usize {
        self.items.len()
    }

    fn rotate(&self, n: usize) {
        let current_len = self.len();
        let current_index = self.index.load(Relaxed);
        if (current_index + n) >= current_len {
            self.index.store((current_index + n) - current_len, Relaxed);
        } else {
            self.index.fetch_add(n, Relaxed);
        }
    }

    fn get(&self) -> Option<&T> {
        let index = self.index.load(Relaxed);
        self.items.get(index)
    }

    fn append(&mut self, item: T) {
        self.items.push(item)
    }

    fn delete(&mut self, pos: usize) -> Option<T> {
        if pos < self.len() {
            let result = Some(self.items.remove(pos));

            let index = self.index.load(Relaxed);
            if index >= pos {
                self.index.store(0, Relaxed);
            }

            result

        } else {
            None
        }

    }
}

impl<T> Default for AtomicDeQueue<T> {
    fn default() -> Self {
        Self::new()
    }
}