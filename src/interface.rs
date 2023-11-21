use std::marker::PhantomData;
use std::ops::{Add, AddAssign, Mul, MulAssign, Neg, Sub, SubAssign};

use async_trait::async_trait;
use forward_ref_generic::{forward_ref_binop, forward_ref_op_assign, forward_ref_unop};

use crate::bgv::residue::native::GenericNativeResidue;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Share<KS, K, const PID: usize>
where
    KS: GenericNativeResidue,
    K: GenericNativeResidue,
{
    /// Share of the value.
    /// Note that (as usual in SPDZ2k-like protocols) only the lower part of the value matters.
    pub val: KS,
    /// Share of the MAC tag.
    pub tag: KS,
    pub phantom: PhantomData<K>,
}

#[derive(Clone, Debug)]
pub struct BeaverTriple<KS, K, const PID: usize>
where
    KS: GenericNativeResidue,
    K: GenericNativeResidue,
{
    pub a: Share<KS, K, PID>,
    pub b: Share<KS, K, PID>,
    pub c: Share<KS, K, PID>,
    pub phantom: PhantomData<K>,
}

#[async_trait]
pub trait Preprocessor<KS, K, const PID: usize>
where
    KS: GenericNativeResidue,
    K: GenericNativeResidue,
{
    /// Returns `n` `BeaverTriple`s
    async fn get_beaver_triples(&mut self, n: usize) -> Vec<BeaverTriple<KS, K, PID>>;

    async fn finish(self);
}

#[async_trait]
pub trait BatchedPreprocessor<KS, K, const PID: usize>
where
    KS: GenericNativeResidue,
    K: GenericNativeResidue,
{
    const BATCH_SIZE: usize;

    /// Returns `n` `BeaverTriple`s
    async fn get_beaver_triples(&mut self) -> Vec<BeaverTriple<KS, K, PID>>;

    async fn finish(self);
}

pub fn get_batch_size<Preproc, KS, K, const PID: usize>(_preproc: &Preproc) -> usize
where
    Preproc: BatchedPreprocessor<KS, K, PID>,
    KS: GenericNativeResidue,
    K: GenericNativeResidue,
{
    Preproc::BATCH_SIZE
}

impl<KS, K, const PID: usize> BeaverTriple<KS, K, PID>
where
    KS: GenericNativeResidue,
    K: GenericNativeResidue,
{
    pub const fn new(a: Share<KS, K, PID>, b: Share<KS, K, PID>, c: Share<KS, K, PID>) -> Self {
        Self {
            a,
            b,
            c,
            phantom: PhantomData,
        }
    }
}

impl<KS, K, const PID: usize> Share<KS, K, PID>
where
    KS: GenericNativeResidue,
    K: GenericNativeResidue,
{
    pub const ZERO: Self = Self::new(KS::ZERO, KS::ZERO);

    pub const fn new(val: KS, tag: KS) -> Self {
        Self {
            val,
            tag,
            phantom: PhantomData,
        }
    }
}

impl<KS, K, const PID: usize> From<K> for Share<KS, K, PID>
where
    KS: GenericNativeResidue,
    K: GenericNativeResidue,
{
    fn from(cleartext: K) -> Self {
        Self::new(
            if PID == 0 {
                KS::from_uint(cleartext.retrieve())
            } else {
                KS::ZERO
            },
            KS::ZERO, // TODO: Correct tag
        )
    }
}

impl<KS, K, const PID: usize> Add<Self> for Share<KS, K, PID>
where
    KS: GenericNativeResidue,
    K: GenericNativeResidue,
{
    type Output = Self;
    fn add(mut self, rhs: Self) -> Self {
        self += rhs;
        self
    }
}

forward_ref_binop!(
    [KS: GenericNativeResidue, K: GenericNativeResidue, const PID: usize]
    impl Add, add for Share<KS, K, PID>, Self
);

impl<KS, K, const PID: usize> Add<K> for Share<KS, K, PID>
where
    KS: GenericNativeResidue,
    K: GenericNativeResidue,
{
    type Output = Self;
    fn add(mut self, rhs: K) -> Self {
        self += rhs;
        self
    }
}

forward_ref_binop!(
    [KS: GenericNativeResidue, K: GenericNativeResidue, const PID: usize]
    impl Add, add for Share<KS, K, PID>, K
);

