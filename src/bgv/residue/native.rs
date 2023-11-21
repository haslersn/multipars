use std::{
    cmp::min,
    ops::{Add, AddAssign, Mul, MulAssign, Sub, SubAssign},
};

use crypto_bigint::{
    rand_core::CryptoRngCore,
    subtle::{Choice, ConstantTimeEq},
    CtChoice, Encoding, Limb, Random, Uint, Word, Zero,
};
use serde::{Deserialize, Serialize};

use crate::bgv::generic_uint::{ExtendableUint, GenericUint};

use super::GenericResidue;

pub trait GenericNativeResidue: GenericResidue {}

// TODO: Serialize and Deserialize must use reduced form for security (and shortness).
#[derive(Clone, Copy, Debug, Eq, Serialize, Deserialize)]
#[serde(bound(deserialize = "Uint<NLIMBS>: Encoding"))]
#[serde(bound(serialize = "Uint<NLIMBS>: Encoding"))]
pub struct NativeResidue<const BITS: usize, const NLIMBS: usize>(Uint<NLIMBS>)
where
    Uint<NLIMBS>: ExtendableUint;

impl<const BITS: usize, const NLIMBS: usize> GenericNativeResidue for NativeResidue<BITS, NLIMBS>
where
    Self: GenericResidue,
    Uint<NLIMBS>: ExtendableUint,
{
}

impl<const BITS: usize, const NLIMBS: usize> Zero for NativeResidue<BITS, NLIMBS>
where
    Uint<NLIMBS>: ExtendableUint,
{
    const ZERO: Self = Self(Uint::ZERO);
}

impl<const BITS: usize, const NLIMBS: usize> PartialEq for NativeResidue<BITS, NLIMBS>
where
    Uint<NLIMBS>: ExtendableUint,
{
    fn eq(&self, other: &Self) -> bool {
        self.retrieve() == other.retrieve()
    }
}

impl<const BITS: usize, const NLIMBS: usize> ConstantTimeEq for NativeResidue<BITS, NLIMBS>
where
    Uint<NLIMBS>: ExtendableUint,
{
    fn ct_eq(&self, other: &Self) -> Choice {
        self.retrieve().ct_eq(&other.retrieve())
    }
}

impl<const BITS: usize, const NLIMBS: usize> Random for NativeResidue<BITS, NLIMBS>
where
    Uint<NLIMBS>: ExtendableUint,
{
    fn random(rng: &mut impl CryptoRngCore) -> Self {
        Self(Random::random(rng))
    }
}

impl<const BITS: usize, const NLIMBS: usize> Add for NativeResidue<BITS, NLIMBS>
where
    Uint<NLIMBS>: ExtendableUint,
{
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0.wrapping_add(&rhs.0))
    }
}

impl<const BITS: usize, const NLIMBS: usize> Sub for NativeResidue<BITS, NLIMBS>
where
    Uint<NLIMBS>: ExtendableUint,
{
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0.wrapping_sub(&rhs.0))
    }
}

impl<const BITS: usize, const NLIMBS: usize> Mul for NativeResidue<BITS, NLIMBS>
where
    Uint<NLIMBS>: ExtendableUint,
{
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        Self(self.0.wrapping_mul(&rhs.0))
    }
}

impl<const BITS: usize, const NLIMBS: usize> AddAssign<Self> for NativeResidue<BITS, NLIMBS>
where
    Uint<NLIMBS>: ExtendableUint,
{
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl<const BITS: usize, const NLIMBS: usize> SubAssign<Self> for NativeResidue<BITS, NLIMBS>
where
    Uint<NLIMBS>: ExtendableUint,
{
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}

impl<const BITS: usize, const NLIMBS: usize> MulAssign<Self> for NativeResidue<BITS, NLIMBS>
where
    Uint<NLIMBS>: ExtendableUint,
{
    fn mul_assign(&mut self, rhs: Self) {
        *self = *self * rhs;
    }
}

impl<const BITS: usize, const NLIMBS: usize> GenericResidue for NativeResidue<BITS, NLIMBS>
where
    Uint<NLIMBS>: ExtendableUint,
{
    const BITS: usize = BITS;

    type Uint = Uint<NLIMBS>;

    #[inline(always)]
    fn retrieve(&self) -> Self::Uint {
        let mut repr = self.0;
        let cutoff = Uint::NLIMBS * 64 - BITS;
        debug_assert!(cutoff < 64);
        repr.limbs_mut()[Uint::NLIMBS - 1].0 &= Word::MAX >> cutoff;
        repr
    }

    #[inline(always)]
    fn from_uint<SourceUint: GenericUint>(source: SourceUint) -> Self {
        let mut repr = Self::Uint::ZERO;
        {
            let n = min(Self::Uint::NLIMBS, SourceUint::NLIMBS);
            repr.limbs_mut()[..n].clone_from_slice(&source.limbs()[..n]);
        }
        Self(repr)
    }

    #[inline(always)]
    fn from_i64(source: i64) -> Self {
        Self(Uint::from_i64(source))
    }

    #[inline(always)]
    fn from_signed_int<SourceInt: GenericUint>(source: SourceInt) -> Self {
        let mut repr = Self::Uint::ZERO;
        let n = min(Self::Uint::NLIMBS, SourceInt::NLIMBS);
        repr.limbs_mut()[..n].clone_from_slice(&source.limbs()[..n]);
        let sign = (source.limbs()[n - 1].0 as i64 >> (Limb::BITS - 1)) as Word;
        for limb in &mut repr.limbs_mut()[n..] {
            limb.0 = sign;
        }
        Self(repr)
    }

    #[inline(always)]
    fn from_reduced<SourceUint: GenericUint>(source: SourceUint) -> Self {
        // TODO: check that source is reduced?
        Self::from_uint(source)
    }

    #[inline(always)]
    fn invert(&self) -> (Self, CtChoice) {
        // TODO: to implement this correctly, we need to return False if `self` is even.
        (Self(self.0.inv_mod2k_vartime(BITS)), CtChoice::TRUE)
    }
}
