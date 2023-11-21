use crate::{
    bgv::{
        params::{
            phi337_mod_p259::Phi337ModP259, phi337_mod_t86::Phi337ModT86,
            phi43691_mod_p387::Phi43691ModP387, phi43691_mod_p616::Phi43691ModP616,
            phi43691_mod_p744::Phi43691ModP744, phi43691_mod_t135::Phi43691ModT135,
            phi43691_mod_t233::Phi43691ModT233, phi43691_mod_t297::Phi43691ModT297,
        },
        poly::PolyParameters,
        residue::native::NativeResidue,
    },
    low_gear_dealer::params::{DealerK128S64, DealerK32S32, DealerK64S64, ToyDealerK32S32},
};

use super::PreprocessorParameters;

#[derive(Debug, PartialEq)]
pub struct ToyPreprocK32S32 {}

impl PreprocessorParameters for ToyPreprocK32S32 {
    type DealerParams = ToyDealerK32S32;
    type PlaintextUint = <Self::PlaintextParams as PolyParameters>::Uint;
    type PlaintextParams = Phi337ModT86;
    type CiphertextParams = Phi337ModP259;
    type BgvParams = (Self::PlaintextParams, Self::CiphertextParams);
    type K = NativeResidue<32, 1>;
    type S = NativeResidue<32, 1>;
    type KS = NativeResidue<64, 1>;
    type KSS = NativeResidue<96, 2>;

    // TODO: can we use `zkpopk::num_proofs`? Requires `const fn`.
    const ZKPOPK_AMORTIZE: usize = 4 * 4;
    const ZKPOPK_SND_SEC: usize = 26;
}

#[derive(Debug, PartialEq)]
pub struct PreprocK32S32 {}

impl PreprocessorParameters for PreprocK32S32 {
    type DealerParams = DealerK32S32;
    type PlaintextUint = <Self::PlaintextParams as PolyParameters>::Uint;
    type PlaintextParams = Phi43691ModT135;
    type CiphertextParams = Phi43691ModP387;
    type BgvParams = (Self::PlaintextParams, Self::CiphertextParams);
    type K = NativeResidue<32, 1>;
    type S = NativeResidue<32, 1>;
    type KS = NativeResidue<64, 1>;
    type KSS = NativeResidue<96, 2>;

    // TODO: can we use `zkpopk::num_proofs`? Requires `const fn`.
    const ZKPOPK_AMORTIZE: usize = 4 * 3;
    const ZKPOPK_SND_SEC: usize = 26;
}

#[derive(Debug, PartialEq)]
pub struct PreprocK64S64 {}

impl PreprocessorParameters for PreprocK64S64 {
    type DealerParams = DealerK64S64;
    type PlaintextUint = <Self::PlaintextParams as PolyParameters>::Uint;
    type PlaintextParams = Phi43691ModT233;
    type CiphertextParams = Phi43691ModP616;
    type BgvParams = (Self::PlaintextParams, Self::CiphertextParams);
    type K = NativeResidue<64, 1>;
    type S = NativeResidue<64, 1>;
    type KS = NativeResidue<128, 2>;
    type KSS = NativeResidue<192, 3>;

    // TODO: can we use `zkpopk::num_proofs`? Requires `const fn`.
    const ZKPOPK_AMORTIZE: usize = 4 * 5;
    const ZKPOPK_SND_SEC: usize = 57;
}

#[derive(Debug, PartialEq)]
pub struct PreprocK128S64 {}

impl PreprocessorParameters for PreprocK128S64 {
    type DealerParams = DealerK128S64;
    type PlaintextUint = <Self::PlaintextParams as PolyParameters>::Uint;
    type PlaintextParams = Phi43691ModT297;
    type CiphertextParams = Phi43691ModP744;
    type BgvParams = (Self::PlaintextParams, Self::CiphertextParams);
    type K = NativeResidue<128, 2>;
    type S = NativeResidue<64, 1>;
    type KS = NativeResidue<192, 3>;
    type KSS = NativeResidue<256, 4>;

    // TODO: can we use `zkpopk::num_proofs`? Requires `const fn`.
    const ZKPOPK_AMORTIZE: usize = 4 * 5;
    const ZKPOPK_SND_SEC: usize = 57;
}
