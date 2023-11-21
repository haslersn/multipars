use std::{fmt::Debug, fs::File, io::BufReader};

use crypto_bigint::{Integer, U64};
use serde::{Deserialize, Serialize};

use crate::bgv::generic_uint::GenericUint;

use self::crt::CrtPolyParameters;

use super::{
    fourier::fast_fourier_transform,
    generic_uint::ExtendableUint,
    residue::{vec::GenericResidueVec, GenericResidue},
};

pub mod crt;
pub mod power;

// We currently need to wrap residues in this annoying `Diagonal` struct when
// using some overloaded operators, because otherwise the compiler refuses to
// compile the overloaded operators due to conflicting implementations.
#[derive(Clone, Copy)]
pub struct Diagonal<R: GenericResidue>(pub R);

pub trait PolyParameters: PartialEq + Debug + Send + Sync + 'static {
    type Vec: GenericResidueVec<Residue = Self::Residue>;
    type Residue: GenericResidue<Uint = Self::Uint>;
    type Uint: ExtendableUint;

    /// Determines the polynomial `\Phi_m(X)`.
    const M: usize;

    /// Must be the degree of `\Phi_m(X)`, i.e. `\phi(m)`.
    const CYCLOTOMIC_DEGREE: usize;
}

pub enum CrtStrategy {
    Factors { file: &'static str },
    Fourier,
}

pub trait FourierCrtPolyParameters: CrtPolyParameters
where
    Self: CrtPolyParameters<CRT_STRATEGY = { CrtStrategy::Fourier }>,
{
}

impl<P> FourierCrtPolyParameters for P where
    P: CrtPolyParameters<CRT_STRATEGY = { CrtStrategy::Fourier }>
{
}

#[derive(Debug, Deserialize, Serialize)]
pub enum CrtContext<P>
where
    P: CrtPolyParameters,
{
    Factors(FactorsContext<P>),
    Fourier(FourierContext<P>),
}

#[derive(Debug, Deserialize, Serialize)]
pub struct FactorsContext<P>
where
    P: CrtPolyParameters,
{
    pub factors: P::Vec,
    pub basis_coefficients: P::Vec,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct FourierContext<P>
where
    P: CrtPolyParameters,
{
    m_inverse: P::Residue,
    mth_root: P::Residue,
    mth_root_inverse: P::Residue,
    pub dft_size: usize,
    pub dft_size_inverse: P::Residue,
    kernel_from_crt: P::Vec,
    kernel_from_power: P::Vec,
    pub dft_root_powers: P::Vec,
}

impl<P> CrtContext<P>
where
    P: CrtPolyParameters,
{
    pub async fn gen() -> Self {
        match P::CRT_STRATEGY {
            CrtStrategy::Factors { file } => Self::read_factors(file).await,
            CrtStrategy::Fourier => Self::gen_fourier().await,
        }
    }

    async fn read_factors(path: &str) -> Self {
        // TODO: Error handling
        let file = File::open(path).unwrap();
        let reader = BufReader::new(file);
        CrtContext::Factors(serde_json::from_reader(reader).unwrap())
    }

    async fn gen_fourier() -> Self {
        let (m_inverse, exists) = P::Residue::from_uint(U64::from_u64(P::M as u64)).invert();
        assert!(bool::from(exists));

        // We have prime modulus. For prime modulus q, the group order is phi(q) = q-1.
        // We can use -1 which gets reduced to q-1.
        let group_order = P::Residue::from_i64(-1).retrieve();

        // TODO: mention in the paper that we require m-1 to be a multiple of m and dft_size.
        let mth_root = {
            let (div, rem) = group_order.div_rem_u64(P::M as u64);
            assert_eq!(rem, 0);
            P::GENERATOR.pow_vartime(div)
        };

        let (mth_root_inverse, exists) = mth_root.invert();
        assert!(bool::from(exists));

        let dft_size = (2 * P::CYCLOTOMIC_DEGREE - 1).next_power_of_two();
        assert_ne!(dft_size, 0);

        let (dft_size_inverse, exists) =
            P::Residue::from_uint(U64::from_u64(dft_size as u64)).invert();
        assert!(bool::from(exists));

        let mut dft_root_powers = P::Vec::new(dft_size);
        {
            let dft_root = P::GENERATOR
                .pow_vartime(group_order.shr_vartime(dft_size.trailing_zeros() as usize));
            let mut current = P::Residue::from_reduced(<P::Residue as GenericResidue>::Uint::ONE);
            dft_root_powers[0] = current;
            for entry in dft_root_powers.iter_mut().skip(1) {
                current *= dft_root;
                *entry = current;
            }
        }

        CrtContext::Fourier(FourierContext {
            m_inverse,
            mth_root,
            mth_root_inverse,
            dft_size,
            dft_size_inverse,
            kernel_from_crt: {
                let mut kernel = P::Vec::new(dft_size);
                let mut root = mth_root_inverse;
                let common_factor = m_inverse * dft_size_inverse;
                for entry in kernel.iter_mut().take(P::CYCLOTOMIC_DEGREE).rev() {
                    root = root.pow_usize_vartime(P::SLOT_GENERATOR);
                    *entry =
                        root - P::Residue::from_reduced(<P::Residue as GenericResidue>::Uint::ONE);
                    *entry *= common_factor;
                }
                fast_fourier_transform(&dft_root_powers, false, kernel).await
            },
            kernel_from_power: {
                let mut kernel = P::Vec::new(dft_size);
                let mut root = mth_root;
                for entry in kernel.iter_mut().take(P::CYCLOTOMIC_DEGREE) {
                    *entry = root * dft_size_inverse;
                    root = root.pow_usize_vartime(P::SLOT_GENERATOR);
                }
                fast_fourier_transform(&dft_root_powers, false, kernel).await
            },
            dft_root_powers,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::bgv::{
        params::{ToyCipher, ToyPlain},
        poly::{crt::CrtPoly, power::PowerPoly, CrtContext},
    };

    use super::crt::CrtPolyParameters;

    #[tokio::test]
    async fn ciphertext_basis_roundtrip_crt() {
        basis_roundtrip_crt::<ToyCipher>().await;
    }

    #[tokio::test]
    async fn plaintext_basis_roundtrip_crt() {
        basis_roundtrip_crt::<ToyPlain>().await;
    }

    async fn basis_roundtrip_crt<P>()
    where
        P: CrtPolyParameters,
    {
        let mut rng = rand::thread_rng();
        let ctx = CrtContext::gen().await;
        let crt = CrtPoly::<P>::random(&mut rng);
        let power = PowerPoly::from_crt(&ctx, &crt).await;
        let crt_roundtrip = CrtPoly::from_power(&ctx, &power).await;
        assert_eq!(crt, crt_roundtrip);
    }

    #[tokio::test]
    async fn ciphertext_basis_roundtrip_power() {
        basis_roundtrip_power::<ToyCipher>().await;
    }

    #[tokio::test]
    async fn plaintext_basis_roundtrip_power() {
        basis_roundtrip_power::<ToyPlain>().await;
    }

    async fn basis_roundtrip_power<P>()
    where
        P: CrtPolyParameters,
    {
        let mut rng = rand::thread_rng();
        let ctx = CrtContext::gen().await;
        let power = PowerPoly::<P>::random(&mut rng);
        let crt = CrtPoly::from_power(&ctx, &power).await;
        let power_roundtrip = PowerPoly::from_crt(&ctx, &crt).await;
        assert_eq!(power, power_roundtrip);
    }
}
