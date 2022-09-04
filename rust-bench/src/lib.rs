use ark_ec::AffineCurve;
use ark_ec::msm::VariableBaseMSM;
use ark_ff::PrimeField;
use num_bigint::BigUint;

use rust_rw_device::rw_msm_to_dram;

pub mod util;

pub fn msm_ark<G: AffineCurve>(
    points: &[G],
    scalars: &[<G::ScalarField as PrimeField>::BigInt],
) -> G::Projective {
    let npoints = points.len();
    if npoints != scalars.len() {
        panic!("length mismatch")
    }

    let ret = VariableBaseMSM::multi_scalar_mul(points, scalars);
    ret
}

pub fn msm_cloud<G: AffineCurve>(
    points: &Vec<BigUint>,
    scalars: &Vec<BigUint>,
) -> ([BigUint; 3], u8) {
    let npoints = points.len() / 2;
    if npoints != scalars.len() {
        panic!("length mismatch")
    }
    let ret  = rw_msm_to_dram::msm_calc_biguint(&points, &scalars, scalars.len());
    let result:([BigUint; 3], u8) = (ret.0, ret.2);
    return result;
}
