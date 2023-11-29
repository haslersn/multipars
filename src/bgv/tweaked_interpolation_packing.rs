use crypto_bigint::{Zero, U64};
use rand::{CryptoRng, RngCore};

use crate::bgv::{poly::PolyParameters, residue::GenericResidue};

use super::{
    poly::crt::{CrtPoly, CrtPolyParameters},
    residue::{native::GenericNativeResidue, vec::GenericResidueVec},
};
pub trait TIPParameters: CrtPolyParameters
where
    Self::Residue: GenericNativeResidue,
{
    const DELTA: u32;
}

pub const fn packing_capacity<P>() -> usize
where
    P: CrtPolyParameters,
{
    P::FACTOR_COUNT * packing_capacity_per_slot::<P>()
}

pub const fn packing_capacity_per_slot<P>() -> usize
where
    P: CrtPolyParameters,
{
    (P::FACTOR_DEGREE + 1) / 2
}

pub fn get_random_unpacked<P, T>(mut rng: impl CryptoRng + RngCore) -> Vec<T>
where
    P: TIPParameters,
    P::Residue: GenericNativeResidue,
    T: GenericNativeResidue,
{
    (0..packing_capacity::<P>())
        .map(|_| T::random(&mut rng))
        .collect()
}

pub fn pack<P>(unpacked: &[impl GenericNativeResidue]) -> CrtPoly<P>
where
    P: TIPParameters,
    P::Residue: GenericNativeResidue,
{
    assert!(unpacked.len() <= packing_capacity::<P>());

    // TODO: Precompute
    let mut lagrange_polys =
        vec![<P as PolyParameters>::Vec::new(P::FACTOR_DEGREE); packing_capacity_per_slot::<P>()];
    for (j, lp) in lagrange_polys.iter_mut().enumerate() {
        lp[0] = GenericResidue::from_uint(U64::ONE);
        let mut trailing_zeros = 0u32;
        let mut denom = 1i64;

        for i in 0..packing_capacity_per_slot::<P>() {
            if i != j {
                let i_res = <P as PolyParameters>::Residue::from_uint(U64::from_u64(i as u64));
                denom *= j as i64 - i as i64;
                trailing_zeros += denom.trailing_zeros();
                denom >>= denom.trailing_zeros();
                // Compute lp *= (X - i)
                for k in (1..P::FACTOR_DEGREE).rev() {
                    lp[k] = lp[k - 1] - i_res * lp[k];
                }
                lp[0] = <P as PolyParameters>::Residue::ZERO - (i_res * lp[0]);
            }
        }

        assert!(trailing_zeros <= P::DELTA);

        // Compute factor := 2^delta / denom
        let denom = <P as PolyParameters>::Residue::from_i64(denom);
        let factor = denom
            .invert()
            .0
            .shl_vartime((P::DELTA - trailing_zeros) as usize);

        // Compute lp *= factor
        for entry in lp.iter_mut() {
            *entry *= factor;
        }
    }

    let mut result = CrtPoly::<P>::new();

    for (factor_index, chunk) in unpacked
        .chunks(packing_capacity_per_slot::<P>())
        .enumerate()
    {
        let slot_begin = factor_index * P::FACTOR_DEGREE;
        for (entry, lp) in chunk.iter().zip(lagrange_polys.iter()) {
            for i in 0..P::FACTOR_DEGREE {
                let extended: <P as PolyParameters>::Residue =
                    GenericResidue::from_unsigned(*entry);
                result.coefficients[slot_begin + i] += extended * lp[i];
            }
        }
    }

    // // Alternative implementation, TODO: check which one is more cache-friendly
    // for (chunk, lp) in unpacked
    //     .chunks(P::FACTOR_COUNT)
    //     .zip(lagrange_polys.iter())
    // {
    //     // Here we treat chunk as a CrtPoly where each slot is constant,
    //     // and we compute result += chunk * lp.
    //     for (factor_index, entry) in chunk.iter().enumerate() {
    //         let slot_begin = factor_index * P::FACTOR_DEGREE;
    //         for i in 0..P::FACTOR_DEGREE {
    //             let extended: <P as PolyParameters>::Residue =
    //                 GenericResidue::from_unsigned(*entry);
    //             result.coefficients[slot_begin + i] += extended * lp[i];
    //         }
    //     }
    // }

    result
}

