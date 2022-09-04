use std::thread::sleep;
use std::time::Duration;
use std::{ops::{Mul, Add}};

use num_bigint::BigUint;

use crate::rw_msm_to_dram::*;

mod rw_msm_to_dram;

use ark_ec::{ProjectiveCurve, AffineCurve};
use ark_ff::{PrimeField, Field, BigInteger};
use ark_bls12_377::{G1Projective as G, G1Affine as GAffine, Fr, g1, Fq};
use ark_std::{UniformRand};

use pbr::ProgressBar;

fn main(){
    println!("Generating MSM input ...");
    init();
    let size = 1024; //2^10
    let tests: usize = 12;
    let (points, scalars, msm_result) = input_generator_biguint(size);
    let result = msm_calc_biguint(&points, &scalars, size);
    result_check_biguint(result.0, msm_result);
    sleep(Duration::from_secs(1));
    let (points, scalars, msm_result) = input_generator_u32(size);
    for _ in 0.. tests{
    let result = msm_calc_u32(&points, &scalars, size);
    result_check_u32(result.0, msm_result);
    sleep(Duration::from_secs(1));
    }
    // let (points, scalars, msm_result) = input_generator_u32(size);
    // write_points_to_hbm(&points,size);
    // let result = msm_calc_u32_only_scalars(&scalars, size);
    // result_check_u32(result.0, msm_result);
    // sleep(Duration::from_secs(1));
}

fn result_check_biguint(result: [BigUint; 3], msm_result: ark_ec::short_weierstrass_jacobian::GroupProjective<g1::Parameters>) {
    let proj_x_field = Fq::from_le_bytes_mod_order(&result[0].to_bytes_le());
    let proj_y_field = Fq::from_le_bytes_mod_order(&result[1].to_bytes_le());
    let proj_z_field = Fq::from_le_bytes_mod_order(&result[2].to_bytes_le());
    let aff_x = proj_x_field.mul(proj_z_field.inverse().unwrap());
    let aff_y = proj_y_field.mul(proj_z_field.inverse().unwrap());
    let point = GAffine::new(aff_x, aff_y, false);
    println!("Is point on the curve {}",point.is_on_curve());
    println!("Is Result Equal To Expected {}", point.to_string() == msm_result.into_affine().to_string());
}

fn result_check_u32(result: [Vec<u32>; 3], msm_result: ark_ec::short_weierstrass_jacobian::GroupProjective<g1::Parameters>) {
    let res_x: Vec<u8> = u32_vec_to_u8_vec(&result[0]);
    let res_y: Vec<u8> = u32_vec_to_u8_vec(&result[1]);
    let res_z: Vec<u8> = u32_vec_to_u8_vec(&result[2]);
    let proj_x_field = Fq::from_le_bytes_mod_order(&res_x);
    let proj_y_field = Fq::from_le_bytes_mod_order(&res_y);
    let proj_z_field = Fq::from_le_bytes_mod_order(&res_z);
    let aff_x = proj_x_field.mul(proj_z_field.inverse().unwrap());
    let aff_y = proj_y_field.mul(proj_z_field.inverse().unwrap());
    let point = GAffine::new(aff_x, aff_y, false);
    println!("Is point on the curve {}",point.is_on_curve());
    println!("Is Result Equal To Expected {}", point.to_string() == msm_result.into_affine().to_string());
}

fn input_generator_biguint(nof_elements: usize) -> (Vec<BigUint>, Vec<BigUint>, ark_ec::short_weierstrass_jacobian::GroupProjective<g1::Parameters>) {
    let mut rng = ark_std::rand::thread_rng();
    let mut points: Vec<BigUint> = Vec::new();
    let mut scalars: Vec<BigUint>  = Vec::new();
    let mut msm_result = GAffine::prime_subgroup_generator().mul(0);
    let mut pb = ProgressBar::new(nof_elements.try_into().unwrap());
    pb.format("╢▌▌░╟");
    for _ in 0..nof_elements{
        pb.inc();
        let aff = G::rand(&mut rng).into_affine();
        points.push(BigUint::from_bytes_le(&aff.y.into_repr().to_bytes_le()));
        points.push(BigUint::from_bytes_le(&aff.x.into_repr().to_bytes_le()));
        let scalar = Fr::rand(&mut rng);
        scalars.push(BigUint::from_bytes_le(&scalar.into_repr().to_bytes_le()));
        msm_result = msm_result.add(aff.mul(scalar));
    }
    pb.finish_print("Done Generation...");
    (points, scalars, msm_result)
}

fn input_generator_u32(nof_elements: usize) -> (Vec<u32>, Vec<u32>, ark_ec::short_weierstrass_jacobian::GroupProjective<g1::Parameters>) {
    let mut rng = ark_std::rand::thread_rng();
    let mut points: Vec<u32> = Vec::new();
    let mut scalars: Vec<u32>  = Vec::new();
    let mut msm_result = GAffine::prime_subgroup_generator().mul(0);
    let mut pb = ProgressBar::new(nof_elements.try_into().unwrap());
    pb.format("╢▌▌░╟");
    for _ in 0..nof_elements{
        pb.inc();
        let aff = G::rand(&mut rng).into_affine();
        points.extend(as_u32_le(aff.y.into_repr().to_bytes_le()));
        points.extend(as_u32_le(aff.x.into_repr().to_bytes_le()));
        let scalar = Fr::rand(&mut rng);
        scalars.extend(as_u32_le(scalar.into_repr().to_bytes_le()));
        msm_result = msm_result.add(aff.mul(scalar));
    }
    pb.finish_print("Done Generation...");
    (points, scalars, msm_result)
}
