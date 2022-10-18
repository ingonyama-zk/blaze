use std::str::FromStr;

use ark_bls12_377::G1Affine;
use ark_ff::{BigInteger256, PrimeField};
use num_bigint::BigUint;

use ingo_x::util;

#[test]
pub fn msm_correctness() {
    let test_npow = std::env::var("TEST_NPOW").unwrap_or("23".to_string());
    let n_points = i32::from_str(&test_npow).unwrap();

    let (points, scalars) = util::generate_points_scalars::<G1Affine>(1usize << n_points);

    let msm_ark_projective = ingo_x::msm_ark(&points, unsafe {
        std::mem::transmute::<&[_], &[BigInteger256]>(scalars.as_slice())
    });

    let mut msm_ark_vec: Vec<BigUint> = vec![];
    msm_ark_vec.push(msm_ark_projective.x.into_repr().into());
    msm_ark_vec.push(msm_ark_projective.y.into_repr().into());
    msm_ark_vec.push(msm_ark_projective.z.into_repr().into());

    let points_as_big_int = points.into_iter()
        .map(|point| [point.x.into_repr().into(), point.y.into_repr().into()])
        .flatten()
        .collect::<Vec<BigUint>>();

    let scalar_as_big_int = scalars.into_iter()
        .map(|scalar| scalar.into_repr().into())
        .collect::<Vec<BigUint>>();


    let msm_cloud_vec = ingo_x::msm_cloud::<G1Affine>(&points_as_big_int, &scalar_as_big_int);
    assert_eq!(msm_ark_vec, msm_cloud_vec.0)
}