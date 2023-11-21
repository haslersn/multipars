use futures_util::{SinkExt, StreamExt};
use log::info;

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
    channel: BiChannel<Vec<KS>>,
    mac_key: S,
}

impl<KS, S> MacCheckOpener<KS, S>
where
    KS: GenericNativeResidue,
    S: GenericNativeResidue,
{
    pub async fn new(conn: &mut Connection, mac_key: S) -> Result<Self, StreamError> {
        Ok(Self {
            channel: BiChannel::open(conn).await?,
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
        let (rx, tx) = self.channel.split();

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
                "MacCheckOpener::single expected 1 value but received {}",
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
                "MacCheckOpener::single expected 1 value but received {}",
                received.len()
            );
            return Err(MacCheckFailed {});
        }

        let sum = z + received[0];

        if sum != KS::ZERO {
            info!("MacCheckOpener::single failed");
            return Err(MacCheckFailed {});
        }

        Ok(K::from_unsigned(val))
    }

    pub async fn finish(self) {
        let _ = self.channel.writer.into_inner().finish().await;
    }
}
