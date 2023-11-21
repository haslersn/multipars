use std::{
    fmt::Debug,
    ops::{Index, IndexMut},
};

use crypto_bigint::{
    modular::constant_mod::{Residue, ResidueParams},
    Uint, Zero,
};
use serde::{Deserialize, Serialize};

use crate::bgv::generic_uint::ExtendableUint;

use super::{native::NativeResidue, GenericResidue};

pub trait GenericResidueVec:
    IndexMut<usize, Output = Self::Residue>
    + Clone
    + Debug
    + Eq
    + Serialize
    + for<'de> Deserialize<'de>
    + Send
    + Sync
{
    type Residue: GenericResidue;

    fn new(len: usize) -> Self;

    fn len(&self) -> usize;

    fn iter(&self) -> impl ExactSizeIterator + DoubleEndedIterator<Item = &Self::Residue>;

    fn iter_mut(
        &mut self,
    ) -> impl ExactSizeIterator + DoubleEndedIterator<Item = &mut Self::Residue>;
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(bound(deserialize = ""))]
#[serde(bound(serialize = ""))]
pub struct ResidueVec<MOD, const NLIMBS: usize>(Vec<Residue<MOD, NLIMBS>>)
where
    MOD: ResidueParams<NLIMBS>,
    Uint<NLIMBS>: ExtendableUint;

impl<MOD, const NLIMBS: usize> Index<usize> for ResidueVec<MOD, NLIMBS>
where
    MOD: ResidueParams<NLIMBS>,
    Uint<NLIMBS>: ExtendableUint,
{
    type Output = Residue<MOD, NLIMBS>;

    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}

impl<MOD, const NLIMBS: usize> IndexMut<usize> for ResidueVec<MOD, NLIMBS>
where
    MOD: ResidueParams<NLIMBS>,
    Uint<NLIMBS>: ExtendableUint,
{
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.0[index]
    }
}

impl<MOD, const NLIMBS: usize> GenericResidueVec for ResidueVec<MOD, NLIMBS>
where
    MOD: ResidueParams<NLIMBS>,
    Uint<NLIMBS>: ExtendableUint,
{
    type Residue = Residue<MOD, NLIMBS>;

    fn new(len: usize) -> Self {
        Self(vec![Self::Residue::ZERO; len])
    }

    fn len(&self) -> usize {
        self.0.len()
    }

    fn iter(&self) -> impl ExactSizeIterator + DoubleEndedIterator<Item = &Self::Residue> {
        self.0.iter()
    }

    fn iter_mut(
        &mut self,
    ) -> impl ExactSizeIterator + DoubleEndedIterator<Item = &mut Self::Residue> {
        self.0.iter_mut()
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(bound(deserialize = ""))]
#[serde(bound(serialize = ""))]
pub struct NativeResidueVec<const BITS: usize, const NLIMBS: usize>(
    Vec<NativeResidue<BITS, NLIMBS>>,
)
where
    Uint<NLIMBS>: ExtendableUint;

impl<const BITS: usize, const NLIMBS: usize> Index<usize> for NativeResidueVec<BITS, NLIMBS>
where
    Uint<NLIMBS>: ExtendableUint,
{
    type Output = NativeResidue<BITS, NLIMBS>;

    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}

impl<const BITS: usize, const NLIMBS: usize> IndexMut<usize> for NativeResidueVec<BITS, NLIMBS>
where
    Uint<NLIMBS>: ExtendableUint,
{
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.0[index]
    }
}

impl<const BITS: usize, const NLIMBS: usize> GenericResidueVec for NativeResidueVec<BITS, NLIMBS>
where
    Uint<NLIMBS>: ExtendableUint,
{
    type Residue = NativeResidue<BITS, NLIMBS>;

    fn new(len: usize) -> Self {
        Self(vec![Self::Residue::ZERO; len])
    }

    fn len(&self) -> usize {
        self.0.len()
    }

    fn iter(&self) -> impl ExactSizeIterator + DoubleEndedIterator<Item = &Self::Residue> {
        self.0.iter()
    }

    fn iter_mut(
        &mut self,
    ) -> impl ExactSizeIterator + DoubleEndedIterator<Item = &mut Self::Residue> {
        self.0.iter_mut()
    }
}