pub fn pack_diagonal<P>(unpacked: impl GenericNativeResidue) -> CrtPoly<P>
where
    P: TIPParameters,
    P::Residue: GenericNativeResidue,
{
    let mut result = CrtPoly::<P>::new();
    let cc = <P as PolyParameters>::Residue::from_unsigned(unpacked).shl_vartime(P::DELTA as usize);
    for factor_index in 0..P::FACTOR_COUNT {
        result.coefficients[factor_index * P::FACTOR_DEGREE] = cc;
    }
    result
}

pub fn pack_mask<P>(unpacked: &[impl GenericNativeResidue]) -> CrtPoly<P>
where
    P: TIPParameters,
    P::Residue: GenericNativeResidue,
{
    let mut result = pack::<P>(unpacked);
    for coeff in result.coefficients.iter_mut() {
        *coeff = coeff.shl_vartime(P::DELTA as usize);
    }
    // TODO: Add fiber of 0 and mask upper bits
    result
}

pub fn unpack<P, T>(crt: &CrtPoly<P>) -> Option<Vec<T>>
where
    P: TIPParameters,
    P::Residue: GenericNativeResidue,
    T: GenericNativeResidue,
{
    // TODO: Precompute
    // powers[b][e] is a lookup table for b^e
    let mut powers =
        vec![<P as PolyParameters>::Vec::new(P::FACTOR_DEGREE); packing_capacity_per_slot::<P>()];
    for (b, b_powers) in powers.iter_mut().enumerate() {
        let base = GenericResidue::from_uint(U64::from_u64(b as u64));
        b_powers[0] = GenericResidue::from_uint(U64::ONE);
        b_powers[1] = base;
        let mut temp = base;
        for p in b_powers.iter_mut().skip(2) {
            temp *= base;
            *p = temp;
        }
    }

    let mut result = vec![T::ZERO; packing_capacity::<P>()];

    for (factor_index, chunk) in result
        .chunks_mut(packing_capacity_per_slot::<P>())
        .enumerate()
    {
        let slot_begin = factor_index * P::FACTOR_DEGREE;
        for (entry, b_powers) in chunk.iter_mut().zip(powers.iter()) {
            let mut evaluated = <P as PolyParameters>::Residue::ZERO;
            for i in 0..P::FACTOR_DEGREE {
                evaluated += crt.coefficients[slot_begin + i] * b_powers[i];
            }
            // TODO: Check that `evaluated` is divisible by 2^(2delta)
            *entry = GenericResidue::from_unsigned(evaluated.shr_vartime(2 * P::DELTA as usize));
        }
    }

    Some(result)
}

#[cfg(test)]
mod tests {
    use crypto_bigint::Random;

    use crate::{
        bgv::{
            poly::CrtContext,
            tweaked_interpolation_packing::{
                get_random_unpacked, pack, pack_diagonal, pack_mask, packing_capacity, unpack,
            },
        },
        low_gear_preproc::{
            params::{PreprocK128S64, PreprocK32S32, PreprocK64S64},
            PreprocessorParameters,
        },
    };

    #[tokio::test]
    async fn pack_mul_unpack_single_t96() {
        pack_mul_unpack_single::<PreprocK32S32>().await;
    }

    #[tokio::test]
    async fn pack_mul_unpack_single_t192() {
        pack_mul_unpack_single::<PreprocK64S64>().await;
    }

    #[tokio::test]
    async fn pack_mul_unpack_single_t256() {
        pack_mul_unpack_single::<PreprocK128S64>().await;
    }

