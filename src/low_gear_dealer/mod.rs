pub mod params;

use std::fmt::Debug;

use async_bincode::tokio::{AsyncBincodeReader, AsyncBincodeWriter};
use async_bincode::AsyncDestination;
use crypto_bigint::{Random, Zero};
use futures_util::{SinkExt, StreamExt};
use log::info;
use serde::{Deserialize, Serialize};

use crate::bgv::poly::crt::CrtPolyParameters;
use crate::bgv::poly::power::PowerPoly;
use crate::bgv::poly::{CrtContext, PolyParameters};
use crate::bgv::residue::native::GenericNativeResidue;
use crate::bgv::residue::vec::GenericResidueVec;
use crate::bgv::residue::GenericResidue;
use crate::bgv::{self, BgvParameters, Ciphertext, Cleartext, PublicKey, SecretKey};
use crate::connection::{Connection, StreamError};

pub trait DealerParameters: PartialEq + Debug + Send + Sync + 'static {
    type PlaintextParams: PolyParameters<Residue = Self::KS>;

    type CiphertextParams: CrtPolyParameters;

    type BgvParams: BgvParameters<
        PlaintextParams = Self::PlaintextParams,
        CiphertextParams = Self::CiphertextParams,
    >;

    type K: GenericNativeResidue;

    type S: GenericNativeResidue;

    type KS: GenericNativeResidue;
}

pub struct LowGearDealer<P>
where
    P: DealerParameters,
{
    bincode_tx: AsyncBincodeWriter<quinn::SendStream, Message<P>, AsyncDestination>,
    bincode_rx: AsyncBincodeReader<quinn::RecvStream, Message<P>>,
    ctx: CrtContext<P::CiphertextParams>,
    sk: SecretKey<P::BgvParams>,
    remote_pk: PublicKey<P::BgvParams>,
    mac_key: P::S,
    remote_mac_key: Ciphertext<P::BgvParams>,
}

#[derive(Deserialize, Serialize)]
#[serde(bound(deserialize = ""))]
#[serde(bound(serialize = ""))]
enum Message<P>
where
    P: DealerParameters,
{
    Init {
        pk: PublicKey<P::BgvParams>,
        mac_key: Ciphertext<P::BgvParams>,
    },
    Tags(Ciphertext<P::BgvParams>),
}

impl<P> LowGearDealer<P>
where
    P: DealerParameters,
{
    pub async fn new(conn: &mut Connection, mac_key: P::S) -> Result<Self, StreamError> {
        let (tx, rx) = conn.open_bi("LowGearDealer").await?;
        let mut bincode_tx = AsyncBincodeWriter::from(tx).for_async();
        let mut bincode_rx = AsyncBincodeReader::from(rx);
        let ctx = CrtContext::gen().await;
        let sk = SecretKey::gen(&ctx).await;
        let pk = PublicKey::gen(&ctx, &sk).await;
        // TODO: Can the noise bound be improved via secret-key encryption?
        let encrypted_mac_key = {
            // TODO: Use Neg once available
            let negative = P::KS::ZERO - P::KS::from_unsigned(mac_key);
            let mut power = PowerPoly::<P::PlaintextParams>::new();
            for coeff in power.coefficients.iter_mut() {
                *coeff = negative;
            }
            bgv::encrypt(&ctx, &pk, &power).await
        };
        let (_, (remote_pk, remote_mac_key)) = tokio::join!(
            // Send our message to the other party.
            async {
                bincode_tx
                    .send(Message::Init {
                        pk,
                        mac_key: encrypted_mac_key,
                    })
                    .await
                    .unwrap();
            },
            // Concurrently receive the message from the other party.
            async {
                match bincode_rx.next().await.unwrap().unwrap() {
                    Message::Init { pk, mac_key } => (pk, mac_key),
                    _ => panic!("Received message with wrong round number"),
                }
            }
        );

        // TODO: Perform ZKPoPK

        Ok(Self {
            bincode_tx,
            bincode_rx,
            ctx,
            sk,
            remote_pk,
            mac_key,
            remote_mac_key,
        })
    }

    pub async fn authenticate(&mut self, values: &[P::K]) -> Vec<P::KS> {
        if values.len() > packing_capacity::<P::PlaintextParams>() {
            panic!(
                "Batch size {} is too large. \
                `LowGearDealer` can authenticate at most {} values at once",
                values.len(),
                packing_capacity::<P::PlaintextParams>(),
            );
        }

        // 2. - 6.
        let (mut tags, tags2) = tokio::join!(
            send_mac_tags(
                &mut self.bincode_tx,
                &self.ctx,
                &self.remote_pk,
                self.mac_key,
                &self.remote_mac_key,
                values
            ),
            recv_mac_tags(&mut self.bincode_rx, &self.ctx, &self.sk, values.len()),
        );

        // 7. - 8.
        for (t, t2) in tags.iter_mut().zip(&tags2) {
            *t += *t2; // TODO: Can we support references on the RHS, too?
        }

        tags
    }

    pub async fn finish(self) {
        let _ = self.bincode_tx.into_inner().finish().await;
    }
}

