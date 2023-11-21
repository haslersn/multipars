use std::ops::{AddAssign, MulAssign, SubAssign};

use crypto_bigint::{Random, Zero};
use forward_ref_generic::forward_ref_op_assign;
use rand::{CryptoRng, RngCore};
use serde::{Deserialize, Serialize};

use crate::bgv::{
    fourier::fast_fourier_transform,
    generic_uint::GenericUint,
    residue::{vec::GenericResidueVec, GenericResidue},
};

use super::{
    crt::{CrtPoly, CrtPolyParameters},
    CrtContext, Diagonal, FactorsContext, FourierContext, PolyParameters,
};

/// An element of the cyclotomic ring of integers `\mathbb{Z}[X]/\Phi_m(X)` in power basis (i.e. in
/// coefficient embedding).
#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct PowerPoly<P>
where
    P: PolyParameters,
{
    /// Vector of coefficients.
    pub coefficients: P::Vec,
}

impl<P> PowerPoly<P>
where
    P: PolyParameters,
{
    pub fn new() -> Self {
        let coefficients = P::Vec::new(P::CYCLOTOMIC_DEGREE);
        Self { coefficients }
    }

    pub fn clone_from_signed_ints<SourceInt>(&mut self, source: &[SourceInt])
    where
        SourceInt: GenericUint,
    {
        debug_assert!(source.len() == P::CYCLOTOMIC_DEGREE);
        for (dst, src) in self.coefficients.iter_mut().zip(source) {
            *dst = GenericResidue::from_signed_int(*src);
        }
    }

    pub fn from_signed_ints<SourceInt>(source: &[SourceInt]) -> Self
    where
        SourceInt: GenericUint,
    {
        let mut this = Self::new();
        this.clone_from_signed_ints(source);
        this
    }

    pub fn clone_from_i64s(&mut self, source: &[i64]) {
        debug_assert!(source.len() == P::CYCLOTOMIC_DEGREE);
        for (dst, src) in self.coefficients.iter_mut().zip(source) {
            *dst = GenericResidue::from_i64(*src);
        }
    }

    pub fn from_i64s<SourceInt>(source: &[i64]) -> Self
    where
        SourceInt: GenericUint,
    {
        let mut this = Self::new();
        this.clone_from_i64s(source);
        this
    }

    pub async fn clone_from_crt(&mut self, ctx: &CrtContext<P>, crt: &CrtPoly<P>)
    where
        P: CrtPolyParameters,
    {
        match ctx {
            CrtContext::Factors(ctx) => self.clone_from_crt_via_factors(ctx, crt),
            CrtContext::Fourier(ctx) => self.clone_from_crt_via_fourier(ctx, crt).await,
        }
    }

    fn clone_from_crt_via_factors(&mut self, ctx: &FactorsContext<P>, crt: &CrtPoly<P>)
    where
        P: CrtPolyParameters,
    {
        for c in self.coefficients.iter_mut() {
            *c = Zero::ZERO;
        }

        let mut intermediate = vec![P::Residue::ZERO; P::CYCLOTOMIC_DEGREE];

        for factor_index in 0..P::FACTOR_COUNT {
            for basis_index in 0..P::FACTOR_COUNT {
                for factor_exp in 0..P::FACTOR_DEGREE {
                    let coeff = crt.coefficients[factor_index * P::FACTOR_DEGREE + factor_exp];
                    let index = (factor_index + basis_index) % P::FACTOR_COUNT;
                    let summand = ctx.basis_coefficients[index] * coeff;
                    intermediate[basis_index * P::FACTOR_DEGREE + factor_exp] += summand;
                }
            }
        }

        let mut last_coeff = Zero::ZERO;
        let mut basis_exp_repr = 1;
        for basis_index in 0..P::FACTOR_COUNT {
            for factor_exp in 0..P::FACTOR_DEGREE {
                let slot = intermediate[basis_index * P::FACTOR_DEGREE + factor_exp];
                let mut basis_exp = basis_exp_repr;
                for _ in 0..P::FACTOR_DEGREE {
                    let exp = (factor_exp + basis_exp) % P::M;
                    if exp == P::CYCLOTOMIC_DEGREE {
                        last_coeff += slot;
                    } else {
                        self.coefficients[exp] += slot;
                    }
                    basis_exp *= 2; // TODO: Support arbitrary prime powers and not just `2^k`.
                    basis_exp %= P::M;
                }
            }
            basis_exp_repr *= P::SLOT_GENERATOR;
            basis_exp_repr %= P::M;
        }

        let first_coeff = self.coefficients[0];
        self.coefficients[0] = last_coeff;
        for c in self.coefficients.iter_mut() {
            *c -= first_coeff;
        }
    }

    async fn clone_from_crt_via_fourier(&mut self, ctx: &FourierContext<P>, crt: &CrtPoly<P>)
    where
        P: CrtPolyParameters,
    {
        for c in self.coefficients.iter_mut() {
            *c = Zero::ZERO;
        }

        let mut padded = P::Vec::new(ctx.dft_size);
        for (dst, src) in padded.iter_mut().zip(crt.coefficients.iter()) {
            *dst = *src; // TODO: use vectorized copy
        }

        let mut padded_fft = fast_fourier_transform(&ctx.dft_root_powers, false, padded).await;

        for (dst, src) in padded_fft.iter_mut().zip(ctx.kernel_from_crt.iter()) {
            *dst *= *src; // TODO: use vectorized copy
        }
        let padded = fast_fourier_transform(&ctx.dft_root_powers, true, padded_fft).await;

        let mut exp = 1;
        for entry in padded.iter().take(P::CYCLOTOMIC_DEGREE) {
            if exp == P::CYCLOTOMIC_DEGREE {
                self.coefficients[0] = *entry;
            } else {
                self.coefficients[exp] = *entry;
            }
            exp *= P::SLOT_GENERATOR_INVERSE;
            exp %= P::M;
        }
        for entry in padded
            .iter()
            .skip(P::CYCLOTOMIC_DEGREE)
            .take(P::CYCLOTOMIC_DEGREE - 1)
        {
            if exp == P::CYCLOTOMIC_DEGREE {
                self.coefficients[0] += *entry; // TODO: Can we support references on the RHS, too?
            } else {
                self.coefficients[exp] += *entry; // TODO: Can we support references on the RHS, too?
            }
            exp *= P::SLOT_GENERATOR_INVERSE;
            exp %= P::M;
        }
    }

    pub async fn from_crt(ctx: &CrtContext<P>, crt: &CrtPoly<P>) -> Self
    where
        P: CrtPolyParameters,
    {
        let mut this = Self::new();
        this.clone_from_crt(ctx, crt).await;
        this
    }

    pub fn clone_from_power<P2>(&mut self, other: &PowerPoly<P2>)
    where
        P2: PolyParameters,
    {
        debug_assert!(P::M == P2::M);
        for (dst, src) in self.coefficients.iter_mut().zip(other.coefficients.iter()) {
            *dst = GenericResidue::from_unsigned(*src);
        }
    }

    pub fn from_power<P2>(other: &PowerPoly<P2>) -> Self
    where
        P2: PolyParameters,
    {
        let mut this = Self::new();
        this.clone_from_power(other);
        this
    }

    pub fn random(mut rng: impl CryptoRng + RngCore) -> Self {
        let mut this = Self::new();
        for coeff in this.coefficients.iter_mut() {
            *coeff = Random::random(&mut rng);
        }
        this
    }

    pub fn add_assign_rotated(&mut self, rhs: &Self, rotate_right: usize) {
        for (i, rhs_coeff) in rhs.coefficients.iter().enumerate() {
            let rhs_power = if i == 0 { P::M - 1 } else { i };
            let lhs_power = (rhs_power + rotate_right) % P::M;
            if lhs_power == 0 {
                for coeff in self.coefficients.iter_mut() {
                    *coeff -= *rhs_coeff; // TODO: Can we support references on the RHS, too?
                }
            } else {
                let lhs_index = lhs_power % (P::M - 1);
                self.coefficients[lhs_index] += *rhs_coeff; // TODO: Can we support references on the RHS, too?
            }
        }
    }

    pub fn sub_assign_rotated(&mut self, rhs: &Self, rotate_right: usize) {
        for (i, rhs_coeff) in rhs.coefficients.iter().enumerate() {
            let rhs_power = if i == 0 { P::M - 1 } else { i };
            let lhs_power = (rhs_power + rotate_right) % P::M;
            if lhs_power == 0 {
                for coeff in self.coefficients.iter_mut() {
                    *coeff += *rhs_coeff; // TODO: Can we support references on the RHS, too?
                }
            } else {
                let lhs_index = lhs_power % (P::M - 1);
                self.coefficients[lhs_index] -= *rhs_coeff; // TODO: Can we support references on the RHS, too?
            }
        }
    }

    pub fn add_assign_slided(&mut self, rhs: &Self, length: usize) {
        if length == 0 {
            return;
        }
        let mut sum = Zero::ZERO;
        for power in 1..P::M {
            sum += rhs.coefficients[power % (P::M - 1)];
            if power != length {
                sum -= rhs.coefficients[(power + P::M - length) % P::M % (P::M - 1)];
            }
            self.coefficients[power % (P::M - 1)] += sum;
        }
    }
}

