// Toy parameters for k=s=32
pub mod phi179_mod_p163;
pub mod phi179_mod_t64;
pub mod phi337_mod_p259;
pub mod phi337_mod_t86;

// Production parameters for k=s=32
pub mod phi21851_mod_p188;
pub mod phi21851_mod_t64;
pub mod phi43691_mod_p387;
pub mod phi43691_mod_t135;

// Production parameters for k=s=64
pub mod phi21851_mod_p316;
pub mod phi21851_mod_t128;
pub mod phi43691_mod_p616;
pub mod phi43691_mod_t233;

// Production parameters for k=128, s=64
pub mod phi21851_mod_p444;
pub mod phi21851_mod_t192;
pub mod phi43691_mod_p744;
pub mod phi43691_mod_t297;

use self::{phi337_mod_p259::Phi337ModP259, phi337_mod_t86::Phi337ModT86};

pub type ToyCipher = Phi337ModP259;
pub type ToyPlain = Phi337ModT86;
pub type ToyBgv = (ToyPlain, ToyCipher);
