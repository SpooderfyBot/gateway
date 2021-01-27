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

        lock.lrotate(1);

        res
    }

    /// Gets the current track on the player.
    pub async fn current_track(&self) -> Option<Track> {
        let lock = self.tracks.read().await;
        let res = match lock.get() {
            None => None,
            Some(v) => Some(v.clone()),
        };

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
    pub title: String,
    pub url: String,
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
        let len = self.len();
        let n = n % len;
        let maybe_index = (self.index.load(Relaxed) as isize) - (n as isize);
        if maybe_index < 0 {
            self.index.store((len as isize + maybe_index) as usize, Relaxed)
        } else {
            self.index.store(maybe_index as usize, Relaxed)
        }
    }

    fn lrotate(&self, n: usize) {
        let len = self.len();
        let n = n % len;
        let maybe_index = self.index.load(Relaxed) + n;
        if maybe_index >= len {
            self.index.store(len - maybe_index, Relaxed)
        } else {
            self.index.store(maybe_index, Relaxed)
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


#[cfg(test)]
mod tests {
    use super::*;

    fn get_queue() -> AtomicDeQueue<Track> {
        let mut queue: AtomicDeQueue<Track> = AtomicDeQueue::new();

        let t1 = Track { title: "Track 1".to_string(), url: "".to_string() };
        let t2 = Track { title: "Track 2".to_string(), url: "".to_string() };
        let t3 = Track { title: "Track 3".to_string(), url: "".to_string() };

        queue.append(t1);
        queue.append(t2);
        queue.append(t3);

        queue
    }

    #[test]
    fn test_queue_len() {
        let queue = get_queue();

        assert_eq!(queue.len(), 3);
    }

    #[test]
    fn test_queue_get() {
        let queue = get_queue();

        let t = queue.get();
        assert!(t.is_some());
    }

    #[test]
    fn test_queue_rotate() {
        let queue = get_queue();

        queue.rotate(1);

        if let Some(t) = queue.get() {
            assert_eq!(t.title, "Track 3");
        }
    }

    #[test]
    fn test_queue_remove() {
        let mut queue = get_queue();

        let t = queue.delete(0);
        assert!(t.is_some());

        if let Some(t) = queue.get() {
            assert_eq!(t.title, "Track 2");
        }
    }

    #[test]
    fn test_queue_next() {
        let queue = get_queue();

        // Rotate left 1
        queue.lrotate(1);

        let track = queue.get();
        assert!(track.is_some());

        let track = track.unwrap();
        assert_eq!(track.title, "Track 2");
    }

    #[test]
    fn test_queue_previous() {
        let queue = get_queue();

        // Rotate right 1
        queue.rotate(1);

        let track = queue.get();
        assert!(track.is_some());

        let track = track.unwrap();
        assert_eq!(track.title, "Track 3");
    }

    #[test]
    fn test_queue_next_large() {
        let queue = get_queue();

        // Rotate left 1
        queue.lrotate(11);

        let track = queue.get();
        assert!(track.is_some());

        let track = track.unwrap();
        assert_eq!(track.title, "Track 3");
    }

    #[test]
    fn test_queue_previous_large() {
        let queue = get_queue();

        // Rotate right 1
        queue.rotate(11);

        let track = queue.get();
        assert!(track.is_some());

        let track = track.unwrap();
        assert_eq!(track.title, "Track 2");
    }
}