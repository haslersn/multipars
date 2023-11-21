// Insecure toy ciphertext parameters (authentication) for `k=s=32` and `U = 4V` without secure key generation

use crypto_bigint::{impl_modulus, modular::constant_mod::Residue, U192};

use crate::bgv::{
    poly::{crt::CrtPolyParameters, CrtStrategy, PolyParameters},
    residue::{
        vec::{GenericResidueVec, ResidueVec},
        GenericResidue,
    },
};

impl_modulus!(
    Phi179ModP163,
    U192,
    "00000007ffffffffffffffffffffffffffffffffffba9e01"
);

impl PolyParameters for Phi179ModP163 {
    type Vec = ResidueVec<Self, 3>;
    type Residue = <Self::Vec as GenericResidueVec>::Residue;
    type Uint = <Self::Residue as GenericResidue>::Uint;

    const M: usize = 179;
    const CYCLOTOMIC_DEGREE: usize = 178;
}

impl CrtPolyParameters for Phi179ModP163 {
    const FACTOR_COUNT: usize = 178;
    const FACTOR_DEGREE: usize = 1;
    const SLOT_GENERATOR: usize = 2;
    const SLOT_GENERATOR_INVERSE: usize = 90;
    const CRT_STRATEGY: CrtStrategy = CrtStrategy::Fourier;
    const GENERATOR: Self::Residue = Residue::new(&U192::from_u64(5));
}
