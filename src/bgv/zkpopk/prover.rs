use std::marker::PhantomData;

use crypto_bigint::{Random, Zero};
use rand::{CryptoRng, Rng, RngCore, SeedableRng};
use rand_chacha::ChaCha20Rng;
use serde::{Deserialize, Serialize};

use crate::bgv::{
    self,
    generic_uint::{ExtendableUint, GenericUint},
    poly::{power::PowerPoly, CrtContext, PolyParameters},
    residue::GenericResidue,
    zkpopk, BgvParameters, PreCiphertext, PreparedPlaintext, PublicKey,
};

use super::{check_bounds, Challenge, Commitment, Response};

pub struct Prover<P>
where
    P: BgvParameters,
{
    inv_fail_prob: usize,
    num_ciphertexts: usize,
    num_proofs: usize,
    pseudo_inputs: Vec<PreparedPlaintext<P::PlaintextParams>>,
}

#[derive(Debug, derive_more::Display, derive_more::Error, Deserialize, Serialize)]
pub struct ResponseAborted;

impl<P> Prover<P>
where
    P: BgvParameters,
{
    pub async fn encrypt_into(
        ctx: &CrtContext<P::CiphertextParams>,
        pk: &PublicKey<P>,
        plaintext: &PowerPoly<P::PlaintextParams>,
        ciphertext: &mut PreCiphertext<P>,
    ) -> PreparedPlaintext<P::PlaintextParams>
    where
        P: BgvParameters,
    {
        let input = bgv::prepare(plaintext);
        input.encrypt_into(ctx, pk, ciphertext).await;
        input
    }

    pub fn new(inv_fail_prob: usize, num_ciphertexts: usize, snd_sec: usize) -> Self {
        let num_proofs = zkpopk::num_proofs::<P>(snd_sec);
        let mut rng = rand::thread_rng();
        let pseudo_inputs = (0..num_proofs)
            .map(|_| {
                make_pseudo_input::<P, _>(&mut rng, inv_fail_prob, num_ciphertexts, num_proofs)
            })
            .collect();
        Self {
            inv_fail_prob,
            num_ciphertexts,
            num_proofs,
            pseudo_inputs,
        }
    }

    pub async fn commit(
        &self,
        ctx: &CrtContext<P::CiphertextParams>,
        pk: &PublicKey<P>,
    ) -> Commitment<P> {
        let mut ciphertexts = Vec::new();
        for pi in &self.pseudo_inputs {
            let mut ciphertext = PreCiphertext::default();
            pi.encrypt_into(ctx, pk, &mut ciphertext).await;
            ciphertexts.push(ciphertext);
        }
        Commitment(ciphertexts)
    }

    pub fn respond(
        self,
        inputs: &[PreparedPlaintext<P::PlaintextParams>],
        challenge: Challenge,
    ) -> Result<Response<P>, ResponseAborted> {
        debug_assert_eq!(self.num_ciphertexts, inputs.len());

        // TODO: Use random oracle instead
        let mut prng = ChaCha20Rng::from_seed(challenge.0);
        let mut accumulated = self.pseudo_inputs;
        for acc in &mut accumulated {
            for input in inputs {
                let challenge = prng.gen_range(0..P::PlaintextParams::M);
                acc.add_assign_slided(input, challenge);
            }
            if !check_bounds::<P>(
                acc,
                self.inv_fail_prob,
                self.num_ciphertexts,
                self.num_proofs,
            ) {
                return Err(ResponseAborted);
            }
        }
        Ok(Response(accumulated))
    }
}

fn make_pseudo_input<P, Rng>(
    mut rng: Rng,
    inv_fail_prob: usize,
    num_ciphertexts: usize,
    num_proofs: usize,
) -> PreparedPlaintext<P::PlaintextParams>
where
    P: BgvParameters,
    Rng: CryptoRng + RngCore,
{
    let bound = ((3 * (P::PlaintextParams::M - 1) * num_proofs * inv_fail_prob + 1)
        * (P::PlaintextParams::M - 1)
        * num_ciphertexts) as i64;

    type ExtendedUint<P> =
        <<<<P as BgvParameters>::PlaintextParams as PolyParameters>::Residue as GenericResidue>::Uint as ExtendableUint>::Extended;

    assert!(
        ExtendedUint::<P>::NLIMBS * 64
            > 64 - (21 * bound).leading_zeros() as usize + P::PlaintextResidue::BITS
    );

    let noised_plaintext = (0..P::PlaintextParams::CYCLOTOMIC_DEGREE)
        .map(|_| {
            let sample = rng.gen_range(-21 * bound..21 * bound);
            let sample = ExtendedUint::<P>::from_i64(sample);
            let shifted = sample << P::PlaintextResidue::BITS;

            let value = P::PlaintextResidue::random(&mut rng).retrieve();
            // TODO: Use `GenericUint::from_uint()` once implemented
            let mut lhs = ExtendedUint::<P>::ZERO;
            for (src, dst) in value.limbs().iter().zip(lhs.limbs_mut()) {
                *dst = *src;
            }
            lhs | shifted
        })
        .collect();

    let e_1 = (0..P::PlaintextParams::CYCLOTOMIC_DEGREE)
        .map(|_| rng.gen_range(-20 * bound..20 * bound))
        .collect();

    let v = (0..P::PlaintextParams::CYCLOTOMIC_DEGREE)
        .map(|_| rng.gen_range(-bound..bound))
        .collect();

    PreparedPlaintext {
        noised_plaintext,
        e_1,
        v,
        phantom: PhantomData::default(),
    }
}
