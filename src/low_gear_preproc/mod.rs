pub mod params;
pub mod truncer;

use std::fmt::Debug;

use async_trait::async_trait;
use crypto_bigint::Random;
use futures_util::{SinkExt, StreamExt};

use crate::bgv::poly::crt::{CrtPoly, CrtPolyParameters};
use crate::bgv::poly::power::PowerPoly;
use crate::bgv::poly::CrtContext;
use crate::bgv::residue::native::GenericNativeResidue;
use crate::bgv::tweaked_interpolation_packing::{
    get_random_unpacked, pack, pack_diagonal, pack_mask, packing_capacity, unpack, TIPParameters,
};
use crate::bgv::zkpopk::prover::{Prover, ResponseAborted};
use crate::bgv::zkpopk::verifier::Verifier;
use crate::bgv::zkpopk::{Challenge, Commitment, Response};
use crate::bgv::PreparedPlaintext;
use crate::bgv::{
    self, residue::GenericResidue, BgvParameters, Ciphertext, Cleartext, PreCiphertext, PublicKey,
    SecretKey,
};
use crate::bi_channel::BiChannel;
use crate::connection::{Connection, StreamError};
use crate::interface::{BatchedPreprocessor, BeaverTriple, Share};
use crate::low_gear_dealer::{DealerParameters, LowGearDealer};
use crate::mac_check_opener::MacCheckOpener;

use self::truncer::Truncer;

// Low gear parameters
pub trait PreprocessorParameters: PartialEq + Debug + Send + Sync + 'static {
    type DealerParams: DealerParameters<K = Self::K, S = Self::S, KS = Self::KS>;

    type PlaintextResidue: GenericNativeResidue;
    type PlaintextParams: TIPParameters<Residue = Self::PlaintextResidue>;
    type CiphertextParams: CrtPolyParameters;

    type BgvParams: BgvParameters<
        PlaintextParams = Self::PlaintextParams,
        CiphertextParams = Self::CiphertextParams,
    >;

    type K: GenericNativeResidue;

    type S: GenericNativeResidue;

    type KS: GenericNativeResidue;

    type KSS: GenericNativeResidue;

    const ZKPOPK_AMORTIZE: usize;

    const ZKPOPK_SND_SEC: usize;

    const ZKPOPK_INV_FAIL_PROB: usize = 256;

    const ZKPOPK_MAX_REPS: usize = 16;
}

pub struct LowGearPreprocessor<P, const PID: usize>
where
    P: PreprocessorParameters,
{
    dealer: LowGearDealer<P::DealerParams>,
    opener: MacCheckOpener<P::KS, P::S>,
    truncer: Truncer<P::S>,

    ch_ciphertext_there: BiChannel<PreCiphertext<P::BgvParams>>,
    ch_commitment: BiChannel<Commitment<P::BgvParams>>,
    ch_challenge: BiChannel<Challenge>,
    ch_response: BiChannel<Result<Response<P::BgvParams>, ResponseAborted>>,
    ch_ciphertext_back: BiChannel<Ciphertext<P::BgvParams>>,

    ctx_cipher: CrtContext<<P::BgvParams as BgvParameters>::CiphertextParams>,
    ctx_plain: CrtContext<P::PlaintextParams>,
    sk: SecretKey<P::BgvParams>,
    pk: PublicKey<P::BgvParams>,
    remote_pk: PublicKey<P::BgvParams>,
    mac_key: P::S,

    a_stack: Vec<(Vec<P::KSS>, Ciphertext<P::BgvParams>)>,
}

