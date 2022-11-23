use ark_ec::{AffineCurve, ProjectiveCurve};
use ark_ff::{BigInteger256, Field, PrimeField, Zero};
use num_bigint::BigUint;
use rust_rw_device::curve::{Fq, G1Affine, G1Projective};
use std::{
    ops::{Add, Mul},
    str::FromStr,
};

use ingo_x::util;

#[test]
pub fn msm_correctness() {
    let test_npow = std::env::var("TEST_NPOW").unwrap_or("11".to_string());
    let n_points = i32::from_str(&test_npow).unwrap();

    let len = 1 << n_points;
    let (points, scalars) = util::generate_points_scalars::<G1Affine>(len);

    let msm_ark_projective = ingo_x::msm_ark(
        &points,
        &scalars
            .to_vec()
            .into_iter()
            .map(|s| BigInteger256::try_from(s).unwrap())
            .collect::<Vec<BigInteger256>>(), //this is safe but slow conversion
    );

    let mut msm_result_cpu_ingo_ref = G1Projective::zero(); //TODO: same as G1Affine::prime_subgroup_generator().mul(0);
    let mut msm_result_cpu_ref1 = G1Projective::zero();
    for i in 0..len {
        msm_result_cpu_ingo_ref = msm_result_cpu_ingo_ref.add(points[i].mul(scalars[i]));
        msm_result_cpu_ref1 =
            msm_result_cpu_ref1.add_mixed(&points[i].mul(scalars[i]).into_affine());
    }

    assert_eq!(msm_result_cpu_ingo_ref, msm_result_cpu_ref1);
    assert_eq!(msm_result_cpu_ingo_ref, msm_ark_projective);

    let points_as_big_int = points.into_iter()
        .map(|point| [point.y.into_repr().into(), point.x.into_repr().into()])
        .flatten()
        .collect::<Vec<BigUint>>();

    let scalar_as_big_int = scalars.into_iter()
        .map(|scalar| scalar.into_repr().into())
        .collect::<Vec<BigUint>>();

    let msm_cloud_vec = ingo_x::msm_cloud::<G1Affine>(&points_as_big_int, &scalar_as_big_int);

    let result = msm_cloud_vec.0;

    let proj_x_field = Fq::from_le_bytes_mod_order(&result[0].to_bytes_le());
    let proj_y_field = Fq::from_le_bytes_mod_order(&result[1].to_bytes_le());
    let proj_z_field = Fq::from_le_bytes_mod_order(&result[2].to_bytes_le());
    let z_inverse = proj_z_field.inverse().unwrap();
    let aff_x = proj_x_field.mul(z_inverse);
    let aff_y = proj_y_field.mul(z_inverse);
    let cloud_aff_point = G1Affine::new(aff_x, aff_y, false);

    assert_eq!(cloud_aff_point, msm_ark_projective); //raw vec comparison isn't always meaningful
}