impl<KS, K, const PID: usize> AddAssign<Self> for Share<KS, K, PID>
where
    KS: GenericNativeResidue,
    K: GenericNativeResidue,
{
    fn add_assign(&mut self, rhs: Self) {
        self.val += rhs.val;
        self.tag += rhs.tag;
    }
}

forward_ref_op_assign!(
    [KS: GenericNativeResidue, K: GenericNativeResidue, const PID: usize]
    impl AddAssign, add_assign for Share<KS, K, PID>, Self
);

impl<KS, K, const PID: usize> AddAssign<K> for Share<KS, K, PID>
where
    KS: GenericNativeResidue,
    K: GenericNativeResidue,
{
    fn add_assign(&mut self, rhs: K) {
        *self += Self::from(rhs);
    }
}

forward_ref_op_assign!(
    [KS: GenericNativeResidue, K: GenericNativeResidue, const PID: usize]
    impl AddAssign, add_assign for Share<KS, K, PID>, K
);

impl<KS, K, const PID: usize> Sub<Self> for Share<KS, K, PID>
where
    KS: GenericNativeResidue,
    K: GenericNativeResidue,
{
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        self + -rhs
    }
}

forward_ref_binop!(
    [KS: GenericNativeResidue, K: GenericNativeResidue, const PID: usize]
    impl Sub, sub for Share<KS, K, PID>, Self
);

impl<KS, K, const PID: usize> Sub<K> for Share<KS, K, PID>
where
    KS: GenericNativeResidue,
    K: GenericNativeResidue,
{
    type Output = Self;
    fn sub(self, rhs: K) -> Self {
        self + -Self::from(rhs)
    }
}

forward_ref_binop!(
    [KS: GenericNativeResidue, K: GenericNativeResidue, const PID: usize]
    impl Sub, sub for Share<KS, K, PID>, K
);

impl<KS, K, const PID: usize> SubAssign<Self> for Share<KS, K, PID>
where
    KS: GenericNativeResidue,
    K: GenericNativeResidue,
{
    fn sub_assign(&mut self, rhs: Self) {
        *self += -rhs;
    }
}

forward_ref_op_assign!(
    [KS: GenericNativeResidue, K: GenericNativeResidue, const PID: usize]
    impl SubAssign, sub_assign for Share<KS, K, PID>, Self
);

impl<KS, K, const PID: usize> SubAssign<K> for Share<KS, K, PID>
where
    KS: GenericNativeResidue,
    K: GenericNativeResidue,
{
    fn sub_assign(&mut self, rhs: K) {
        *self -= Self::from(rhs);
    }
}

forward_ref_op_assign!(
    [KS: GenericNativeResidue, K: GenericNativeResidue, const PID: usize]
    impl SubAssign, sub_assign for Share<KS, K, PID>, K
);

impl<KS, K, const PID: usize> Neg for Share<KS, K, PID>
where
    KS: GenericNativeResidue,
    K: GenericNativeResidue,
{
    type Output = Self;
    fn neg(self) -> Self {
        Self::new(
            KS::ZERO - self.val, // TODO: Use Neg once available
            KS::ZERO - self.tag, // TODO: Use Neg once available
        )
    }
}

forward_ref_unop!(
    [KS: GenericNativeResidue, K: GenericNativeResidue, const PID: usize]
    impl Neg, neg for Share<KS, K, PID>
);

impl<KS, K, const PID: usize> Mul<K> for Share<KS, K, PID>
where
    KS: GenericNativeResidue,
    K: GenericNativeResidue,
{
    type Output = Self;
    fn mul(mut self, rhs: K) -> Self {
        self *= rhs;
        self
    }
}

forward_ref_binop!(
    [KS: GenericNativeResidue, K: GenericNativeResidue, const PID: usize]
    impl Mul, mul for Share<KS, K, PID>, K
);

impl<KS, K, const PID: usize> MulAssign<K> for Share<KS, K, PID>
where
    KS: GenericNativeResidue,
    K: GenericNativeResidue,
{
    fn mul_assign(&mut self, rhs: K) {
        let rhs = KS::from_unsigned(rhs);
        self.val = self.val * rhs;
        self.tag = self.tag * rhs;
    }
}

forward_ref_op_assign!(
    [KS: GenericNativeResidue, K: GenericNativeResidue, const PID: usize]
    impl MulAssign, mul_assign for Share<KS, K, PID>, K
);
