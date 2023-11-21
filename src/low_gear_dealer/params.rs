use crate::bgv::{
    params::{
        phi179_mod_p163::Phi179ModP163, phi179_mod_t64::Phi179ModT64,
        phi21851_mod_p188::Phi21851ModP188, phi21851_mod_p316::Phi21851ModP316,
        phi21851_mod_p444::Phi21851ModP444, phi21851_mod_t128::Phi21851ModT128,
        phi21851_mod_t192::Phi21851ModT192, phi21851_mod_t64::Phi21851ModT64,
    },
    residue::native::NativeResidue,
};

use super::DealerParameters;

#[derive(Debug, PartialEq)]
pub struct ToyDealerK32S32 {}

impl DealerParameters for ToyDealerK32S32 {
    type PlaintextParams = Phi179ModT64;
    type CiphertextParams = Phi179ModP163;
    type BgvParams = (Self::PlaintextParams, Self::CiphertextParams);
    type K = NativeResidue<32, 1>;
    type S = NativeResidue<32, 1>;
    type KS = NativeResidue<64, 1>;
}

#[derive(Debug, PartialEq)]
pub struct DealerK32S32 {}

impl DealerParameters for DealerK32S32 {
    type PlaintextParams = Phi21851ModT64;
    type CiphertextParams = Phi21851ModP188;
    type BgvParams = (Self::PlaintextParams, Self::CiphertextParams);
    type K = NativeResidue<32, 1>;
    type S = NativeResidue<32, 1>;
    type KS = NativeResidue<64, 1>;
}

#[derive(Debug, PartialEq)]
pub struct DealerK64S64 {}

impl DealerParameters for DealerK64S64 {
    type PlaintextParams = Phi21851ModT128;
    type CiphertextParams = Phi21851ModP316;
    type BgvParams = (Self::PlaintextParams, Self::CiphertextParams);
    type K = NativeResidue<64, 1>;
    type S = NativeResidue<64, 1>;
    type KS = NativeResidue<128, 2>;
}

#[derive(Debug, PartialEq)]
pub struct DealerK128S64 {}

impl DealerParameters for DealerK128S64 {
    type PlaintextParams = Phi21851ModT192;
    type CiphertextParams = Phi21851ModP444;
    type BgvParams = (Self::PlaintextParams, Self::CiphertextParams);
    type K = NativeResidue<128, 2>;
    type S = NativeResidue<64, 1>;
    type KS = NativeResidue<192, 3>;
}
