pub mod fourier;
pub mod generic_uint;
pub mod params;
pub mod poly;
pub mod residue;
pub mod tweaked_interpolation_packing;
pub mod zkpopk;

use std::{
    fmt::Debug,
    marker::PhantomData,
    ops::{AddAssign, MulAssign, SubAssign},
};

use crypto_bigint::{Integer, Limb, Word, Zero};
use rand::{CryptoRng, Rng, RngCore};
use serde::{Deserialize, Serialize};

use crate::bgv::generic_uint::GenericUint;

use self::{
    generic_uint::ExtendableUint,
    poly::{crt::CrtPoly, power::PowerPoly, CrtContext, FourierCrtPolyParameters, PolyParameters},
    residue::{native::GenericNativeResidue, vec::GenericResidueVec, GenericResidue},
};

pub trait BgvParameters: PartialEq + Debug + Send + 'static {
    type PlaintextUint: ExtendableUint;
    type PlaintextResidue: GenericNativeResidue<Uint = Self::PlaintextUint>;
    type PlaintextParams: PolyParameters<Residue = Self::PlaintextResidue>;
    type CiphertextParams: FourierCrtPolyParameters;
}

impl<PlaintextParams, CiphertextParams> BgvParameters for (PlaintextParams, CiphertextParams)
where
    <PlaintextParams::Residue as GenericResidue>::Uint: ExtendableUint,
    PlaintextParams::Residue: GenericNativeResidue,
    PlaintextParams: PolyParameters,
    CiphertextParams: FourierCrtPolyParameters,
{
    type PlaintextUint = <PlaintextParams::Residue as GenericResidue>::Uint;
    type PlaintextResidue = PlaintextParams::Residue;
    type PlaintextParams = PlaintextParams;
    type CiphertextParams = CiphertextParams;
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct SecretKey<P>
where
    P: BgvParameters,
{
    s: CrtPoly<P::CiphertextParams>,
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct PublicKey<P>
where
    P: BgvParameters,
{
    pub b: CrtPoly<P::CiphertextParams>, // TODO: non-public
    pub a: CrtPoly<P::CiphertextParams>, // TODO: non-public
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct Ciphertext<P>
where
    P: BgvParameters,
{
    pub c_0: CrtPoly<P::CiphertextParams>, // TODO: non-public
    pub c_1: CrtPoly<P::CiphertextParams>, // TODO: non-public
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct PreCiphertext<P>
where
    P: BgvParameters,
{
    pub c_0: PowerPoly<P::CiphertextParams>, // TODO: non-public
    pub c_1: PowerPoly<P::CiphertextParams>, // TODO: non-public
}

// We give `P` as a generic parameter, because `P::CYCLOTOMIC_DEGREE` determines the length of the
// stored vectors.
#[derive(Deserialize, Serialize)]
pub struct PreparedPlaintext<P>
where
    P: PolyParameters,
    <P::Residue as GenericResidue>::Uint: ExtendableUint,
{
    noised_plaintext: Vec<<<P::Residue as GenericResidue>::Uint as ExtendableUint>::Extended>,
    e_1: Vec<i64>,
    v: Vec<i64>,
    phantom: PhantomData<P>,
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct Cleartext<P>(CrtPoly<P::CiphertextParams>)
where
    P: BgvParameters;

impl<P> Clone for SecretKey<P>
where
    P: BgvParameters,
{
    fn clone(&self) -> Self {
        Self { s: self.s.clone() }
    }

    fn clone_from(&mut self, source: &Self) {
        self.s.clone_from(&source.s);
    }
}

impl<P> Clone for PublicKey<P>
where
    P: BgvParameters,
{
    fn clone(&self) -> Self {
        Self {
            b: self.b.clone(),
            a: self.a.clone(),
        }
    }

    fn clone_from(&mut self, source: &Self) {
        self.b.clone_from(&source.b);
        self.a.clone_from(&source.a);
    }
}

impl<P> Clone for Ciphertext<P>
where
    P: BgvParameters,
{
    fn clone(&self) -> Self {
        Self {
            c_0: self.c_0.clone(),
            c_1: self.c_1.clone(),
        }
    }

    fn clone_from(&mut self, source: &Self) {
        self.c_0.clone_from(&source.c_0);
        self.c_1.clone_from(&source.c_1);
    }
}

impl<P> AddAssign<&Self> for Ciphertext<P>
where
    P: BgvParameters,
{
    fn add_assign(&mut self, rhs: &Self) {
        self.c_0 += &rhs.c_0;
        self.c_1 += &rhs.c_1;
    }
}

impl<P> AddAssign<&Cleartext<P>> for Ciphertext<P>
where
    P: BgvParameters,
{
    fn add_assign(&mut self, rhs: &Cleartext<P>) {
        self.c_0 += &rhs.0;
    }
}

impl<P> SubAssign<&Self> for Ciphertext<P>
where
    P: BgvParameters,
{
    fn sub_assign(&mut self, rhs: &Self) {
        self.c_0 -= &rhs.c_0;
        self.c_1 -= &rhs.c_1;
    }
}

impl<P> SubAssign<&Cleartext<P>> for Ciphertext<P>
where
    P: BgvParameters,
{
    fn sub_assign(&mut self, rhs: &Cleartext<P>) {
        self.c_0 -= &rhs.0;
    }
}

impl<P> MulAssign<&Cleartext<P>> for Ciphertext<P>
where
    P: BgvParameters,
{
    fn mul_assign(&mut self, rhs: &Cleartext<P>) {
        self.c_0 *= &rhs.0;
        self.c_1 *= &rhs.0;
    }
}

impl<P> Cleartext<P>
where
    P: BgvParameters,
{
    pub async fn new(
        ctx: &CrtContext<P::CiphertextParams>,
        plaintext: &PowerPoly<P::PlaintextParams>,
    ) -> Self {
        let extended = PowerPoly::from_power(plaintext);
        let crt = CrtPoly::from_power(ctx, &extended).await;
        Self(crt)
    }
}

pub async fn encrypt<P>(
    ctx: &CrtContext<P::CiphertextParams>,
    pk: &PublicKey<P>,
    plaintext: &PowerPoly<P::PlaintextParams>,
) -> Ciphertext<P>
where
    P: BgvParameters,
{
    let mut pre_ct = PreCiphertext::default();
    prepare(plaintext).encrypt_into(ctx, pk, &mut pre_ct).await;
    pre_ct.ciphertext(ctx).await
}

pub async fn encrypt_into<P>(
    ctx: &CrtContext<P::CiphertextParams>,
    pk: &PublicKey<P>,
    plaintext: &PowerPoly<P::PlaintextParams>,
    ciphertext: &mut Ciphertext<P>,
) where
    P: BgvParameters,
{
    let mut pre_ct = PreCiphertext::default();
    prepare(plaintext).encrypt_into(ctx, pk, &mut pre_ct).await;
    pre_ct.ciphertext_into(ctx, ciphertext).await;
}

pub async fn encrypt_and_drown<P>(
    ctx: &CrtContext<P::CiphertextParams>,
    pk: &PublicKey<P>,
    plaintext: &PowerPoly<P::PlaintextParams>,
    noise_bits: usize,
) -> Ciphertext<P>
where
    P: BgvParameters,
{
    let mut ct = Ciphertext::default();
    encrypt_and_drown_into(ctx, pk, plaintext, &mut ct, noise_bits).await;
    ct
}

pub async fn encrypt_and_drown_into<P>(
    ctx: &CrtContext<P::CiphertextParams>,
    pk: &PublicKey<P>,
    plaintext: &PowerPoly<P::PlaintextParams>,
    ciphertext: &mut Ciphertext<P>,
    noise_bits: usize,
) where
    P: BgvParameters,
{
    type CiphertextResidue<P> =
        <<<P as BgvParameters>::CiphertextParams as PolyParameters>::Residue as GenericResidue>::Uint;
    type ExtendedUint<P> =
        <<<<P as BgvParameters>::PlaintextParams as PolyParameters>::Residue as GenericResidue>::Uint as ExtendableUint>::Extended;

    let noised_plaintext: Vec<CiphertextResidue<P>> = add_uniform_scaled(plaintext, noise_bits);
    // We approximate the discrete gaussian distribution of variance 10 with
    // the centered binomial distribution of variance 10.  So the number of
    // iterations and the maximum magnitude is 20.
    let e_1: Vec<ExtendedUint<P>> =
        add_centered_binomial_scaled(&PowerPoly::<P::PlaintextParams>::new(), 20);
    let v = sample_centered_binomial::<P::PlaintextParams>(1);

    let mut temp_power = PowerPoly::new();
    let mut temp_crt = CrtPoly::new();

    temp_power.clone_from_i64s(&v);
    temp_crt.clone_from_power(ctx, &temp_power).await;

    ciphertext.c_0.clone_from(&pk.b);
    ciphertext.c_1.clone_from(&pk.a);

    ciphertext.c_0 *= &temp_crt;
    ciphertext.c_1 *= &temp_crt;

    temp_power.clone_from_signed_ints(&noised_plaintext);
    temp_crt.clone_from_power(ctx, &temp_power).await;
    ciphertext.c_0 += &temp_crt;

    temp_power.clone_from_signed_ints(&e_1);
    temp_crt.clone_from_power(ctx, &temp_power).await;
    ciphertext.c_1 += &temp_crt;
}

fn prepare<P>(plaintext: &PowerPoly<P>) -> PreparedPlaintext<P>
where
    P: PolyParameters,
    P::Residue: GenericNativeResidue,
    <P::Residue as GenericResidue>::Uint: ExtendableUint,
{
    // We approximate the discrete gaussian distribution of variance 10 with
    // the centered binomial distribution of variance 10.  So the number of
    // iterations and the maximum magnitude is 20.
    let noised_plaintext = add_centered_binomial_scaled(&plaintext, 20);
    let e_1 = sample_centered_binomial::<P>(20);
    let v = sample_centered_binomial::<P>(1);
    PreparedPlaintext {
        noised_plaintext,
        e_1,
        v,
        phantom: PhantomData::default(),
    }
}

impl<P> PreparedPlaintext<P>
where
    P: PolyParameters,
    <P::Residue as GenericResidue>::Uint: ExtendableUint,
{
    async fn encrypt_into<BgvParams>(
        &self,
        ctx: &CrtContext<BgvParams::CiphertextParams>,
        pk: &PublicKey<BgvParams>,
        ciphertext: &mut PreCiphertext<BgvParams>,
    ) where
        BgvParams: BgvParameters<PlaintextParams = P>,
    {
        type ExtendedUint<P> =
            <<<P as PolyParameters>::Residue as GenericResidue>::Uint as ExtendableUint>::Extended;

        let scaled_e_1: Vec<_> = self
            .e_1
            .iter()
            .map(|e| {
                let extended = ExtendedUint::<P>::from_i64(*e);
                extended << BgvParams::PlaintextResidue::BITS
            })
            .collect();

        let mut temp_power = PowerPoly::new();
        let mut temp_crt = CrtPoly::new();

        temp_power.clone_from_i64s(&self.v);
        let v = CrtPoly::from_power(ctx, &temp_power).await;

        temp_crt.clone_from(&pk.b);
        temp_crt *= &v;
        ciphertext.c_0.clone_from_crt(ctx, &temp_crt).await;
        temp_power.clone_from_signed_ints(&self.noised_plaintext);
        ciphertext.c_0 += &temp_power;

        temp_crt.clone_from(&pk.a);
        temp_crt *= &v;
        ciphertext.c_1.clone_from_crt(ctx, &temp_crt).await;
        temp_power.clone_from_signed_ints(&scaled_e_1);
        ciphertext.c_1 += &temp_power;
    }

    fn add_assign_slided(&mut self, rhs: &Self, length: usize) {
        if length == 0 {
            return;
        }
        let mut sum_np = <<P::Residue as GenericResidue>::Uint as ExtendableUint>::Extended::ZERO;
        let mut sum_e_1 = 0;
        let mut sum_v = 0;
        for power in 1..P::M {
            let index = power % (P::M - 1);
            sum_np = sum_np.wrapping_add(&rhs.noised_plaintext[index]);
            sum_e_1 += rhs.e_1[index];
            sum_v += rhs.v[index];
            if power != length {
                let rhs_index = (power + P::M - length) % P::M % (P::M - 1);
                sum_np = sum_np.wrapping_sub(&rhs.noised_plaintext[rhs_index]);
                sum_e_1 -= rhs.e_1[rhs_index];
                sum_v -= rhs.v[rhs_index];
            }
            let np = &mut self.noised_plaintext[index];
            *np = np.wrapping_add(&sum_np);
            self.e_1[index] += sum_e_1;
            self.v[index] += sum_v;
        }
    }
}

impl<P> Default for PreparedPlaintext<P>
where
    P: PolyParameters,
    <P::Residue as GenericResidue>::Uint: ExtendableUint,
{
    fn default() -> Self {
        type ExtendedUint<P> = <<P as PolyParameters>::Uint as ExtendableUint>::Extended;
        Self {
            noised_plaintext: vec![ExtendedUint::<P>::default(); P::CYCLOTOMIC_DEGREE],
            e_1: vec![0; P::CYCLOTOMIC_DEGREE],
            v: vec![0; P::CYCLOTOMIC_DEGREE],
            phantom: PhantomData::default(),
        }
    }
}

impl<P> Clone for PreparedPlaintext<P>
where
    P: PolyParameters,
    <P::Residue as GenericResidue>::Uint: ExtendableUint,
{
    fn clone(&self) -> Self {
        Self {
            noised_plaintext: self.noised_plaintext.clone(),
            e_1: self.e_1.clone(),
            v: self.v.clone(),
            phantom: PhantomData::default(),
        }
    }

    fn clone_from(&mut self, source: &Self) {
        self.noised_plaintext.clone_from(&source.noised_plaintext);
        self.e_1.clone_from(&source.e_1);
        self.v.clone_from(&source.v);
    }
}

pub fn sample_centered_binomial<P>(iterations: usize) -> Vec<i64>
where
    P: PolyParameters,
{
    let mut rng = rand::thread_rng();

    (0..P::CYCLOTOMIC_DEGREE)
        .map(|_| sample_binomial(&mut rng, iterations) as i64 - iterations as i64)
        .collect()
}

fn add_centered_binomial_scaled<P, TargetInt>(
    src: &PowerPoly<P>,
    iterations: usize,
) -> Vec<TargetInt>
where
    P: PolyParameters,
    P::Residue: GenericNativeResidue,
    TargetInt: GenericUint,
{
    let nlimbs = <P::Residue as GenericResidue>::Uint::NLIMBS;

    let mut rng = rand::thread_rng();

    src.coefficients
        .iter()
        .map(|coeff| {
            let sample = sample_binomial(&mut rng, iterations) as i64 - iterations as i64;
            let shifted = TargetInt::from_i64(sample) << P::Residue::BITS;
            let mut lhs = TargetInt::ZERO;
            lhs.limbs_mut()[..nlimbs].clone_from_slice(coeff.retrieve().limbs());
            lhs | shifted
        })
        .collect()
}

// The added noise is between -2^(noise_bits-1) and 2^(noise_bits-1).
fn add_uniform_scaled<P, TargetInt>(src: &PowerPoly<P>, noise_bits: usize) -> Vec<TargetInt>
where
    P: PolyParameters,
    P::Residue: GenericNativeResidue,
    TargetInt: GenericUint,
{
    let nlimbs = <P::Residue as GenericResidue>::Uint::NLIMBS;

    debug_assert!(0 < noise_bits);
    debug_assert!(noise_bits <= TargetInt::NLIMBS * Limb::BITS - P::Residue::BITS);

    let mut rng = rand::thread_rng();
    // Set `minimum` to the expected value of `sample`, in order to center the distribution.
    let minimum = TargetInt::from_u32(1) << (noise_bits - 1);

    src.coefficients
        .iter()
        .map(|coeff| {
            let mut sample = TargetInt::ZERO;
            let mut remaining_noise_bits = noise_bits;
            for limb in &mut sample.limbs_mut()[..(noise_bits + 63) / 64] {
                limb.0 = if remaining_noise_bits >= 64 {
                    remaining_noise_bits -= 64;
                    rng.gen::<Word>()
                } else {
                    rng.gen_range(0..1 << remaining_noise_bits)
                };
            }
            let shifted = sample.wrapping_sub(&minimum) << P::Residue::BITS;
            let mut lhs = TargetInt::ZERO;
            lhs.limbs_mut()[..nlimbs].clone_from_slice(coeff.retrieve().limbs());
            lhs | shifted
        })
        .collect()
}

fn sample_binomial(mut rng: impl CryptoRng + RngCore, iterations: usize) -> u32 {
    debug_assert!(2 * iterations <= Limb::BITS);
    let bound: Word = 1 << (2 * iterations);
    let bits = rng.gen::<Word>() & bound.wrapping_sub(1);
    bits.count_ones()
}

pub async fn decrypt<P>(
    ctx: &CrtContext<P::CiphertextParams>,
    secret_key: &SecretKey<P>,
    ciphertext: &Ciphertext<P>,
) -> PowerPoly<P::PlaintextParams>
where
    P: BgvParameters,
{
    let mut power = PowerPoly::new();
    decrypt_into(ctx, secret_key, ciphertext, &mut power).await;
    power
}

pub async fn decrypt_into<P>(
    ctx: &CrtContext<P::CiphertextParams>,
    secret_key: &SecretKey<P>,
    ciphertext: &Ciphertext<P>,
    plaintext: &mut PowerPoly<P::PlaintextParams>,
) where
    P: BgvParameters,
{
    let noise_max = <<P::CiphertextParams as PolyParameters>::Residue as GenericResidue>::Uint::ONE
        << (<P::CiphertextParams as PolyParameters>::Residue::BITS - 1);

    let mut temp = ciphertext.c_1.clone();
    temp *= &secret_key.s;
    temp -= &ciphertext.c_0;
    let mut temp = PowerPoly::from_crt(ctx, &temp).await;
    for coeff in temp.coefficients.iter_mut() {
        *coeff = <P::CiphertextParams as PolyParameters>::Residue::from_reduced(noise_max) - *coeff;
    }
    plaintext.clone_from_power(&temp);
}

impl<P> SecretKey<P>
where
    P: BgvParameters,
{
    pub async fn gen(ctx: &CrtContext<P::CiphertextParams>) -> Self {
        // TODO: Ensure hamming weight N/2 where N is `P::CiphertextParams::CYCLOTOMIC_DEGREE`.
        let e = sample_centered_binomial::<P::PlaintextParams>(1);
        let mut power_e = PowerPoly::new();
        power_e.clone_from_i64s(&e);
        let s = CrtPoly::from_power(ctx, &power_e).await;
        Self { s }
    }
}

impl<P> PublicKey<P>
where
    P: BgvParameters,
{
    pub async fn gen(ctx: &CrtContext<P::CiphertextParams>, sk: &SecretKey<P>) -> Self {
        type ExtendedUint<P> =
            <<<<P as BgvParameters>::PlaintextParams as PolyParameters>::Residue as GenericResidue>::Uint as ExtendableUint>::Extended;
        let a = CrtPoly::random(rand::thread_rng());
        let mut b = a.clone();
        b *= &sk.s;
        // We approximate the discrete gaussian distribution of variance 10 with
        // the centered binomial distribution of variance 10.  So the number of
        // iterations and the maximum magnitude is 20.
        const ITERATIONS: usize = 20;
        let e: Vec<ExtendedUint<P>> =
            add_centered_binomial_scaled(&PowerPoly::<P::PlaintextParams>::new(), ITERATIONS);
        b += &CrtPoly::from_power(ctx, &PowerPoly::from_signed_ints(&e)).await;
        Self { b, a }
    }
}

impl<P> Default for Ciphertext<P>
where
    P: BgvParameters,
{
    fn default() -> Self {
        Self {
            c_0: CrtPoly::new(),
            c_1: CrtPoly::new(),
        }
    }
}

impl<P> PreCiphertext<P>
where
    P: BgvParameters,
{
    pub async fn ciphertext(&self, ctx: &CrtContext<P::CiphertextParams>) -> Ciphertext<P> {
        let mut ciphertext = Ciphertext::default();
        self.ciphertext_into(ctx, &mut ciphertext).await;
        ciphertext
    }

    pub async fn ciphertext_into(
        &self,
        ctx: &CrtContext<P::CiphertextParams>,
        dst: &mut Ciphertext<P>,
    ) {
        dst.c_0.clone_from_power(ctx, &self.c_0).await;
        dst.c_1.clone_from_power(ctx, &self.c_1).await;
    }
}

impl<P> Default for PreCiphertext<P>
where
    P: BgvParameters,
{
    fn default() -> Self {
        Self {
            c_0: PowerPoly::new(),
            c_1: PowerPoly::new(),
        }
    }
}

pub const fn max_drown_bits<P>() -> usize
where
    P: BgvParameters,
{
    <P::CiphertextParams as PolyParameters>::Residue::BITS - P::PlaintextResidue::BITS - 1
}

#[cfg(test)]
mod tests {
    use crate::bgv::{
        decrypt, encrypt, encrypt_and_drown, max_drown_bits,
        params::ToyBgv,
        poly::{power::PowerPoly, CrtContext},
        Cleartext, PublicKey, SecretKey,
    };

    use super::poly::crt::CrtPoly;

    #[tokio::test]
    async fn serde_roundtrip_secret_key() {
        let ctx = CrtContext::gen().await;
        let sk = SecretKey::<ToyBgv>::gen(&ctx).await;
        let bytes = bincode::serialize(&sk).unwrap();
        let sk_roundtrip = bincode::deserialize(&bytes).unwrap();
        assert_eq!(sk, sk_roundtrip);
    }

    #[tokio::test]
    async fn serde_roundtrip_public_key() {
        let ctx = CrtContext::gen().await;
        let sk = SecretKey::<ToyBgv>::gen(&ctx).await;
        let pk = PublicKey::gen(&ctx, &sk).await;
        let bytes = bincode::serialize(&pk).unwrap();
        let pk_roundtrip = bincode::deserialize(&bytes).unwrap();
        assert_eq!(pk, pk_roundtrip);
    }

    #[tokio::test]
    async fn serde_roundtrip_ciphertext() {
        let mut rng = rand::thread_rng();
        let ctx = CrtContext::gen().await;
        let sk = SecretKey::<ToyBgv>::gen(&ctx).await;
        let pk = PublicKey::gen(&ctx, &sk).await;
        let plaintext = PowerPoly::random(&mut rng);
        let ciphertext = encrypt(&ctx, &pk, &plaintext).await;
        let bytes = bincode::serialize(&ciphertext).unwrap();
        let ciphertext_roundtrip = bincode::deserialize(&bytes).unwrap();
        assert_eq!(ciphertext, ciphertext_roundtrip);
    }

    #[tokio::test]
    async fn serde_roundtrip_prepared_plaintext() {
        let mut rng = rand::thread_rng();
        let ctx = CrtContext::gen().await;
        let power = PowerPoly::random(&mut rng);
        let prepared = Cleartext::<ToyBgv>::new(&ctx, &power).await;
        let bytes = bincode::serialize(&prepared).unwrap();
        let prepared_roundtrip = bincode::deserialize(&bytes).unwrap();
        assert_eq!(prepared, prepared_roundtrip);
    }

    #[tokio::test]
    async fn bgv_roundtrip() {
        let mut rng = rand::thread_rng();
        let ctx = CrtContext::gen().await;
        let sk = SecretKey::<ToyBgv>::gen(&ctx).await;
        let pk = PublicKey::gen(&ctx, &sk).await;
        let plaintext = PowerPoly::random(&mut rng);
        let ciphertext = encrypt(&ctx, &pk, &plaintext).await;
        let plaintext_roundtrip = decrypt(&ctx, &sk, &ciphertext).await;
        assert_eq!(plaintext, plaintext_roundtrip);
    }

    #[tokio::test]
    async fn homomorphic_add() {
        let mut rng = rand::thread_rng();
        let ctx_ct = CrtContext::gen().await;
        let ctx_pt = CrtContext::gen().await;
        let sk = SecretKey::<ToyBgv>::gen(&ctx_ct).await;
        let pk = PublicKey::gen(&ctx_ct, &sk).await;
        let lhs = CrtPoly::random(&mut rng);
        let rhs = CrtPoly::random(&mut rng);
        let lhs_ciphertext = encrypt(&ctx_ct, &pk, &PowerPoly::from_crt(&ctx_pt, &lhs).await).await;
        let rhs_ciphertext = encrypt(&ctx_ct, &pk, &PowerPoly::from_crt(&ctx_pt, &rhs).await).await;
        let result_ciphertext = {
            let mut ct = lhs_ciphertext;
            ct += &rhs_ciphertext;
            ct
        };
        let plaintext = decrypt(&ctx_ct, &sk, &result_ciphertext).await;
        let result = CrtPoly::from_power(&ctx_pt, &plaintext).await;
        let correct_result = {
            let mut pt = lhs;
            pt += &rhs;
            pt
        };
        assert_eq!(result, correct_result);
    }

    #[tokio::test]
    async fn homomorphic_add_plain() {
        let mut rng = rand::thread_rng();
        let ctx_ct = CrtContext::gen().await;
        let ctx_pt = CrtContext::gen().await;
        let sk = SecretKey::<ToyBgv>::gen(&ctx_ct).await;
        let pk = PublicKey::gen(&ctx_ct, &sk).await;
        let lhs = CrtPoly::random(&mut rng);
        let rhs = CrtPoly::random(&mut rng);
        let lhs_ciphertext = encrypt(&ctx_ct, &pk, &PowerPoly::from_crt(&ctx_pt, &lhs).await).await;
        let result_ciphertext = {
            let mut ct = lhs_ciphertext;
            let rhs_power = PowerPoly::from_crt(&ctx_pt, &rhs).await;
            ct += &Cleartext::new(&ctx_ct, &rhs_power).await;
            ct
        };
        let plaintext = decrypt(&ctx_ct, &sk, &result_ciphertext).await;
        let result = CrtPoly::from_power(&ctx_pt, &plaintext).await;
        let correct_result = {
            let mut pt = lhs;
            pt += &rhs;
            pt
        };
        assert_eq!(result, correct_result);
    }

    #[tokio::test]
    async fn homomorphic_sub() {
        let mut rng = rand::thread_rng();
        let ctx_ct = CrtContext::gen().await;
        let ctx_pt = CrtContext::gen().await;
        let sk = SecretKey::<ToyBgv>::gen(&ctx_ct).await;
        let pk = PublicKey::gen(&ctx_ct, &sk).await;
        let lhs = CrtPoly::random(&mut rng);
        let rhs = CrtPoly::random(&mut rng);
        let lhs_ciphertext = encrypt(&ctx_ct, &pk, &PowerPoly::from_crt(&ctx_pt, &lhs).await).await;
        let rhs_ciphertext = encrypt(&ctx_ct, &pk, &PowerPoly::from_crt(&ctx_pt, &rhs).await).await;
        let result_ciphertext = {
            let mut ct = lhs_ciphertext;
            ct -= &rhs_ciphertext;
            ct
        };
        let plaintext = decrypt(&ctx_ct, &sk, &result_ciphertext).await;
        let result = CrtPoly::from_power(&ctx_pt, &plaintext).await;
        let correct_result = {
            let mut pt = lhs;
            pt -= &rhs;
            pt
        };
        assert_eq!(result, correct_result);
    }

    #[tokio::test]
    async fn homomorphic_sub_plain() {
        let mut rng = rand::thread_rng();
        let ctx_ct = CrtContext::gen().await;
        let ctx_pt = CrtContext::gen().await;
        let sk = SecretKey::<ToyBgv>::gen(&ctx_ct).await;
        let pk = PublicKey::gen(&ctx_ct, &sk).await;
        let lhs = CrtPoly::random(&mut rng);
        let rhs = CrtPoly::random(&mut rng);
        let lhs_ciphertext = encrypt(&ctx_ct, &pk, &PowerPoly::from_crt(&ctx_pt, &lhs).await).await;
        let result_ciphertext = {
            let mut ct = lhs_ciphertext;
            let rhs_power = PowerPoly::from_crt(&ctx_pt, &rhs).await;
            ct -= &Cleartext::new(&ctx_ct, &rhs_power).await;
            ct
        };
        let plaintext = decrypt(&ctx_ct, &sk, &result_ciphertext).await;
        let result = CrtPoly::from_power(&ctx_pt, &plaintext).await;
        let correct_result = {
            let mut pt = lhs;
            pt -= &rhs;
            pt
        };
        assert_eq!(result, correct_result);
    }

    #[tokio::test]
    async fn homomorphic_mul_plain() {
        let mut rng = rand::thread_rng();
        let ctx_ct = CrtContext::gen().await;
        let ctx_pt = CrtContext::gen().await;
        let sk = SecretKey::<ToyBgv>::gen(&ctx_ct).await;
        let pk = PublicKey::gen(&ctx_ct, &sk).await;
        let lhs = CrtPoly::random(&mut rng);
        let rhs = CrtPoly::random(&mut rng);
        let lhs_ciphertext = encrypt(&ctx_ct, &pk, &PowerPoly::from_crt(&ctx_pt, &lhs).await).await;
        let result_ciphertext = {
            let mut ct = lhs_ciphertext;
            let rhs_power = PowerPoly::from_crt(&ctx_pt, &rhs).await;
            ct *= &Cleartext::new(&ctx_ct, &rhs_power).await;
            ct
        };
        let plaintext = decrypt(&ctx_ct, &sk, &result_ciphertext).await;
        let result = CrtPoly::from_power(&ctx_pt, &plaintext).await;
        let correct_result = {
            let mut pt = lhs;
            pt *= (&rhs, &ctx_pt);
            pt
        };
        assert_eq!(result, correct_result);
    }

    #[tokio::test]
    async fn mask_and_drown() {
        let mut rng = rand::thread_rng();
        let ctx_ct = CrtContext::gen().await;
        let ctx_pt = CrtContext::gen().await;
        let sk = SecretKey::<ToyBgv>::gen(&ctx_ct).await;
        let pk = PublicKey::gen(&ctx_ct, &sk).await;
        let lhs = CrtPoly::random(&mut rng);
        let rhs = CrtPoly::random(&mut rng);
        let mask = CrtPoly::random(&mut rng);
        let lhs_ciphertext = encrypt(&ctx_ct, &pk, &PowerPoly::from_crt(&ctx_pt, &lhs).await).await;
        // 1 more is ok most of the time, so we use it in tests.
        let noise_bits = max_drown_bits::<ToyBgv>() + 1;
        let mask_ciphertext = encrypt_and_drown(
            &ctx_ct,
            &pk,
            &PowerPoly::from_crt(&ctx_pt, &mask).await,
            noise_bits,
        )
        .await;
        let result_ciphertext = {
            let mut ct = lhs_ciphertext;
            let rhs_power = PowerPoly::from_crt(&ctx_pt, &rhs).await;
            ct *= &Cleartext::new(&ctx_ct, &rhs_power).await;
            ct -= &mask_ciphertext;
            ct
        };
        let plaintext = decrypt(&ctx_ct, &sk, &result_ciphertext).await;
        let actual = CrtPoly::from_power(&ctx_pt, &plaintext).await;
        let expected = {
            let mut pt = lhs;
            pt *= (&rhs, &ctx_pt);
            pt -= &mask;
            pt
        };
        assert_eq!(actual, expected);
    }
}