impl<P, const PID: usize> LowGearPreprocessor<P, PID>
where
    P: PreprocessorParameters,
{
    pub async fn new(conn: &mut Connection) -> Result<Self, StreamError> {
        let mac_key = P::S::random(&mut rand::thread_rng());

        // Initialize subprotocols
        let dealer = LowGearDealer::new(conn, mac_key).await?;
        let opener = MacCheckOpener::new(conn, mac_key).await?;
        let trunc = Truncer::new(conn, mac_key).await?;

        // Open channels used by this protocol
        let mut ch_init = BiChannel::open(conn).await?;
        let ch_ciphertext_there = BiChannel::open(conn).await?;
        let ch_commitment = BiChannel::open(conn).await?;
        let ch_challenge = BiChannel::open(conn).await?;
        let ch_response = BiChannel::open(conn).await?;
        let ch_ciphertext_back = BiChannel::open(conn).await?;

        // Generate cryptographic material
        let ctx_cipher = CrtContext::gen().await;
        let ctx_plain = CrtContext::gen().await;
        let sk = SecretKey::gen(&ctx_cipher).await;
        let pk = PublicKey::gen(&ctx_cipher, &sk).await;

        // Initial protocol message
        let (rx_init, tx_init) = ch_init.split();
        let (_, remote_pk) = tokio::join!(
            async {
                tx_init.send(pk.clone()).await.unwrap();
            },
            async { rx_init.next().await.unwrap().unwrap() }
        );

        Ok(Self {
            ch_ciphertext_there,
            ch_commitment,
            ch_challenge,
            ch_response,
            ch_ciphertext_back,
            truncer: trunc,
            dealer,
            opener,
            ctx_cipher,
            ctx_plain,
            sk,
            pk,
            remote_pk,
            mac_key,
            a_stack: Vec::new(),
        })
    }

    async fn get_a(&mut self) -> (Vec<P::KSS>, Ciphertext<P::BgvParams>) {
        if self.a_stack.is_empty() {
            let mut unpacked_a_vec = Vec::new();
            let mut pre_cipher_a_vec = Vec::new();

            let (rx_ciphertext, tx_ciphertext) = self.ch_ciphertext_there.split();
            let (rx_commitment, tx_commitment) = self.ch_commitment.split();
            let (rx_challenge, tx_challenge) = self.ch_challenge.split();
            let (rx_response, tx_response) = self.ch_response.split();

            println!("ZKPoK: amortizing over {} ciphertexts", P::ZKPOPK_AMORTIZE);

            tokio::join!(
                async {
                    let mut inputs = Vec::new();
                    for _ in 0..P::ZKPOPK_AMORTIZE {
                        let unpacked_a =
                            get_random_unpacked::<P::PlaintextParams, P::KS>(rand::thread_rng())
                                .iter()
                                .map(|a| P::KSS::from_unsigned(*a))
                                .collect::<Vec<_>>();
                        let power_a =
                            PowerPoly::from_crt(&self.ctx_plain, &pack(&unpacked_a)).await;
                        let mut cipher_a = PreCiphertext::default();
                        let input: PreparedPlaintext<
                            <P::BgvParams as BgvParameters>::PlaintextParams,
                        > = Prover::<P::BgvParams>::encrypt_into(
                            &self.ctx_cipher,
                            &self.pk,
                            &power_a,
                            &mut cipher_a,
                        )
                        .await;
                        tx_ciphertext.send(cipher_a).await.unwrap();
                        inputs.push(input);
                        unpacked_a_vec.push(unpacked_a);
                    }

                    for rep in 0..P::ZKPOPK_MAX_REPS {
                        let prover = Prover::new(
                            P::ZKPOPK_INV_FAIL_PROB,
                            P::ZKPOPK_AMORTIZE,
                            P::ZKPOPK_SND_SEC,
                        );
                        let commitment = prover.commit(&self.ctx_cipher, &self.pk).await;
                        tx_commitment.send(commitment).await.unwrap();

                        let challenge = rx_challenge.next().await.unwrap().unwrap();

                        let response = prover.respond(&inputs, challenge);
                        let is_ok = response.is_ok();
                        tx_response.send(response).await.unwrap();
                        if is_ok {
                            break;
                        }

                        if rep == P::ZKPOPK_MAX_REPS - 1 {
                            panic!("my ZKPoPK still failed after maximum number of attempts")
                        }
                    }
                },
                async {
                    for iteration_num in 0..P::ZKPOPK_AMORTIZE {
                        let cipher_a = rx_ciphertext.next().await.unwrap().unwrap();
                        pre_cipher_a_vec.push(cipher_a);
                        println!(
                            "ZKPoK: received ciphertext {}/{}",
                            iteration_num + 1,
                            P::ZKPOPK_AMORTIZE
                        );
                    }

                    for rep in 0..P::ZKPOPK_MAX_REPS {
                        let commitment = rx_commitment.next().await.unwrap().unwrap();

                        let verifier = Verifier::new(
                            P::ZKPOPK_INV_FAIL_PROB,
                            P::ZKPOPK_AMORTIZE,
                            P::ZKPOPK_SND_SEC,
                        );
                        let challenge = verifier.challenge();
                        tx_challenge.send(*challenge).await.unwrap();
                        let response = rx_response.next().await.unwrap().unwrap();

                        if let Ok(response) = response {
                            if !verifier
                                .verify(
                                    &self.ctx_cipher,
                                    &self.remote_pk,
                                    &pre_cipher_a_vec[..],
                                    commitment,
                                    &response,
                                )
                                .await
                            {
                                panic!("verification of their ZKPoPK failed");
                            }
                            break;
                        }

                        if rep == P::ZKPOPK_MAX_REPS - 1 {
                            panic!("their ZKPoPK still failed after maximum number of attempts")
                        }
                    }

                    println!("ZKPoK: verification successful");
                }
            );

            for (unpacked_a, pre_cipher_a) in
                unpacked_a_vec.into_iter().zip(pre_cipher_a_vec.into_iter())
            {
                let cipher_a = pre_cipher_a.ciphertext(&self.ctx_cipher).await;
                self.a_stack.push((unpacked_a, cipher_a));
            }
        }

        self.a_stack.pop().unwrap()
    }
}

