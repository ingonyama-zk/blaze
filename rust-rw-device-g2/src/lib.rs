use std::{ops::{Mul, Add}};

use num_bigint::BigUint;

use ark_ec::{ProjectiveCurve, AffineCurve};
use ark_ff::{PrimeField, Field, BigInteger, Zero, QuadExtField};
use ark_bn254::{G2Projective as G2Projective, G2Affine as G2Affine, Fr, Fq};
use ark_std::{UniformRand};
use pbr::ProgressBar;

pub mod rw_msm_to_dram;

pub fn result_check_biguint(result: [Vec<u8>; 6], msm_result:G2Affine) {
    let proj_x_field_c0 = Fq::from_le_bytes_mod_order(&result[5]);
    let proj_x_field_c1 = Fq::from_le_bytes_mod_order(&result[2]);
    let proj_x_field = QuadExtField::new(proj_x_field_c0,proj_x_field_c1);
    let proj_y_field_c0 = Fq::from_le_bytes_mod_order(&result[4]);
    let proj_y_field_c1 = Fq::from_le_bytes_mod_order(&result[1]);
    let proj_y_field = QuadExtField::new(proj_y_field_c0,proj_y_field_c1);
    let proj_z_field_c0 = Fq::from_le_bytes_mod_order(&result[3]);
    let proj_z_field_c1 = Fq::from_le_bytes_mod_order(&result[0]);
    let proj_z_field = QuadExtField::new(proj_z_field_c0,proj_z_field_c1);

    let aff_x = proj_x_field.mul(proj_z_field.inverse().unwrap());
    let aff_y = proj_y_field.mul(proj_z_field.inverse().unwrap());
    let point = G2Affine::new(aff_x,aff_y,false);
    
    println!("point.x Re bytes {:02X?}", &point.x.c0.into_repr().to_bytes_le());
    println!("point.x Re bytes {:02X?}", &point.x.c1.into_repr().to_bytes_le());
    println!("point.y Re bytes {:02X?}", &point.y.c0.into_repr().to_bytes_le());
    println!("point.y Im bytes {:02X?}", &point.y.c1.into_repr().to_bytes_le());


    println!("Is point on the curve {}",point.is_on_curve());
    println!("Is Result Equal To Expected {}", point.to_string() == msm_result.to_string());
}

pub fn input_generator_biguint(nof_elements: usize) -> (Vec<BigUint>, Vec<BigUint>, G2Affine, Vec<G2Affine>, Vec<Fr>) {
    let mut rng = ark_std::rand::thread_rng();
    let mut points: Vec<BigUint> = Vec::new();
    let mut scalars: Vec<BigUint>  = Vec::new();
    let mut points_ga: Vec<G2Affine> = Vec::new();
    let mut scalars_fr: Vec<Fr>  = Vec::new();
    let mut msm_result = G2Affine::zero();
    let mut pb = ProgressBar::new(nof_elements.try_into().unwrap());
    pb.format("╢▌▌░╟");
    for _ in 0..nof_elements{
        pb.inc();
        let aff = G2Projective::rand(&mut rng).into_affine();

        points.push(BigUint::from_bytes_le(&aff.y.c1.into_repr().to_bytes_le()));
        points.push(BigUint::from_bytes_le(&aff.x.c1.into_repr().to_bytes_le()));
        points.push(BigUint::from_bytes_le(&aff.y.c0.into_repr().to_bytes_le()));
        points.push(BigUint::from_bytes_le(&aff.x.c0.into_repr().to_bytes_le()));

        let scalar = Fr::from_le_bytes_mod_order(&Fr::rand(&mut rng).into_repr().to_bytes_le()[0..31]);
        scalars.push(BigUint::from_bytes_le(&scalar.into_repr().to_bytes_le()));
        
        if msm_result.is_zero(){
            msm_result = aff.mul(scalar).into_affine();
        }
        else{
            msm_result = msm_result.add(aff.mul(scalar).into_affine());
        }

        points_ga.push(aff);
        scalars_fr.push(scalar);
    }
    pb.finish_print("Done Generation...");

    println!("msm_result.y Im bytes {:02X?}", &msm_result.y.c1.into_repr().to_bytes_le());
    println!("msm_result.x Im bytes {:02X?}", &msm_result.x.c1.into_repr().to_bytes_le());
    println!("msm_result.y Re bytes {:02X?}", &msm_result.y.c0.into_repr().to_bytes_le());
    println!("msm_result.x Re bytes {:02X?}", &msm_result.x.c0.into_repr().to_bytes_le());
    println!("Point on curve {}", msm_result.is_on_curve());

    (points, scalars, msm_result, points_ga, scalars_fr)
}
