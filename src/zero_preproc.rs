use std::marker::PhantomData;

use async_trait::async_trait;

use crate::{
    bgv::residue::native::GenericNativeResidue,
    interface::{BeaverTriple, Preprocessor, Share},
};

pub struct ZeroPreprocessor {}

impl Default for ZeroPreprocessor {
    fn default() -> Self {
        Self {}
    }
}

#[async_trait]
impl<KS, K, const PID: usize> Preprocessor<KS, K, PID> for ZeroPreprocessor
where
    KS: GenericNativeResidue,
    K: GenericNativeResidue,
{
    async fn get_beaver_triples(&mut self, n: usize) -> Vec<BeaverTriple<KS, K, PID>> {
        let zero = BeaverTriple {
            a: Share::ZERO,
            b: Share::ZERO,
            c: Share::ZERO,
            phantom: PhantomData,
        };
        vec![zero; n]
    }

    async fn finish(self) {}
}
