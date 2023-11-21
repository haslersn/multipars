// Plaintext parameters (triple generation) for `k=s=32` and `U = 4V` without secure key generation

use crypto_bigint::Zero;

use crate::bgv::{
    poly::{crt::CrtPolyParameters, CrtStrategy, PolyParameters},
    residue::{
        vec::{GenericResidueVec, NativeResidueVec},
        GenericResidue,
    },
    tweaked_interpolation_packing::TIPParameters,
};

#[derive(Debug, PartialEq)]
pub struct Phi43691ModT135 {}

impl PolyParameters for Phi43691ModT135 {
    type Vec = NativeResidueVec<135, 3>;
    type Residue = <Self::Vec as GenericResidueVec>::Residue;
    type Uint = <Self::Residue as GenericResidue>::Uint;

    const M: usize = 43691;
    const CYCLOTOMIC_DEGREE: usize = 43690;
}

impl CrtPolyParameters for Phi43691ModT135 {
    const FACTOR_COUNT: usize = 1285;
    const FACTOR_DEGREE: usize = 34;
    const SLOT_GENERATOR: usize = 13208;
    const SLOT_GENERATOR_INVERSE: usize = 12322;
    const CRT_STRATEGY: CrtStrategy = CrtStrategy::Factors {
        file: "params/phi43691_mod_t135.json",
    };
    const GENERATOR: Self::Residue = Zero::ZERO; // Multiplicative group is not cyclic
}

impl TIPParameters for Phi43691ModT135 {
    const DELTA: u32 = 15;
}
