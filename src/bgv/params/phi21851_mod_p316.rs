// Ciphertext parameters (authentication) for `k=s=64` and `U = 4V` without secure key generation

use crypto_bigint::{impl_modulus, modular::constant_mod::Residue, Uint};

use crate::bgv::{
    poly::{crt::CrtPolyParameters, CrtStrategy, PolyParameters},
    residue::{
        vec::{GenericResidueVec, ResidueVec},
        GenericResidue,
    },
};

impl_modulus!(
    Phi21851ModP316,
    Uint::<5>,
    "0fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff\
     fffffff6cd650001"
);

impl PolyParameters for Phi21851ModP316 {
    type Vec = ResidueVec<Self, 5>;
    type Residue = <Self::Vec as GenericResidueVec>::Residue;
    type Uint = <Self::Residue as GenericResidue>::Uint;

    const M: usize = 21851;
    const CYCLOTOMIC_DEGREE: usize = 21850;
}

impl CrtPolyParameters for Phi21851ModP316 {
    const FACTOR_COUNT: usize = 21850;
    const FACTOR_DEGREE: usize = 1;
    const SLOT_GENERATOR: usize = 6;
    const SLOT_GENERATOR_INVERSE: usize = 3642;
    const CRT_STRATEGY: CrtStrategy = CrtStrategy::Fourier;
    const GENERATOR: Self::Residue = Residue::new(&Uint::<5>::from_u64(7));
}
