use std::thread::sleep;
use std::time::Duration;
use std::{ops::{Mul, Add}};

use num_bigint::BigUint;

use rust_rw_device::rw_msm_to_dram::*;

use ark_ec::{ProjectiveCurve, AffineCurve};
use ark_ff::{PrimeField, Field, BigInteger};
use rust_rw_device::curve::{G1Projective, G1Affine, Fr, Fq};
use ark_std::{UniformRand};

use pbr::ProgressBar;

use std::str::FromStr;

fn main(){
    println!("Generating MSM input ...");
    init();
    let test_npow = std::env::var("TEST_NPOW").unwrap_or("10".to_string());
    let n_points = i32::from_str(&test_npow).unwrap();
    let size = 1 << n_points; // 2 ^ n_points
    let (points, scalars, msm_result) = input_generator_biguint(size);
    let result = msm_calc_biguint(&points, &scalars, size);
    result_check_biguint(result.0, msm_result);
    sleep(Duration::from_secs(1));
}

fn result_check_biguint(result: [BigUint; 3], msm_result: G1Projective) {
    let proj_x_field = Fq::from_le_bytes_mod_order(&result[0].to_bytes_le());
    let proj_y_field = Fq::from_le_bytes_mod_order(&result[1].to_bytes_le());
    let proj_z_field = Fq::from_le_bytes_mod_order(&result[2].to_bytes_le());
    let aff_x = proj_x_field.mul(proj_z_field.inverse().unwrap());
    let aff_y = proj_y_field.mul(proj_z_field.inverse().unwrap());
    let point = G1Affine::new(aff_x, aff_y, false);
    println!("Is point on the curve {}",point.is_on_curve());
    println!("Is Result Equal To Expected {}", point.to_string() == msm_result.into_affine().to_string());
    assert_eq!(point, msm_result);
}

fn result_check_u32(result: [Vec<u32>; 3], msm_result: G1Projective) {
    let res_x: Vec<u8> = u32_vec_to_u8_vec(&result[0]);
    let res_y: Vec<u8> = u32_vec_to_u8_vec(&result[1]);
    let res_z: Vec<u8> = u32_vec_to_u8_vec(&result[2]);
    let proj_x_field = Fq::from_le_bytes_mod_order(&res_x);
    let proj_y_field = Fq::from_le_bytes_mod_order(&res_y);
    let proj_z_field = Fq::from_le_bytes_mod_order(&res_z);
    let aff_x = proj_x_field.mul(proj_z_field.inverse().unwrap());
    let aff_y = proj_y_field.mul(proj_z_field.inverse().unwrap());
    let point = G1Affine::new(aff_x, aff_y, false);
    println!("Is point on the curve {}",point.is_on_curve());
    println!("Is Result Equal To Expected {}", point.to_string() == msm_result.into_affine().to_string());
    assert_eq!(point, msm_result);
}

fn input_generator_biguint(nof_elements: usize) -> (Vec<BigUint>, Vec<BigUint>, G1Projective) {
    let mut rng = ark_std::rand::thread_rng();
    let mut points: Vec<BigUint> = Vec::new();
    let mut scalars: Vec<BigUint>  = Vec::new();
    let mut msm_result = G1Affine::prime_subgroup_generator().mul(0);
    let mut pb = ProgressBar::new(nof_elements.try_into().unwrap());
    pb.format("╢▌▌░╟");
    for _ in 0..nof_elements{
        pb.inc();
        let aff = G1Projective::rand(&mut rng).into_affine();
        points.push(BigUint::from_bytes_le(&aff.y.into_repr().to_bytes_le()));
        points.push(BigUint::from_bytes_le(&aff.x.into_repr().to_bytes_le()));
        let scalar = Fr::rand(&mut rng);
        scalars.push(BigUint::from_bytes_le(&scalar.into_repr().to_bytes_le()));
        msm_result = msm_result.add(aff.mul(scalar));
    }
    pb.finish_print("Done Generation...");
    (points, scalars, msm_result)
}

fn input_generator_u32(nof_elements: usize) -> (Vec<u32>, Vec<u32>, G1Projective) {
    let mut rng = ark_std::rand::thread_rng();
    let mut points: Vec<u32> = Vec::new();
    let mut scalars: Vec<u32>  = Vec::new();
    let mut msm_result = G1Affine::prime_subgroup_generator().mul(0);
    let mut pb = ProgressBar::new(nof_elements.try_into().unwrap());
    pb.format("╢▌▌░╟");
    for _ in 0..nof_elements{
        pb.inc();
        let aff = G1Projective::rand(&mut rng).into_affine();
        points.extend(as_u32_le(aff.y.into_repr().to_bytes_le()));
        points.extend(as_u32_le(aff.x.into_repr().to_bytes_le()));
        let scalar = Fr::rand(&mut rng);
        scalars.extend(as_u32_le(scalar.into_repr().to_bytes_le()));
        msm_result = msm_result.add(aff.mul(scalar));
    }
    pb.finish_print("Done Generation...");
    (points, scalars, msm_result)
}
