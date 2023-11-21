use std::ops::{AddAssign, MulAssign, SubAssign};

use crypto_bigint::{Random, Zero};
use forward_ref_generic::forward_ref_op_assign;
use rand::{CryptoRng, RngCore};
use serde::{Deserialize, Serialize};

use crate::bgv::{fourier::fast_fourier_transform, residue::vec::GenericResidueVec};

use super::{
    power::PowerPoly, CrtContext, CrtStrategy, Diagonal, FactorsContext, FourierContext,
    FourierCrtPolyParameters, PolyParameters,
};

pub trait CrtPolyParameters: PolyParameters {
    const FACTOR_COUNT: usize;
    const FACTOR_DEGREE: usize;
    const SLOT_GENERATOR: usize;
    const SLOT_GENERATOR_INVERSE: usize;
    const CRT_STRATEGY: CrtStrategy;
    const GENERATOR: Self::Residue;
}

/// An element of `R_q = \mathbb{Z}_q[X]/\Phi_M(X)` in CRT basis.
#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct CrtPoly<P>
where
    P: CrtPolyParameters,
{
    pub coefficients: P::Vec, // TODO: Non-public.
}

impl<P> CrtPoly<P>
where
    P: CrtPolyParameters,
{
    pub fn new() -> Self {
        let coefficients = P::Vec::new(P::CYCLOTOMIC_DEGREE);
        Self { coefficients }
    }

    pub fn assign_zero(&mut self) {
        for coeff in self.coefficients.iter_mut() {
            *coeff = Zero::ZERO;
        }
    }

    pub async fn clone_from_power(&mut self, ctx: &CrtContext<P>, power: &PowerPoly<P>) {
        match ctx {
            CrtContext::Factors(ctx) => self.clone_from_power_via_factors(ctx, power).await,
            CrtContext::Fourier(ctx) => self.clone_from_power_via_fourier(ctx, power).await,
        }
    }

    async fn clone_from_power_via_factors(
        &mut self,
        ctx: &FactorsContext<P>,
        power: &PowerPoly<P>,
    ) {
        for factor_index in 0..P::FACTOR_COUNT {
            let mut reduced = Vec::new();
            reduced.reserve(P::M);
            reduced.extend(power.coefficients.iter());
            reduced.push(reduced[0]);
            reduced[0] = Zero::ZERO;
            for leading_exp in (P::FACTOR_DEGREE..P::M).rev() {
                let leading = reduced[leading_exp];
                for exp in 0..P::FACTOR_DEGREE {
                    let offset = leading * ctx.factors[factor_index * (P::FACTOR_DEGREE + 1) + exp];
                    reduced[leading_exp - P::FACTOR_DEGREE + exp] -= offset;
                }
            }
            for exp in 0..P::FACTOR_DEGREE {
                self.coefficients[factor_index * P::FACTOR_DEGREE + exp] = reduced[exp];
            }
            tokio::task::yield_now().await;
        }
    }

    async fn clone_from_power_via_fourier(
        &mut self,
        ctx: &FourierContext<P>,
        power: &PowerPoly<P>,
    ) {
        for c in self.coefficients.iter_mut() {
            *c = Zero::ZERO;
        }

        let mut padded = P::Vec::new(ctx.dft_size);
        let mut exp = 1;
        for entry in padded.iter_mut().take(P::CYCLOTOMIC_DEGREE) {
            if exp != P::CYCLOTOMIC_DEGREE {
                *entry = power.coefficients[exp];
            } else {
                *entry = power.coefficients[0];
            }
            exp *= P::SLOT_GENERATOR_INVERSE;
            exp %= P::M;
        }

        let mut padded_fft = fast_fourier_transform(&ctx.dft_root_powers, false, padded).await;

        for (dst, src) in padded_fft.iter_mut().zip(ctx.kernel_from_power.iter()) {
            *dst *= *src; // TODO: use vectorized copy
        }
        let padded = fast_fourier_transform(&ctx.dft_root_powers, true, padded_fft).await;

        for (dst, src) in self.coefficients.iter_mut().zip(padded.iter()) {
            *dst = *src; // TODO: Use vectorized copy
        }
        for (dst, src) in self.coefficients.iter_mut().zip(
            padded
                .iter()
                .skip(P::CYCLOTOMIC_DEGREE)
                .take(P::CYCLOTOMIC_DEGREE - 1),
        ) {
            *dst += *src; // TODO: Use vectorized copy
        }
    }

    pub async fn from_power(ctx: &CrtContext<P>, power: &PowerPoly<P>) -> Self {
        let mut this = Self::new();
        this.clone_from_power(ctx, power).await;
        this
    }

    pub fn random(mut rng: impl CryptoRng + RngCore) -> Self {
        let mut this = Self::new();
        for coeff in this.coefficients.iter_mut() {
            *coeff = Random::random(&mut rng);
        }
        this
    }
}

