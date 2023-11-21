use serde::{Deserialize, Serialize};

use crate::bgv::generic_uint::ExtendableUint;

use super::{
    generic_uint::GenericUint, poly::PolyParameters, residue::GenericResidue, BgvParameters,
    PreCiphertext, PreparedPlaintext,
};

pub mod prover;
pub mod verifier;

#[derive(Deserialize, Serialize)]
#[serde(bound(deserialize = ""))]
#[serde(bound(serialize = ""))]
pub struct Commitment<P>(Vec<PreCiphertext<P>>)
where
    P: BgvParameters;

#[derive(Clone, Copy, Deserialize, Serialize)]
pub struct Challenge([u8; 32]);

#[derive(Deserialize, Serialize)]
pub struct Response<P>(Vec<PreparedPlaintext<P::PlaintextParams>>)
where
    P: BgvParameters;

fn check_bounds<P>(
    prepared_plaintext: &PreparedPlaintext<P::PlaintextParams>,
    inv_fail_prob: usize,
    num_ciphertexts: usize,
    num_proofs: usize,
) -> bool
where
    P: BgvParameters,
{
    type ExtendedUint<P> =
        <<<<P as BgvParameters>::PlaintextParams as PolyParameters>::Residue as GenericResidue>::Uint as ExtendableUint>::Extended;

    let bound = (3
        * (P::PlaintextParams::M - 1)
        * (P::PlaintextParams::M - 1)
        * num_ciphertexts
        * num_proofs
        * inv_fail_prob) as i64;

    let shifted_bound = ExtendedUint::<P>::from_i64(21 * bound) << P::PlaintextResidue::BITS;

    for uint in &prepared_plaintext.noised_plaintext {
        let positive_val = uint.wrapping_add(&shifted_bound);
        let positive_bound = shifted_bound << 1;
        if positive_val >= positive_bound {
            return false;
        }
    }
    for val in &prepared_plaintext.e_1 {
        if -20 * bound > *val || *val >= 20 * bound {
            return false;
        }
    }
    for val in &prepared_plaintext.v {
        if -bound > *val || *val >= bound {
            return false;
        }
    }

    true
}

pub fn num_proofs<P>(snd_sec: usize) -> usize
where
    P: BgvParameters,
{
    ((snd_sec + 2) as f64 / ((P::PlaintextParams::M - 1) as f64).log2()).ceil() as usize
}

#[cfg(test)]
mod tests {
    use crate::bgv::{
        params::ToyBgv,
        poly::{power::PowerPoly, CrtContext},
        PreCiphertext, PublicKey, SecretKey,
    };

    use super::{prover::Prover, verifier::Verifier};

    #[tokio::test]
    async fn zkpopk() {
        const INV_FAIL_PROB: usize = 1 << 20;
        const NUM_CIPHERTEXTS: usize = 5;
        const SND_SEC: usize = 64;

        let mut rng = rand::thread_rng();
        let ctx = CrtContext::gen().await;
        let sk = SecretKey::<ToyBgv>::gen(&ctx).await;
        let pk = PublicKey::gen(&ctx, &sk).await;
        let mut ciphertexts = Vec::new();
        let mut inputs = Vec::new();
        for _ in 0..NUM_CIPHERTEXTS {
            let plaintext = PowerPoly::random(&mut rng);
            let mut ciphertext = PreCiphertext::default();
            let input = Prover::encrypt_into(&ctx, &pk, &plaintext, &mut ciphertext).await;
            ciphertexts.push(ciphertext);
            inputs.push(input);
        }

        let prover = Prover::<ToyBgv>::new(INV_FAIL_PROB, NUM_CIPHERTEXTS, SND_SEC);
        let commitment = prover.commit(&ctx, &pk).await;

        let verifier = Verifier::new(INV_FAIL_PROB, NUM_CIPHERTEXTS, SND_SEC);
        let challenge = verifier.challenge();

        let response = prover.respond(&inputs, *challenge).unwrap();

        assert!(
            verifier
                .verify(&ctx, &pk, &ciphertexts, commitment, &response)
                .await
        );
    }
}
