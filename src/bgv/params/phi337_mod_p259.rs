// Insecure toy ciphertext parameters (triple generation) for `k=s=32` and `U = 4V` without secure key generation

use crypto_bigint::{impl_modulus, modular::constant_mod::Residue, Uint};

use crate::bgv::{
    poly::{crt::CrtPolyParameters, CrtStrategy, PolyParameters},
    residue::{
        vec::{GenericResidueVec, ResidueVec},
        GenericResidue,
    },
};

impl_modulus!(
    Phi337ModP259,
    Uint::<5>,
    "0000000000000007ffffffffffffffffffffffffffffffffffffffffffffffffffffffffff975801"
);

impl PolyParameters for Phi337ModP259 {
    type Vec = ResidueVec<Self, 5>;
    type Residue = <Self::Vec as GenericResidueVec>::Residue;
    type Uint = <Self::Residue as GenericResidue>::Uint;

    const M: usize = 337;
    const CYCLOTOMIC_DEGREE: usize = 336;
}

impl CrtPolyParameters for Phi337ModP259 {
    const FACTOR_COUNT: usize = 336;
    const FACTOR_DEGREE: usize = 1;
    const SLOT_GENERATOR: usize = 10;
    const SLOT_GENERATOR_INVERSE: usize = 236;
    const CRT_STRATEGY: CrtStrategy = CrtStrategy::Fourier;
    const GENERATOR: Self::Residue = Residue::new(&Uint::<5>::from_u64(5));
}
