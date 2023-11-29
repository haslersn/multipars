use futures_util::{SinkExt, StreamExt};
use log::info;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha20Rng;

use crate::bgv::residue::native::GenericNativeResidue;
use crate::bi_channel::BiChannel;
use crate::connection::{Connection, StreamError};
use crate::interface::Share;

#[derive(Debug, derive_more::Display, derive_more::Error)]
pub struct MacCheckFailed {}

pub struct MacCheckOpener<KS, S>
where
    KS: GenericNativeResidue,
    S: GenericNativeResidue,
{
    ch_values: BiChannel<Vec<KS>>,
    ch_seed: BiChannel<[u8; 32]>,
    mac_key: S,
}

impl<KS, S> MacCheckOpener<KS, S>
where
    KS: GenericNativeResidue,
    S: GenericNativeResidue,
{
    pub async fn new(conn: &mut Connection, mac_key: S) -> Result<Self, StreamError> {
        Ok(Self {
            ch_values: BiChannel::open(conn).await?,
            ch_seed: BiChannel::open(conn).await?,
            mac_key,
        })
    }
}

impl<KS, S> MacCheckOpener<KS, S>
where
    KS: GenericNativeResidue,
    S: GenericNativeResidue,
{
    pub async fn single_check<K, const PID: usize>(
        &mut self,
        share: Share<KS, K, PID>,
    ) -> Result<K, MacCheckFailed>
    where
        K: GenericNativeResidue,
    {
        let (rx, tx) = self.ch_values.split();

        let (_, received) = tokio::join!(
            async {
                let mut values = Vec::new();
                values.push(share.val);
                tx.send(values).await.unwrap();
            },
            async { rx.next().await.unwrap().unwrap() }
        );

        if received.len() != 1 {
            info!(
                "MacCheckOpener::single_check expected 1 value but received {}",
                received.len()
            );
            return Err(MacCheckFailed {});
        }

        let val = share.val + received[0];
        let z = share.tag - val * KS::from_unsigned(self.mac_key);

        let (_, received) = tokio::join!(
            async {
                let mut values = Vec::new();
                values.push(z);
                tx.send(values).await.unwrap();
            },
            async { rx.next().await.unwrap().unwrap() }
        );

        if received.len() != 1 {
            info!(
                "MacCheckOpener::single_check expected 1 value but received {}",
                received.len()
            );
            return Err(MacCheckFailed {});
        }

        let sum = z + received[0];

        if sum != KS::ZERO {
            info!("MacCheckOpener::single_check failed");
            return Err(MacCheckFailed {});
        }

        println!("MacCheck: check passed");

        Ok(K::from_unsigned(val))
    }

    pub async fn batch_check<K, const PID: usize>(
        &mut self,
        shares: impl Iterator<Item = Share<KS, K, PID>>,
        mut mask: Share<KS, K, PID>,
    ) -> Result<(), MacCheckFailed>
    where
        K: GenericNativeResidue,
    {
        let (rx, tx) = self.ch_seed.split();

        let local_seed: [u8; 32] = rand::thread_rng().gen();

        tokio::join!(
            async {
                tx.send(local_seed).await.unwrap();
            },
            async {
                let remote_seed = rx.next().await.unwrap().unwrap();
                let mut seed = local_seed.clone();
                for (dst, src) in seed.iter_mut().zip(remote_seed) {
                    *dst ^= src;
                }
                let mut prng = ChaCha20Rng::from_seed(seed);
                for share in shares {
                    // TODO: random value should be in S
                    mask += share * K::random(&mut prng);
                }
            }
        );

        self.single_check(mask).await?;
        Ok(())
    }

    pub async fn finish(self) {
        let _ = self.ch_values.writer.into_inner().finish().await;
    }
}
