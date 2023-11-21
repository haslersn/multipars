// Insecure toy plaintext parameters (triple generation) for `k=s=32` and `U = 4V` without secure key generation

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
pub struct Phi337ModT86 {}

impl PolyParameters for Phi337ModT86 {
    type Vec = NativeResidueVec<86, 2>;
    type Residue = <Self::Vec as GenericResidueVec>::Residue;
    type Uint = <Self::Residue as GenericResidue>::Uint;

    const M: usize = 337;
    const CYCLOTOMIC_DEGREE: usize = 336;
}

impl CrtPolyParameters for Phi337ModT86 {
    const FACTOR_COUNT: usize = 16;
    const FACTOR_DEGREE: usize = 21;
    const SLOT_GENERATOR: usize = 191;
    const SLOT_GENERATOR_INVERSE: usize = 30;
    const CRT_STRATEGY: CrtStrategy = CrtStrategy::Factors {
        file: "params/phi337_mod_t86.json",
    };
    const GENERATOR: Self::Residue = Zero::ZERO; // Multiplicative group is not cyclic
}

impl TIPParameters for Phi337ModT86 {
    const DELTA: u32 = 8;
}
