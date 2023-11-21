pub mod native;
pub mod vec;

use std::{
    cmp::min,
    fmt::Debug,
    ops::{Add, AddAssign, Mul, MulAssign, Sub, SubAssign},
};

use crypto_bigint::{
    modular::constant_mod::{Residue, ResidueParams},
    CtChoice, Integer, Limb, Random, Uint, Word, Zero,
};
use serde::{Deserialize, Serialize};

use super::generic_uint::{ExtendableUint, GenericUint};

pub trait GenericResidue:
    Clone
    + Copy
    + Debug
    + Eq
    + Random
    + Add<Output = Self>
    + Sub<Output = Self>
    + Mul<Output = Self>
    + AddAssign<Self>
    + SubAssign<Self>
    + MulAssign<Self>
    // TODO: Also require Neg
    + Zero
    + Serialize
    + for<'de> Deserialize<'de>
    + Send
    + Sync
    + 'static
{
    const BITS: usize;

    type Uint: ExtendableUint;

    fn retrieve(&self) -> Self::Uint;
    fn from_uint<SourceUint: GenericUint>(source: SourceUint) -> Self;
    fn from_i64(source: i64) -> Self;
    fn from_signed_int<SourceInt: GenericUint>(source: SourceInt) -> Self;
    fn from_reduced<SourceUint: GenericUint>(source: SourceUint) -> Self;
    fn invert(&self) -> (Self, CtChoice);

    fn from_unsigned<SourceRes: GenericResidue>(source: SourceRes) -> Self {
        Self::from_uint(source.retrieve())
    }

    /// This method is constant-time only with respect to `self`.  Depending on
    /// `exp`, timing can and will vary.
    fn pow_usize_vartime(mut self, mut exp: usize) -> Self {
        let mut result = Self::from_reduced(Self::Uint::ONE);
        while exp != 0 {
            if exp % 2 == 1 {
                result *= self;
            }
            exp >>= 1;
            self *= self;
        }
        result
    }

    /// This method is constant-time only with respect to `self`.  Depending on
    /// `exp`, timing can and will vary.
    fn pow_vartime<SourceInt: GenericUint>(mut self, mut exp: SourceInt) -> Self{
        let mut result = Self::from_reduced(Self::Uint::ONE);
        while exp != SourceInt::ZERO {
            if bool::from(exp.is_odd()) {
                result *= self;
            }
            exp = exp.shr_vartime(1);
            self *= self;
        }
        result
    }
}

impl<MOD, const NLIMBS: usize> GenericResidue for Residue<MOD, NLIMBS>
where
    MOD: ResidueParams<NLIMBS>,
    Uint<NLIMBS>: ExtendableUint,
{
    const BITS: usize = MOD::MODULUS.bits_vartime();

    type Uint = Uint<NLIMBS>;

    #[inline(always)]
    fn retrieve(&self) -> Self::Uint {
        self.retrieve()
    }

    #[inline(always)]
    fn from_uint<SourceUint: GenericUint>(source: SourceUint) -> Self {
        if SourceUint::NLIMBS > Self::Uint::NLIMBS {
            todo!();
            // let mut ext_modulus = SourceUint::ZERO;
            // ext_modulus.limbs_mut()[..NLIMBS].clone_from_slice(modulus.limbs());
            // source = source.reduce(&ext_modulus).unwrap();
        }

        let mut repr = Uint::ZERO;
        {
            let n = min(NLIMBS, SourceUint::NLIMBS);
            repr.limbs_mut()[..n].clone_from_slice(&source.limbs()[..n]);
        }

        Self::new(&repr)
    }

    #[inline(always)]
    fn from_i64(source: i64) -> Self {
        let mut repr = Uint::ZERO;
        repr.limbs_mut()[0].0 = source as u64;
        let sign = (source >> 63) as u64;
        for limb in &mut repr.limbs_mut()[1..] {
            limb.0 = sign;
        }

        let mut summand = MOD::MODULUS;
        for limb in summand.limbs_mut() {
            limb.0 &= sign;
        }
        repr = repr.wrapping_add(&summand);

        Self::new(&repr)
    }

    #[inline(always)]
    fn from_signed_int<SourceInt: GenericUint>(source: SourceInt) -> Self {
        if SourceInt::NLIMBS > NLIMBS {
            todo!();
        }

        let mut repr = Uint::ZERO;
        let n = min(NLIMBS, SourceInt::NLIMBS);
        repr.limbs_mut()[..n].clone_from_slice(&source.limbs()[..n]);
        let sign = (source.limbs()[n - 1].0 as i64 >> (Limb::BITS - 1)) as Word;
        for limb in &mut repr.limbs_mut()[n..] {
            limb.0 = sign;
        }

        let mut summand = MOD::MODULUS;
        for limb in summand.limbs_mut() {
            limb.0 &= sign;
        }
        repr = repr.wrapping_add(&summand);

        Self::new(&repr)
    }

    #[inline(always)]
    fn from_reduced<SourceUint: GenericUint>(source: SourceUint) -> Self {
        // TODO: check that source is reduced?

        let mut repr = Uint::ZERO;
        {
            let n = min(NLIMBS, SourceUint::NLIMBS);
            repr.limbs_mut()[..n].clone_from_slice(&source.limbs()[..n]);
        }

        Self::new(&repr)
    }

    #[inline(always)]
    fn invert(&self) -> (Self, CtChoice) {
        Residue::invert(&self)
    }
}

