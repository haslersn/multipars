use std::marker::PhantomData;

use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha20Rng;

use crate::bgv::{
    poly::{CrtContext, PolyParameters},
    zkpopk, BgvParameters, PreCiphertext, PublicKey,
};

use super::{check_bounds, Challenge, Commitment, Response};

pub struct Verifier<P>
where
    P: BgvParameters,
{
    inv_fail_prob: usize,
    num_ciphertexts: usize,
    num_proofs: usize,
    challenge: Challenge,
    phantom: PhantomData<P>,
}

pub struct VerificationFailed;

impl<P> Verifier<P>
where
    P: BgvParameters,
{
    pub fn new(inv_fail_prob: usize, num_ciphertexts: usize, snd_sec: usize) -> Self {
        let num_proofs = zkpopk::num_proofs::<P>(snd_sec);
        let mut rng = rand::thread_rng();
        let challenge = Challenge(rng.gen());
        Self {
            inv_fail_prob,
            num_ciphertexts,
            num_proofs,
            challenge,
            phantom: PhantomData::default(),
        }
    }

    pub fn challenge(&self) -> &Challenge {
        &self.challenge
    }

    pub async fn verify(
        self,
        ctx: &CrtContext<P::CiphertextParams>,
        pk: &PublicKey<P>,
        ciphertexts: &[PreCiphertext<P>],
        commitment: Commitment<P>,
        response: &Response<P>,
    ) -> bool {
        if commitment.0.len() != self.num_proofs {
            return false;
        }
        if response.0.len() != self.num_proofs {
            return false;
        }

        for prepared_plaintext in &response.0 {
            if !check_bounds::<P>(
                prepared_plaintext,
                self.inv_fail_prob,
                self.num_ciphertexts,
                self.num_proofs,
            ) {
                return false;
            }
        }

        let mut prng = ChaCha20Rng::from_seed(self.challenge.0);
        let mut accumulated = commitment.0;
        for acc in &mut accumulated {
            for output in ciphertexts {
                let challenge = prng.gen_range(0..P::PlaintextParams::M);
                acc.c_0.add_assign_slided(&output.c_0, challenge);
                acc.c_1.add_assign_slided(&output.c_1, challenge);
            }
        }

        let mut ciphertext = PreCiphertext::default();
        for (prepared_plaintext, acc) in response.0.iter().zip(&accumulated) {
            prepared_plaintext
                .encrypt_into(ctx, pk, &mut ciphertext)
                .await;
            if &ciphertext != acc {
                return false;
            }
        }

        true
    }
}