async fn send_mac_tags<P>(
    bincode_tx: &mut AsyncBincodeWriter<quinn::SendStream, Message<P>, AsyncDestination>,
    ctx: &CrtContext<P::CiphertextParams>,
    remote_pk: &PublicKey<P::BgvParams>,
    mac_key: P::S,
    remote_mac_key: &Ciphertext<P::BgvParams>,
    values: &[P::K],
) -> Vec<P::KS>
where
    P: DealerParameters,
{
    // We skip steps 4-6, because in practice the check in step 6 is not required.  Hence, we also
    // don't need the random element from step 2.

    let plain_e = {
        let mut temp = PowerPoly::<P::PlaintextParams>::new();
        let mut rng = rand::thread_rng();
        for coeff in temp.coefficients.iter_mut().take(values.len()) {
            *coeff = P::KS::random(&mut rng);
        }
        temp
    };

    {
        let plain_values = {
            let mut temp = PowerPoly::<P::PlaintextParams>::new();
            for (coeff, val) in temp.coefficients.iter_mut().zip(values.iter()) {
                *coeff = P::KS::from_unsigned(*val);
            }
            temp
        };
        let mut ciphertext = remote_mac_key.clone();
        ciphertext *= &Cleartext::new(ctx, &plain_values).await;
        ciphertext -= &bgv::encrypt_and_drown(
            ctx,
            remote_pk,
            &plain_e,
            bgv::max_drown_bits::<P::BgvParams>(),
        )
        .await;
        // TODO: return error instead of unwrapping.
        bincode_tx.send(Message::Tags(ciphertext)).await.unwrap();
    }

    let wide_mac_key = P::KS::from_unsigned(mac_key);

    values
        .iter()
        .zip(plain_e.coefficients.iter())
        .map(|(val, tag)| {
            let val = P::KS::from_unsigned(*val);
            *tag + val * wide_mac_key
        })
        .collect()
}

async fn recv_mac_tags<P>(
    bincode_rx: &mut AsyncBincodeReader<quinn::RecvStream, Message<P>>,
    ctx: &CrtContext<P::CiphertextParams>,
    sk: &SecretKey<P::BgvParams>,
    n: usize,
) -> Vec<P::KS>
where
    P: DealerParameters,
{
    // We skip steps 4-6, because in practice the check in step 6 is not required.

    // TODO: return error instead of unwrapping.
    let plain_d = match bincode_rx.next().await.unwrap().unwrap() {
        Message::Tags(ciphertext) => bgv::decrypt(ctx, sk, &ciphertext).await,
        _ => panic!("Received message with wrong round number"),
    };
    info!("Auth: decrypted ciphertext");
    plain_d.coefficients.iter().take(n).copied().collect()
}

const fn packing_capacity<P>() -> usize
where
    P: PolyParameters,
{
    P::CYCLOTOMIC_DEGREE
}
