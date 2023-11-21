use std::collections::VecDeque;
use std::sync::Arc;

use async_trait::async_trait;
use log::warn;
use tokio::sync::{oneshot, Mutex, Semaphore};

use crate::{
    bgv::residue::native::GenericNativeResidue,
    interface::{BatchedPreprocessor, BeaverTriple, Preprocessor},
};

pub struct BufferedPreprocessor<KS, K, const PID: usize>
where
    KS: GenericNativeResidue,
    K: GenericNativeResidue,
{
    queue: Arc<Mutex<VecDeque<BeaverTriple<KS, K, PID>>>>,
    producer_sem: Arc<Semaphore>,
    consumer_sem: Arc<Semaphore>,
    terminated_rx: Option<oneshot::Receiver<()>>,
}

impl<KS, K, const PID: usize> BufferedPreprocessor<KS, K, PID>
where
    KS: GenericNativeResidue,
    K: GenericNativeResidue,
{
    pub fn new<Preproc>(inner: Preproc, budget: usize) -> Self
    where
        Preproc: BatchedPreprocessor<KS, K, PID> + Send + 'static,
    {
        let queue = Arc::default();
        let producer_sem = Arc::new(Semaphore::new(budget + Preproc::BATCH_SIZE)); // Target number of triples
        let consumer_sem = Arc::new(Semaphore::new(0)); // Initial number of triples
        let (terminated_tx, terminated_rx) = oneshot::channel();
        let preproc = Self {
            queue: Arc::clone(&queue),
            producer_sem: Arc::clone(&producer_sem),
            consumer_sem: Arc::clone(&consumer_sem),
            terminated_rx: Some(terminated_rx),
        };

        tokio::task::spawn(async move {
            produce(inner, &queue, &producer_sem, &consumer_sem, terminated_tx).await;
        });

        preproc
    }
}

impl<KS, K, const PID: usize> Drop for BufferedPreprocessor<KS, K, PID>
where
    KS: GenericNativeResidue,
    K: GenericNativeResidue,
{
    fn drop(&mut self) {
        if let Some(_) = self.terminated_rx {
            warn!("BufferedPreprocessor dropped without calling finish()");
            self.producer_sem.close();
        }
    }
}

async fn produce<KS, K, Preproc, const PID: usize>(
    mut inner: Preproc,
    queue: &Mutex<VecDeque<BeaverTriple<KS, K, PID>>>,
    producer_sem: &Semaphore,
    consumer_sem: &Semaphore,
    terminated_tx: oneshot::Sender<()>,
) where
    KS: GenericNativeResidue,
    K: GenericNativeResidue,
    Preproc: BatchedPreprocessor<KS, K, PID>,
{
    loop {
        if let Ok(permit) = producer_sem.acquire_many(Preproc::BATCH_SIZE as u32).await {
            permit.forget();
        } else {
            // TODO: Synchronize producer termination with the remote party.
            inner.finish().await;
            let _ = terminated_tx.send(());
            return;
        }

        let triples = inner.get_beaver_triples().await;
        queue.lock().await.extend(triples.into_iter());

        consumer_sem.add_permits(Preproc::BATCH_SIZE);
    }
}

#[async_trait]
impl<KS, K, const PID: usize> Preprocessor<KS, K, PID> for BufferedPreprocessor<KS, K, PID>
where
    KS: GenericNativeResidue,
    K: GenericNativeResidue,
{
    async fn get_beaver_triples(&mut self, n: usize) -> Vec<BeaverTriple<KS, K, PID>> {
        self.consumer_sem
            .acquire_many(n as u32)
            .await
            .unwrap()
            .forget();

        let vec = {
            let mut queue = self.queue.lock().await;
            queue.drain(..n).collect()
        };

        self.producer_sem.add_permits(n);

        vec
    }

    async fn finish(mut self) {
        if let Some(terminated_rx) = std::mem::take(&mut self.terminated_rx) {
            self.producer_sem.close();
            // This cannot fail, because `produce()` never drops the `Sender` without sending.
            terminated_rx.await.unwrap();
        }
    }
}