#[async_trait]
impl<P, const PID: usize> BatchedPreprocessor<P::KS, P::K, PID> for LowGearPreprocessor<P, PID>
where
    P: PreprocessorParameters,
{
    const BATCH_SIZE: usize = batch_size::<P>();

    async fn get_beaver_triples(&mut self) -> Vec<BeaverTriple<P::KS, P::K, PID>> {
        let mac_key_wide = P::KSS::from_unsigned(self.mac_key);

        let mut triples = Vec::new();
        for iteration_num in 0..P::ZKPOPK_AMORTIZE {
            let (unpacked_wide_a, cipher_a) = self.get_a().await;
            println!(
                "started iteration {}/{}",
                iteration_num + 1,
                P::ZKPOPK_AMORTIZE
            );
            let mut unpacked_wide_a_tags: Vec<_> =
                unpacked_wide_a.iter().map(|a| *a * mac_key_wide).collect();

            let (batch_check_mask, unpacked_b, unpacked_b_tags) = {
                let mut input = get_random_unpacked::<P::PlaintextParams, P::K>(rand::thread_rng());
                input.push(P::K::random(&mut rand::thread_rng()));
                input.push(P::K::random(&mut rand::thread_rng()));
                let mut output = self.dealer.authenticate(&input).await;
                let r = Share::new(
                    P::KS::from_unsigned(input.pop().unwrap()),
                    output.pop().unwrap(),
                );
                let m = Share::new(
                    P::KS::from_unsigned(input.pop().unwrap()),
                    output.pop().unwrap(),
                );
                (m + (r << P::K::BITS), input, output)
            };

            let mut unpacked_wide_c: Vec<_> = unpacked_wide_a
                .iter()
                .zip(&unpacked_b)
                .map(|(a, b)| *a * P::KSS::from_unsigned(*b))
                .collect();
            let mut unpacked_wide_c_tags: Vec<_> = unpacked_wide_a
                .iter()
                .zip(&unpacked_b_tags)
                .map(|(a, b_tag)| *a * P::KSS::from_unsigned(*b_tag))
                .collect();

            let unpacked_e_arr = [(); 3]
                .map(|_| get_random_unpacked::<P::PlaintextParams, P::KSS>(rand::thread_rng()));

            let (rx_ciphertext, tx_ciphertext) = self.ch_ciphertext_back.split();

            tokio::join!(
                async {
                    let unpacked_wide_b: Vec<_> = unpacked_b
                        .iter()
                        .map(|b| P::KSS::from_unsigned(*b))
                        .collect();
                    let unpacked_wide_b_tags: Vec<_> = unpacked_b_tags
                        .iter()
                        .map(|b_tag| P::KSS::from_unsigned(*b_tag))
                        .collect();
                    for (i, unpacked_e) in unpacked_e_arr.iter().enumerate() {
                        let power_e = pack_mask(unpacked_e);
                        let mut cipher_d = cipher_a.clone();
                        cipher_d *= &Cleartext::new(
                            &self.ctx_cipher,
                            &PowerPoly::from_crt(
                                &self.ctx_plain,
                                &match i {
                                    0 => pack_diagonal(self.mac_key),
                                    1 => pack(&unpacked_wide_b),
                                    _ => pack(&unpacked_wide_b_tags),
                                },
                            )
                            .await,
                        )
                        .await;
                        cipher_d -= &bgv::encrypt_and_drown(
                            &self.ctx_cipher,
                            &self.remote_pk,
                            &PowerPoly::from_crt(&self.ctx_plain, &power_e).await,
                            bgv::max_drown_bits::<P::BgvParams>(),
                        )
                        .await;
                        // TODO: return error instead of unwrapping.
                        tx_ciphertext.send(cipher_d).await.unwrap();
                    }
                },
                async {
                    for (i, unpacked_e) in unpacked_e_arr.iter().enumerate() {
                        // TODO: return error instead of unwrapping.
                        let cipher_d = rx_ciphertext.next().await.unwrap().unwrap();
                        let plain_d = bgv::decrypt(&self.ctx_cipher, &self.sk, &cipher_d).await;
                        // TODO: return error instead of unwrapping when unpacking fails.
                        let unpacked_d = unpack::<_, P::KSS>(
                            &CrtPoly::from_power(&self.ctx_plain, &plain_d).await,
                        )
                        .unwrap();
                        println!("VOLE: decrypted & unpacked {}/3", i + 1);
                        let target = match i {
                            0 => &mut unpacked_wide_a_tags,
                            1 => &mut unpacked_wide_c,
                            _ => &mut unpacked_wide_c_tags,
                        };
                        for ((d, e), t) in unpacked_d.iter().zip(unpacked_e).zip(target) {
                            *t += *d + *e;
                        }
                    }
                }
            );

            let (unpacked_a, unpacked_a_tags, unpacked_c, unpacked_c_tags) = self
                .truncer
                .truncate::<_, _, _, PID>(
                    &unpacked_wide_a,
                    &unpacked_wide_a_tags,
                    &unpacked_b,
                    &unpacked_b_tags,
                    &unpacked_wide_c,
                    &unpacked_wide_c_tags,
                )
                .await;

            triples.extend(
                unpacked_a
                    .iter()
                    .zip(&unpacked_a_tags)
                    .zip(&unpacked_b)
                    .zip(&unpacked_b_tags)
                    .zip(&unpacked_c)
                    .zip(&unpacked_c_tags)
                    .map(|(((((a, a_tag), b), b_tag), c), c_tag)| {
                        BeaverTriple::new(
                            Share::new(*a, *a_tag),
                            Share::new(P::KS::from_unsigned(*b), *b_tag),
                            Share::new(*c, *c_tag),
                        )
                    }),
            );

            self.opener
                .batch_check::<P::K, PID>([].into_iter(), batch_check_mask)
                .await
                .unwrap();
        }

        assert!(self.a_stack.is_empty());

        println!("batch of size {} completed", triples.len());

        triples
    }

    async fn finish(self) {
        self.dealer.finish().await;
        self.opener.finish().await;
    }
}

pub const fn batch_size<P>() -> usize
where
    P: PreprocessorParameters,
{
    P::ZKPOPK_AMORTIZE * packing_capacity::<P::PlaintextParams>()
}

#[cfg(test)]
mod tests {}
