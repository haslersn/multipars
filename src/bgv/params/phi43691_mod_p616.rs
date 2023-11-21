// Ciphertext parameters (triple generation) for `k=s=64` and `U = 4V` without secure key generation

use crypto_bigint::{impl_modulus, modular::constant_mod::Residue, Uint};

use crate::bgv::{
    poly::{crt::CrtPolyParameters, CrtStrategy, PolyParameters},
    residue::{
        vec::{GenericResidueVec, ResidueVec},
        GenericResidue,
    },
};

impl_modulus!(
    Phi43691ModP616,
    Uint::<10>,
    "000000ffffffffffffffffffffffffffffffffffffffffffffffffffffffffff\
     ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff\
     ffffffffffffffffffffff53ffca0001"
);

impl PolyParameters for Phi43691ModP616 {
    type Vec = ResidueVec<Self, 10>;
    type Residue = <Self::Vec as GenericResidueVec>::Residue;
    type Uint = <Self::Residue as GenericResidue>::Uint;

    const M: usize = 43691;
    const CYCLOTOMIC_DEGREE: usize = 43690;
}

impl CrtPolyParameters for Phi43691ModP616 {
    const FACTOR_COUNT: usize = 43690;
    const FACTOR_DEGREE: usize = 1;
    const SLOT_GENERATOR: usize = 6;
    const SLOT_GENERATOR_INVERSE: usize = 7282;
    const CRT_STRATEGY: CrtStrategy = CrtStrategy::Fourier;
    const GENERATOR: Self::Residue = Residue::new(&Uint::<10>::from_u64(7));
}