    async fn pack_mul_unpack_single<P: PreprocessorParameters>() {
        let mut rng = rand::thread_rng();
        let ctx = CrtContext::gen().await;
        let a = [P::KSS::random(&mut rng)];
        let b = [P::KSS::random(&mut rng)];
        let packed_a = pack::<P::PlaintextParams>(&a);
        let packed_b = pack::<P::PlaintextParams>(&b);
        let mut packed_prod = packed_a;
        packed_prod *= (&packed_b, &ctx);
        let actual = unpack(&packed_prod).unwrap()[0];
        let expected = a[0] * b[0];
        assert_eq!(expected, actual);
    }

    #[tokio::test]
    async fn pack_mul_unpack_t96() {
        pack_mul_unpack::<PreprocK32S32>().await;
    }

    #[tokio::test]
    async fn pack_mul_unpack_t192() {
        pack_mul_unpack::<PreprocK64S64>().await;
    }

    #[tokio::test]
    async fn pack_mul_unpack_t256() {
        pack_mul_unpack::<PreprocK128S64>().await;
    }

    async fn pack_mul_unpack<P: PreprocessorParameters>() {
        let mut rng = rand::thread_rng();
        let ctx = CrtContext::gen().await;
        let a = get_random_unpacked::<P::PlaintextParams, P::KSS>(&mut rng);
        let b = get_random_unpacked::<P::PlaintextParams, P::KSS>(&mut rng);
        let packed_a = pack::<P::PlaintextParams>(&a);
        let packed_b = pack::<P::PlaintextParams>(&b);
        let mut packed_prod = packed_a;
        packed_prod *= (&packed_b, &ctx);
        let actual = unpack(&packed_prod).unwrap();
        let expected: Vec<_> = a.iter().zip(b.iter()).map(|(a, b)| *a * *b).collect();
        assert_eq!(expected, actual);
    }

    #[tokio::test]
    async fn pack_mul_mask_unpack_t96() {
        pack_mul_mask_unpack::<PreprocK32S32>().await;
    }

    #[tokio::test]
    async fn pack_mul_mask_unpack_t192() {
        pack_mul_mask_unpack::<PreprocK64S64>().await;
    }

    #[tokio::test]
    async fn pack_mul_mask_unpack_t256() {
        pack_mul_mask_unpack::<PreprocK128S64>().await;
    }

    async fn pack_mul_mask_unpack<P: PreprocessorParameters>() {
        let mut rng = rand::thread_rng();
        let ctx = CrtContext::gen().await;
        let a = get_random_unpacked::<P::PlaintextParams, P::KSS>(&mut rng);
        let b = get_random_unpacked::<P::PlaintextParams, P::KSS>(&mut rng);
        let e = get_random_unpacked::<P::PlaintextParams, P::KSS>(&mut rng);
        let packed_a = pack::<P::PlaintextParams>(&a);
        let packed_b = pack::<P::PlaintextParams>(&b);
        let packed_e = pack_mask(&e);
        let mut packed_prod = packed_a;
        packed_prod *= (&packed_b, &ctx);
        packed_prod += &packed_e;
        let actual = unpack(&packed_prod).unwrap();
        let expected: Vec<_> = a
            .iter()
            .zip(b.iter())
            .zip(e.iter())
            .map(|((a, b), e)| *a * *b + *e)
            .collect();
        assert_eq!(expected, actual);
    }

    #[tokio::test]
    async fn pack_diagonal_eq_t96() {
        pack_diagonal_eq::<PreprocK32S32>().await;
    }

    #[tokio::test]
    async fn pack_diagonal_eq_t192() {
        pack_diagonal_eq::<PreprocK64S64>().await;
    }

    #[tokio::test]
    async fn pack_diagonal_eq_t256() {
        pack_diagonal_eq::<PreprocK128S64>().await;
    }

    async fn pack_diagonal_eq<P: PreprocessorParameters>() {
        let mut rng = rand::thread_rng();
        let x = P::KSS::random(&mut rng);
        let actual = pack_diagonal(x);
        let diag: Vec<_> = (0..packing_capacity::<P::PlaintextParams>())
            .map(|_| x)
            .collect();
        let expected = pack::<P::PlaintextParams>(&diag);
        assert_eq!(expected, actual);
    }
}