impl<P> Clone for CrtPoly<P>
where
    P: CrtPolyParameters,
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

impl<P> AddAssign<&Self> for CrtPoly<P>
where
    P: CrtPolyParameters,
{
    fn add_assign(&mut self, rhs: &Self) {
        for (dst, src) in self.coefficients.iter_mut().zip(rhs.coefficients.iter()) {
            *dst += *src; // TODO: Can we support references on the RHS, too?
        }
    }
}

impl<P> AddAssign<Diagonal<P::Residue>> for CrtPoly<P>
where
    P: CrtPolyParameters,
{
    fn add_assign(&mut self, rhs: Diagonal<P::Residue>) {
        for coeff in self.coefficients.iter_mut().step_by(P::FACTOR_DEGREE) {
            *coeff += rhs.0;
        }
    }
}

forward_ref_op_assign!(
    [ P ]
    impl AddAssign, add_assign for CrtPoly<P>, Diagonal<P::Residue>
    where P: CrtPolyParameters
);

impl<P> SubAssign<&Self> for CrtPoly<P>
where
    P: CrtPolyParameters,
{
    fn sub_assign(&mut self, rhs: &Self) {
        for (dst, src) in self.coefficients.iter_mut().zip(rhs.coefficients.iter()) {
            *dst -= *src; // TODO: Can we support references on the RHS, too?
        }
    }
}

impl<P> SubAssign<Diagonal<P::Residue>> for CrtPoly<P>
where
    P: CrtPolyParameters,
{
    fn sub_assign(&mut self, rhs: Diagonal<P::Residue>) {
        for coeff in self.coefficients.iter_mut().step_by(P::FACTOR_DEGREE) {
            *coeff -= rhs.0;
        }
    }
}

forward_ref_op_assign!(
    [ P ]
    impl SubAssign, sub_assign for CrtPoly<P>, Diagonal<P::Residue>
    where P: CrtPolyParameters
);

impl<P> MulAssign<(&Self, &CrtContext<P>)> for CrtPoly<P>
where
    P: CrtPolyParameters,
{
    fn mul_assign(&mut self, args: (&Self, &CrtContext<P>)) {
        let rhs = args.0;
        let ctx = args.1;

        if let CrtContext::Factors(ctx) = ctx {
            // While computing the result for a slot, `temp` stores the intermediate results.
            let mut temp = vec![Zero::ZERO; P::FACTOR_DEGREE];

            // We proceed slot after slot, so we can reuse the `temp` vector used as scratch space.
            for factor_index in 0..P::FACTOR_COUNT {
                for j in (0..P::FACTOR_DEGREE).rev() {
                    let rhs_coeff = rhs.coefficients[factor_index * P::FACTOR_DEGREE + j];
                    for i in 0..P::FACTOR_DEGREE {
                        let lhs_coeff = self.coefficients[factor_index * P::FACTOR_DEGREE + i];
                        let prod = lhs_coeff * rhs_coeff;
                        if j == P::FACTOR_DEGREE - 1 {
                            temp[i] = prod;
                        } else {
                            temp[i] += prod;
                        }
                    }
                    if j != 0 {
                        // Multiply the intermediate result by X (via shift by 1 index) and then
                        // reduce modulo the factor of this slot.
                        let leading = temp[P::FACTOR_DEGREE - 1];
                        for i in (0..P::FACTOR_DEGREE).rev() {
                            let offset =
                                leading * ctx.factors[factor_index * (P::FACTOR_DEGREE + 1) + i];
                            let shifted = if i != 0 { temp[i - 1] } else { Zero::ZERO };
                            temp[i] = shifted - offset;
                        }
                    } else {
                        for i in 0..P::FACTOR_DEGREE {
                            self.coefficients[factor_index * P::FACTOR_DEGREE + i] = temp[i];
                        }
                    }
                }
            }
        } else {
            for (dst, src) in self.coefficients.iter_mut().zip(rhs.coefficients.iter()) {
                *dst *= *src;
            }
        }
    }
}

impl<P> MulAssign<&Self> for CrtPoly<P>
where
    P: FourierCrtPolyParameters,
{
    fn mul_assign(&mut self, rhs: &Self) {
        for (dst, src) in self.coefficients.iter_mut().zip(rhs.coefficients.iter()) {
            *dst *= *src;
        }
    }
}

impl<P> MulAssign<Diagonal<P::Residue>> for CrtPoly<P>
where
    P: CrtPolyParameters,
{
    fn mul_assign(&mut self, rhs: Diagonal<P::Residue>) {
        for dst in self.coefficients.iter_mut() {
            *dst *= rhs.0;
        }
    }
}

forward_ref_op_assign!(
    [ P ]
    impl MulAssign, mul_assign for CrtPoly<P>, Diagonal<P::Residue>
    where P: CrtPolyParameters
);