#[cfg(test)]
mod tests {
    use crypto_bigint::U64;
    use rand::Rng;

    use crate::bgv::{
        params::{ToyCipher, ToyPlain},
        poly::PolyParameters,
    };

    use super::GenericResidue;

    #[test]
    fn ciphertext_residue_add_assign() {
        residue_add_assign::<<ToyCipher as PolyParameters>::Residue>();
    }

    #[test]
    fn plaintext_residue_add_assign() {
        residue_add_assign::<<ToyPlain as PolyParameters>::Residue>();
    }

    fn residue_add_assign<Residue>()
    where
        Residue: GenericResidue,
    {
        let mut rng = rand::thread_rng();
        let lhs_num = rng.gen::<u64>();
        let rhs_num = rng.gen_range(0..u64::MAX - lhs_num);
        let mut lhs = Residue::from_uint(U64::from_u64(lhs_num));
        let rhs = Residue::from_uint(U64::from_u64(rhs_num));
        lhs += rhs;
        let result = Residue::from_uint(U64::from_u64(lhs_num + rhs_num));
        assert_eq!(lhs, result);
    }

    #[test]
    fn ciphertext_residue_add_assign_uint() {
        residue_add_assign_uint::<<ToyCipher as PolyParameters>::Residue>();
    }

    #[test]
    fn plaintext_residue_add_assign_uint() {
        residue_add_assign_uint::<<ToyPlain as PolyParameters>::Residue>();
    }

    fn residue_add_assign_uint<Residue>()
    where
        Residue: GenericResidue,
    {
        let mut rng = rand::thread_rng();
        let lhs_num = rng.gen::<u64>();
        let rhs_num = rng.gen_range(0..u64::MAX - lhs_num);
        let mut lhs = Residue::from_uint(U64::from_u64(lhs_num));
        lhs += Residue::from_reduced(U64::from_u64(rhs_num));
        let result = Residue::from_uint(U64::from_u64(lhs_num + rhs_num));
        assert_eq!(lhs, result);
    }

    #[test]
    fn ciphertext_residue_sub_assign() {
        residue_sub_assign::<<ToyCipher as PolyParameters>::Residue>();
    }

    #[test]
    fn plaintext_residue_sub_assign() {
        residue_sub_assign::<<ToyPlain as PolyParameters>::Residue>();
    }

    fn residue_sub_assign<Residue>()
    where
        Residue: GenericResidue,
    {
        let mut rng = rand::thread_rng();
        let lhs_num = rng.gen::<u64>();
        let rhs_num = rng.gen_range(0..lhs_num);
        let mut lhs = Residue::from_uint(U64::from_u64(lhs_num));
        let rhs = Residue::from_uint(U64::from_u64(rhs_num));
        lhs -= rhs;
        let result = Residue::from_uint(U64::from_u64(lhs_num - rhs_num));
        assert_eq!(lhs, result);
    }

    #[test]
    fn ciphertext_residue_sub_assign_uint() {
        residue_sub_assign_uint::<<ToyCipher as PolyParameters>::Residue>();
    }

    #[test]
    fn plaintext_residue_sub_assign_uint() {
        residue_sub_assign_uint::<<ToyPlain as PolyParameters>::Residue>();
    }

    fn residue_sub_assign_uint<Residue>()
    where
        Residue: GenericResidue,
    {
        let mut rng = rand::thread_rng();
        let lhs_num = rng.gen::<u64>();
        let rhs_num = rng.gen_range(0..lhs_num);
        let mut lhs = Residue::from_uint(U64::from_u64(lhs_num));
        lhs -= Residue::from_reduced(U64::from_u64(rhs_num));
        let result = Residue::from_uint(U64::from_u64(lhs_num - rhs_num));
        assert_eq!(lhs, result);
    }
}