impl<P> Clone for PowerPoly<P>
where
    P: PolyParameters,
{
    fn clone(&self) -> Self {
        Self {
            coefficients: self.coefficients.clone(),
        }
    }

    fn clone_from(&mut self, source: &Self) {
        self.coefficients.clone_from(&source.coefficients);
    }
}

impl<P> AddAssign<&Self> for PowerPoly<P>
where
    P: PolyParameters,
{
    fn add_assign(&mut self, rhs: &Self) {
        for (dst, src) in self.coefficients.iter_mut().zip(rhs.coefficients.iter()) {
            *dst += *src; // TODO: Can we support references on the RHS, too?
        }
    }
}

impl<P> SubAssign<&Self> for PowerPoly<P>
where
    P: PolyParameters,
{
    fn sub_assign(&mut self, rhs: &Self) {
        for (dst, src) in self.coefficients.iter_mut().zip(rhs.coefficients.iter()) {
            *dst -= *src; // TODO: Can we support references on the RHS, too?
        }
    }
}

impl<P> MulAssign<Diagonal<P::Residue>> for PowerPoly<P>
where
    P: PolyParameters,
{
    fn mul_assign(&mut self, rhs: Diagonal<P::Residue>) {
        for dst in self.coefficients.iter_mut() {
            *dst *= rhs.0;
        }
    }
}

