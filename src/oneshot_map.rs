use std::collections::hash_map::Entry::{Occupied, Vacant};
use std::collections::HashMap;
use std::hash::Hash;

use tokio::sync::{oneshot, Mutex};

#[derive(Debug, derive_more::Display, derive_more::Error)]
pub struct SendBusy {}

#[derive(Debug, derive_more::Display, derive_more::Error)]
pub struct RecvBusy {}

pub struct OneshotMap<K, V> {
    inner: Mutex<HashMap<K, Inner<V>>>,
}

enum Inner<V> {
    Sender(oneshot::Sender<V>),
    Receiver(oneshot::Receiver<V>),
}

impl<K: Eq + Hash, V> OneshotMap<K, V> {
    pub async fn send(&self, k: K, v: V) -> Result<(), SendBusy> {
        let tx = match self.inner.lock().await.entry(k) {
            Occupied(entry) => match entry.get() {
                Inner::Sender(_) => match entry.remove_entry().1 {
                    Inner::Sender(tx) => tx,
                    _ => panic!(),
                },
                Inner::Receiver(_) => return Err(SendBusy {}),
            },
            Vacant(entry) => {
                let (tx, rx) = oneshot::channel();
                entry.insert(Inner::Receiver(rx));
                tx
            }
        };
        tx.send(v).ok().expect("Failed to send value");
        Ok(())
    }

    pub async fn recv(&self, k: K) -> Result<V, RecvBusy> {
        let rx = match self.inner.lock().await.entry(k) {
            Occupied(entry) => match entry.get() {
                Inner::Sender(_) => return Err(RecvBusy {}),
                Inner::Receiver(_) => match entry.remove_entry().1 {
                    Inner::Receiver(rx) => rx,
                    _ => panic!(),
                },
            },
            Vacant(entry) => {
                let (tx, rx) = oneshot::channel();
                entry.insert(Inner::Sender(tx));
                rx
            }
        };
        Ok(rx.await.expect("Failed to receive value"))
    }
}

impl<K, V> Default for OneshotMap<K, V> {
    fn default() -> Self {
        Self {
            inner: Default::default(),
        }
    }
}
