//! This Rust package offers a basic AXI infrastructure and the ability to work with user logic
//! through custom modules.
//! The custom modules provided are designed for [MSM](crate::ingo_msm) and
//! [Poseidon hash](crate::ingo_hash) and allow for the loading of user
//! logic onto an FPGA. These modules simplify the interaction with the user logic,
//! making it easier to develop efficient FPGA designs.
//!
pub mod driver_client;
pub mod error;
pub mod ingo_hash;
pub mod ingo_msm;
pub mod utils;