#[cfg(test)]
mod tests {
    use crypto_bigint::{Random, Zero};
    use rand::Rng;
    use serde::{Deserialize, Serialize};

    use crate::bgv::{
        params::{ToyCipher, ToyPlain},
        poly::{crt::CrtPoly, power::PowerPoly, CrtContext, PolyParameters},
    };

    use super::CrtPolyParameters;

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

    #[tokio::test]
    async fn plaintext_crt_poly_mul() {
        crt_poly_mul::<ToyPlain>().await;
    }

    #[tokio::test]
    async fn ciphertext_crt_poly_mul() {
        crt_poly_mul::<ToyCipher>().await;
    }

    async fn crt_poly_mul<P>()
    where
        P: CrtPolyParameters,
    {
        let mut rng = rand::thread_rng();
        let lhs = CrtPoly::<P>::random(&mut rng);
        let rhs = CrtPoly::<P>::random(&mut rng);
        let ctx = CrtContext::gen().await;

        let result = {
            let mut temp = lhs.clone();
            temp *= (&rhs, &ctx);
            temp
        };

        let correct_result = {
            let lhs = PowerPoly::from_crt(&ctx, &lhs).await;
            let rhs = PowerPoly::from_crt(&ctx, &rhs).await;
            let mut temp = PowerPoly::<P>::new();
            let mut first_coeff = Zero::ZERO;
            for j in 0..P::M {
                let rhs_coeff = if j == 0 {
                    Zero::ZERO
                } else {
                    rhs.coefficients[j % (P::M - 1)]
                };
                for i in 0..P::M {
                    let lhs_coeff = if i == 0 {
                        Zero::ZERO
                    } else {
                        lhs.coefficients[i % (P::M - 1)]
                    };
                    let prod = lhs_coeff * rhs_coeff;
                    let k = (i + j) % P::M;
                    if k == 0 {
                        first_coeff += prod;
                    } else {
                        temp.coefficients[k % (P::M - 1)] += prod;
                    }
                }
            }
            for k in 0..P::M - 1 {
                temp.coefficients[k] -= first_coeff;
            }
            CrtPoly::from_power(&ctx, &temp).await
        };

        assert_eq!(result, correct_result);
    }

    #[tokio::test]
    async fn plaintext_crt_poly_mul_commutative() {
        crt_poly_mul_commutative::<ToyPlain>().await;
    }

    #[tokio::test]
    async fn ciphertext_crt_poly_mul_commutative() {
        crt_poly_mul_commutative::<ToyCipher>().await;
    }

    async fn crt_poly_mul_commutative<P>()
    where
        P: CrtPolyParameters,
    {
        let ctx = CrtContext::gen().await;
        let mut rng = rand::thread_rng();
        let lhs = CrtPoly::<P>::random(&mut rng);
        let rhs = CrtPoly::<P>::random(&mut rng);
        let lhs_result = {
            let mut temp = lhs.clone();
            temp *= (&rhs, &ctx);
            temp
        };
        let rhs_result = {
            let mut temp = rhs.clone();
            temp *= (&lhs, &ctx);
            temp
        };
        assert_eq!(lhs_result, rhs_result);
    }

    #[tokio::test]
    async fn plaintext_crt_poly_mul_rotate() {
        crt_poly_mul_rotate::<ToyPlain>().await;
    }

    #[tokio::test]
    async fn ciphertext_crt_poly_mul_rotate() {
        crt_poly_mul_rotate::<ToyCipher>().await;
    }

    async fn crt_poly_mul_rotate<P>()
    where
        P: CrtPolyParameters,
    {
        let mut rng = rand::thread_rng();
        let ctx = CrtContext::gen().await;

        let lhs = PowerPoly::<P>::random(&mut rng);

        let rhs_index = rng.gen_range(1..P::M - 1);
        let rhs_val = Random::random(&mut rng);
        let rhs = {
            let mut rhs = PowerPoly::<P>::new();
            rhs.coefficients[rhs_index] = rhs_val;
            rhs
        };

        let result = {
            let mut crt = CrtPoly::from_power(&ctx, &lhs).await;
            crt *= (&CrtPoly::from_power(&ctx, &rhs).await, &ctx);
            PowerPoly::from_crt(&ctx, &crt).await
        };

        let correct_result = {
            let mut temp = PowerPoly::<P>::new();
            let last_coeff = lhs.coefficients[P::M - rhs_index];
            for i in 1..P::M {
                let src_i = (i + P::M - rhs_index) % P::M;
                let src = if src_i == 0 {
                    Zero::ZERO
                } else {
                    lhs.coefficients[src_i % (P::M - 1)]
                };
                temp.coefficients[i % (P::M - 1)] = (src - last_coeff) * rhs_val;
            }
            temp
        };

        assert_eq!(result, correct_result);
    }
}
