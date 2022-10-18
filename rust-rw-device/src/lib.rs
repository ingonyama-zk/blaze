pub mod rw_msm_to_dram;

#[cfg(feature = "bls12-377")]
pub use ark_bls12_377 as curve;

#[cfg(feature = "bn254")]
pub use ark_bn254 as curve;