forward_ref_op_assign!(
    [ P ]
    impl MulAssign, mul_assign for PowerPoly<P>, Diagonal<P::Residue>
    where P: PolyParameters
);

#[cfg(test)]
mod tests {
    use rand::Rng;
    use serde::{Deserialize, Serialize};

    use crate::bgv::{
        params::{ToyCipher, ToyPlain},
        poly::{power::PowerPoly, PolyParameters},
    };

    #[test]
    fn ciphertext_serde_roundtrip_power_poly() {
        serde_roundtrip_power_poly::<ToyCipher>();
    }

    #[test]
    fn plaintext_serde_roundtrip_power_poly() {
        serde_roundtrip_power_poly::<ToyPlain>();
    }

    fn serde_roundtrip_power_poly<P>()
    where
        P: PolyParameters,
        PowerPoly<P>: Serialize,
        for<'a> PowerPoly<P>: Deserialize<'a>,
    {
        let mut rng = rand::thread_rng();
        let power = PowerPoly::<P>::random(&mut rng);
        let bytes = bincode::serialize(&power).unwrap();
        let power_roundtrip = bincode::deserialize(&bytes).unwrap();
        assert_eq!(power, power_roundtrip);
    }

    #[test]
    fn ciphertext_add_assign_slided() {
        add_assign_slided::<ToyCipher>();
    }

    #[test]
    fn plaintext_add_assign_slided() {
        add_assign_slided::<ToyPlain>();
    }

    fn add_assign_slided<P>()
    where
        P: PolyParameters,
    {
        let mut rng = rand::thread_rng();
        let mut actual = PowerPoly::<P>::random(&mut rng);
        let mut expected = actual.clone();
        let rhs = PowerPoly::random(&mut rng);
        let length = rng.gen_range(0..P::M);

        // Compute actual
        actual.add_assign_slided(&rhs, length);

        // Compute expected
        for i in 0..length {
            expected.add_assign_rotated(&rhs, i);
        }

        assert_eq!(actual, expected);
    }
}
