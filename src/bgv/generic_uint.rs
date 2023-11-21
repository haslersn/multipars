use crypto_bigint::{Encoding, Integer, Limb, NonZero, Random, RandomMod, Uint};
use serde::{Deserialize, Serialize};

pub trait GenericUint:
    Encoding + Integer + Random + RandomMod + Serialize + for<'de> Deserialize<'de>
{
    const NLIMBS: usize;
    fn add_mod_special(&self, rhs: &Self, c: Limb) -> Self;
    fn sub_mod_special(&self, rhs: &Self, c: Limb) -> Self;
    fn mul_mod_special(&self, rhs: &Self, c: Limb) -> Self;
    fn wrapping_add(&self, rhs: &Self) -> Self;
    fn wrapping_sub(&self, rhs: &Self) -> Self;
    fn wrapping_mul(&self, rhs: &Self) -> Self;
    fn limbs(&self) -> &[Limb];
    fn limbs_mut(&mut self) -> &mut [Limb];
    fn from_u32(n: u32) -> Self;
    fn from_i64(source: i64) -> Self;
    fn shr_vartime(&self, shift: usize) -> Self;
    fn shl_vartime(&self, shift: usize) -> Self;
    fn div_rem_u64(&self, rhs: u64) -> (Self, u64);
}

impl<const NLIMBS: usize> GenericUint for Uint<NLIMBS>
where
    Self: Encoding,
{
    const NLIMBS: usize = NLIMBS;

    #[inline(always)]
    fn add_mod_special(&self, rhs: &Self, c: Limb) -> Self {
        self.add_mod_special(rhs, c)
    }

    #[inline(always)]
    fn sub_mod_special(&self, rhs: &Self, c: Limb) -> Self {
        self.sub_mod_special(rhs, c)
    }

    #[inline(always)]
    fn mul_mod_special(&self, rhs: &Self, c: Limb) -> Self {
        self.mul_mod_special(rhs, c)
    }

    #[inline(always)]
    fn wrapping_add(&self, rhs: &Self) -> Self {
        self.wrapping_add(rhs)
    }

    #[inline(always)]
    fn wrapping_sub(&self, rhs: &Self) -> Self {
        self.wrapping_sub(rhs)
    }

    #[inline(always)]
    fn wrapping_mul(&self, rhs: &Self) -> Self {
        self.wrapping_mul(rhs)
    }

    #[inline(always)]
    fn limbs(&self) -> &[Limb] {
        self.as_limbs()
    }

    #[inline(always)]
    fn limbs_mut(&mut self) -> &mut [Limb] {
        self.as_limbs_mut()
    }

    #[inline(always)]
    fn from_u32(n: u32) -> Self {
        Self::from_u32(n)
    }

    #[inline(always)]
    fn from_i64(source: i64) -> Self {
        let mut repr = Self::ZERO;
        repr.limbs_mut()[0].0 = source as u64;
        let sign = (source >> 63) as u64;
        for limb in &mut repr.limbs_mut()[1..] {
            limb.0 = sign;
        }
        repr
    }

    #[inline(always)]
    fn shr_vartime(&self, shift: usize) -> Self {
        Uint::shr_vartime(self, shift)
    }

    #[inline(always)]
    fn shl_vartime(&self, shift: usize) -> Self {
        Uint::shl_vartime(self, shift)
    }

    #[inline(always)]
    fn div_rem_u64(&self, rhs: u64) -> (Self, u64) {
        let (div, rem) = Uint::div_rem_limb(self, NonZero::new(Limb(rhs)).unwrap());
        (div, rem.0)
    }
}

pub trait ExtendableUint: GenericUint {
    type Extended: GenericUint;
}

macro_rules! impl_extendable_uint {
    ($nlimbs:expr) => {
        impl ExtendableUint for Uint<$nlimbs> {
            type Extended = Uint<{ $nlimbs + 1 }>;
        }
    };
}

impl_extendable_uint!(1);
impl_extendable_uint!(2);
impl_extendable_uint!(3);
impl_extendable_uint!(4);
impl_extendable_uint!(5);
impl_extendable_uint!(6);
impl_extendable_uint!(7);
impl_extendable_uint!(8);
impl_extendable_uint!(9);
impl_extendable_uint!(10);
impl_extendable_uint!(11);
impl_extendable_uint!(12);
