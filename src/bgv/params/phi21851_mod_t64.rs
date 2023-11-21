// Plaintext parameters (authentication) for `k=s=32` and `U = 4V` without secure key generation

use crate::bgv::{
    poly::PolyParameters,
    residue::{
        vec::{GenericResidueVec, NativeResidueVec},
        GenericResidue,
    },
};

#[derive(Debug, PartialEq)]
pub struct Phi21851ModT64 {}

impl PolyParameters for Phi21851ModT64 {
    type Vec = NativeResidueVec<64, 1>;
    type Residue = <Self::Vec as GenericResidueVec>::Residue;
    type Uint = <Self::Residue as GenericResidue>::Uint;

    const M: usize = 21851;
    const CYCLOTOMIC_DEGREE: usize = 21850;
}
