use ark_bls12_377::{
    Fq as bls377Fq, Fr as bls377Fr, G1Affine as bls377G1Affine, G1Projective as bls377G1Projective,
};
use ark_bls12_381::{
    Fq as bls381Fq, Fr as bls381Fr, G1Affine as bls381G1Affine, G1Projective as bls381G1Projective,
};
use ark_bn254::{
    Fq as bn254Fq, Fr as bn254Fr, G1Affine as bn254G1Affine, G1Projective as bn254G1Projective,
};
use ark_ec::{AffineCurve, ProjectiveCurve};
use ark_ff::{BigInteger, Field, PrimeField, Zero};
use ark_std::UniformRand;

use ::std::ops::{Add, Mul};
use std::{fmt::Display, time::Duration};

const LARGE_PARAM: usize = 256;

fn get_large_param(nof_elements: usize) -> (bool, usize, usize, usize) {
    if nof_elements > LARGE_PARAM {
        let mult = nof_elements / LARGE_PARAM;
        let rest = nof_elements % LARGE_PARAM;
        (true, LARGE_PARAM, mult, rest)
    } else {
        (false, nof_elements, 0, 0)
    }
}

#[derive(Debug)]
pub struct RunResults {
    pub msm_size: usize,
    pub dur_set_data: Duration,
    pub dur_get_result: Duration,
    pub dur_full_comput: Duration,
    pub on_curve: bool,
    pub correct: bool,
}
impl Display for RunResults {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "MSM size: {}\nDuration while set data: {:?}\nDuration while get result: {:?}\nDuration all: {:?}\nOn curve: {}\nCorrect: {}",
            self.msm_size, self.dur_set_data,self.dur_get_result, self.dur_full_comput, self.on_curve, self.correct
        )
    }
}

pub fn input_generator_bls12_377(
    nof_elements: usize,
    precompute_factor: u32,
) -> (
    Vec<u8>,
    Vec<u8>,
    bls377G1Projective,
    Vec<bls377G1Projective>,
) {
    log::info!(
        "Generate bases with precompute factor: {}",
        precompute_factor
    );
    log::info!("Starting to generate bases and scalars on BLS12_377 curve...");
    let mut rng = ark_std::rand::thread_rng();
    let mut bases: Vec<u8> = Vec::new();
    let mut scalars: Vec<u8> = Vec::new();
    let mut msm_result = bls377G1Projective::zero();
    let mut results: Vec<bls377G1Projective> = Vec::new();

    let (is_large, nof_elems, mult, rest) = get_large_param(nof_elements);
    log::info!(
        "Generation Parameters: {}, {}, {}, {}",
        is_large,
        nof_elems,
        mult,
        rest
    );

    for _ in 0..nof_elems {
        let aff = bls377G1Projective::rand(&mut rng).into_affine();
        let ext = precompute_base_bls12_377(aff, precompute_factor);
        bases.extend(ext);

        let scalar = bls377Fr::rand(&mut rng);
        scalars.extend(scalar.into_repr().to_bytes_le());
        msm_result = msm_result.add(aff.mul(scalar));
        results.push(msm_result)
    }

    if is_large {
        let mut buf_base = bases.clone();
        let mut buf_scalar = scalars.clone();
        for _ in 0..mult - 1 {
            bases.extend(buf_base.clone());
            scalars.extend(buf_scalar.clone());
            msm_result = msm_result.add(msm_result);
        }

        buf_base.truncate(rest * 96 * 8);
        bases.extend(buf_base);
        buf_scalar.truncate(rest * 32);
        scalars.extend(buf_scalar);
        for r in results.iter().take(rest) {
            msm_result = msm_result.add(r);
        }
        results.push(msm_result);
    }
    log::info!("Done Generation...");

    (bases, scalars, msm_result, results)
}

pub fn precompute_base_bls12_377(base: bls377G1Affine, precompute_factor: u32) -> Vec<u8> {
    let mut bases = vec![];
    let mut current_point = base;
    let x_bytes = current_point.x.into_repr().to_bytes_le();
    let y_bytes = current_point.y.into_repr().to_bytes_le();
    bases.extend_from_slice(&x_bytes);
    bases.extend_from_slice(&y_bytes);
    let two = num_bigint::BigUint::from(2u32);

    let scalar_size_bls377= (LARGE_PARAM as f32/precompute_factor as f32).ceil() as u32;

    for i in 1..precompute_factor {
        current_point = base;
        let coeff = bls377Fr::from(two.pow(scalar_size_bls377 * i));
                current_point = current_point.mul(coeff).into_affine();
        let x_bytes = current_point.x.into_repr().to_bytes_le();
        let y_bytes = current_point.y.into_repr().to_bytes_le();
        bases.extend_from_slice(&x_bytes);
        bases.extend_from_slice(&y_bytes);
    }

    bases
}

pub fn result_check_bls12_377(
    result: Vec<u8>,
    msm_result: bls377G1Projective,
    results: Vec<bls377G1Projective>,
    nof_elements: usize,
) -> (bool, bool) {
    let mut msm_res: bls377G1Projective = bls377G1Projective::zero();
    let chunk = LARGE_PARAM;
    for _ in 0..nof_elements / chunk {
        msm_res = msm_res.add(results[results.len() - 2]);
    }
    if nof_elements % chunk > 0 && results.len() >= chunk {
        msm_res = msm_res.add(results[nof_elements % chunk - 1]);
    }

    let proj_x_field = bls377Fq::from_le_bytes_mod_order(&result[96..144]);
    let proj_y_field = bls377Fq::from_le_bytes_mod_order(&result[48..96]);
    let proj_z_field = bls377Fq::from_le_bytes_mod_order(&result[0..48]);

    let z_inv = proj_z_field.inverse().unwrap();
    let aff_x = proj_x_field.mul(z_inv);
    let aff_y = proj_y_field.mul(z_inv);

    let point = bls377G1Affine::new(aff_x, aff_y, false);
    log::debug!("Result affine point on BLS12_377: {:}", point.to_string());
    if nof_elements < chunk {
        log::debug!("Expected MSM result: {:}\n", msm_result.into_affine());
        (
            point.is_on_curve(),
            point.to_string() == msm_result.into_affine().to_string(),
        )
    } else {
        log::debug!("Expected MSM result: {:}\n", msm_res.into_affine());
        println!("{}     {}",point.to_string(), msm_res.into_affine().to_string());
        (
            point.is_on_curve(),
            point.to_string() == msm_res.into_affine().to_string(),
        )
    }
}

pub fn input_generator_bn254(
    nof_elements: usize,
    precompute_factor: u32,
) -> (Vec<u8>, Vec<u8>, bn254G1Projective, Vec<bn254G1Projective>) {
    log::info!(
        "Generate bases with precompute factor: {}",
        precompute_factor
    );
    log::info!("Starting to generate bases and scalars on BN254 curve...");
    let mut rng = ark_std::rand::thread_rng();
    let mut bases: Vec<u8> = Vec::new();
    let mut scalars: Vec<u8> = Vec::new();
    let mut msm_result = bn254G1Projective::zero();
    let mut results: Vec<bn254G1Projective> = Vec::new();

    let (is_large, nof_elems, mult, rest) = get_large_param(nof_elements);
    log::info!(
        "Generation Parameters: {}, {}, {}, {}",
        is_large,
        nof_elems,
        mult,
        rest
    );

    for _ in 0..nof_elems {
        let aff = bn254G1Projective::rand(&mut rng).into_affine();
        let ext = precompute_base_bn254(aff, precompute_factor);
        bases.extend(ext);

        let scalar = bn254Fr::rand(&mut rng);
        scalars.extend(scalar.into_repr().to_bytes_le());
        msm_result = msm_result.add(aff.mul(scalar));
        results.push(msm_result)
    }

    if is_large {
        let mut buf_base = bases.clone();
        let mut buf_scalar = scalars.clone();
        for _ in 0..mult - 1 {
            bases.extend(buf_base.clone());
            scalars.extend(buf_scalar.clone());
            msm_result = msm_result.add(msm_result);
        }

        buf_base.truncate(rest * 96 * 8);
        bases.extend(buf_base);
        buf_scalar.truncate(rest * 32);
        scalars.extend(buf_scalar);
        for r in results.iter().take(rest) {
            msm_result = msm_result.add(r);
        }
        results.push(msm_result);
    }
    log::info!("Done Generation...");

    (bases, scalars, msm_result, results)
}

pub fn precompute_base_bn254(base: bn254G1Affine, precompute_factor: u32) -> Vec<u8> {
    let mut bases = vec![];
    let mut current_point = base;
    let x_bytes = current_point.x.into_repr().to_bytes_le();
    let y_bytes = current_point.y.into_repr().to_bytes_le();
    bases.extend_from_slice(&x_bytes);
    bases.extend_from_slice(&y_bytes);
    let two = num_bigint::BigUint::from(2u32);

    let scalar_size_bn254= (LARGE_PARAM as f32/precompute_factor as f32).ceil() as u32;

    for i in 1..precompute_factor {
        current_point = base;
        let coeff = bn254Fr::from(two.pow(scalar_size_bn254 * i));
        current_point = current_point.mul(coeff).into_affine();
        let x_bytes = current_point.x.into_repr().to_bytes_le();
        let y_bytes = current_point.y.into_repr().to_bytes_le();
        bases.extend_from_slice(&x_bytes);
        bases.extend_from_slice(&y_bytes);
    }

    bases
}

pub fn result_check_bn254(
    result: Vec<u8>,
    msm_result: bn254G1Projective,
    results: Vec<bn254G1Projective>,
    nof_elements: usize,
) -> (bool, bool) {
    let mut msm_res = bn254G1Projective::zero();
    let chunk = LARGE_PARAM;
    for _ in 0..nof_elements / chunk {
        msm_res = msm_res.add(results[results.len() - 2]);
    }
    if nof_elements % chunk > 0 && results.len() >= chunk {
        msm_res = msm_res.add(results[nof_elements % chunk - 1]);
    }

    let proj_x_field = bn254Fq::from_le_bytes_mod_order(&result[96..144]);
    let proj_y_field = bn254Fq::from_le_bytes_mod_order(&result[48..96]);
    let proj_z_field = bn254Fq::from_le_bytes_mod_order(&result[0..48]);

    let z_inv = proj_z_field.inverse().unwrap();
    let aff_x = proj_x_field.mul(z_inv);
    let aff_y = proj_y_field.mul(z_inv);

    let point = bn254G1Affine::new(aff_x, aff_y, false);
    log::debug!("Result affine point on BN254: {:}", point.to_string());
    if nof_elements < chunk {
        log::debug!("Expected MSM result: {:}\n", msm_result.into_affine());
        (
            point.is_on_curve(),
            point.to_string() == msm_result.into_affine().to_string(),
        )
    } else {
        log::debug!("Expected MSM result: {:}\n", msm_res.into_affine());
        (
            point.is_on_curve(),
            point.to_string() == msm_res.into_affine().to_string(),
        )
    }
}

pub fn input_generator_bls12_381(
    nof_elements: usize,
    precompute_factor: u32,
) -> (
    Vec<u8>,
    Vec<u8>,
    bls381G1Projective,
    Vec<bls381G1Projective>,
) {
    log::info!(
        "Generate bases with precompute factor: {}",
        precompute_factor
    );
    log::info!("Starting to generate bases and scalars on BLS12_381 curve...");
    let mut rng = ark_std::rand::thread_rng();
    let mut bases: Vec<u8> = Vec::new();
    let mut scalars: Vec<u8> = Vec::new();
    let mut msm_result = bls381G1Projective::zero();
    let mut results: Vec<bls381G1Projective> = Vec::new();

    let (is_large, nof_elems, mult, rest) = get_large_param(nof_elements);
    log::info!(
        "Generation Parameters: {}, {}, {}, {}",
        is_large,
        nof_elems,
        mult,
        rest
    );

    for _ in 0..nof_elems {
        let aff = bls381G1Projective::rand(&mut rng).into_affine();
        let ext = precompute_base_bls12_381(aff, precompute_factor);
        bases.extend(ext);

        let scalar = bls381Fr::rand(&mut rng);
        scalars.extend(scalar.into_repr().to_bytes_le());
        msm_result = msm_result.add(aff.mul(scalar));
        results.push(msm_result)
    }

    if is_large {
        let mut buf_base = bases.clone();
        let mut buf_scalar = scalars.clone();
        for _ in 0..mult - 1 {
            bases.extend(buf_base.clone());
            scalars.extend(buf_scalar.clone());
            msm_result = msm_result.add(msm_result);
        }

        buf_base.truncate(rest * 96 * 8);
        bases.extend(buf_base);
        buf_scalar.truncate(rest * 32);
        scalars.extend(buf_scalar);
        for r in results.iter().take(rest) {
            msm_result = msm_result.add(r);
        }
        results.push(msm_result);
    }
    log::info!("Done Generation...");

    (bases, scalars, msm_result, results)
}

pub fn precompute_base_bls12_381(base: bls381G1Affine, precompute_factor: u32) -> Vec<u8> {
    let mut bases = vec![];
    let mut current_point = base;
    let x_bytes = current_point.x.into_repr().to_bytes_le();
    let y_bytes = current_point.y.into_repr().to_bytes_le();
    bases.extend_from_slice(&x_bytes);
    bases.extend_from_slice(&y_bytes);
    let two = num_bigint::BigUint::from(2u32);

    let scalar_size_bls381= (LARGE_PARAM as f32/precompute_factor as f32).ceil() as u32;

    for i in 1..precompute_factor {
        current_point = base;        
        let coeff = bls381Fr::from(two.pow(scalar_size_bls381 * i));
        current_point = current_point.mul(coeff).into_affine();
        let x_bytes = current_point.x.into_repr().to_bytes_le();
        let y_bytes = current_point.y.into_repr().to_bytes_le();
        bases.extend_from_slice(&x_bytes);
        bases.extend_from_slice(&y_bytes);
    }

    bases
}

pub fn result_check_bls12_381(
    result: Vec<u8>,
    msm_result: bls381G1Projective,
    results: Vec<bls381G1Projective>,
    nof_elements: usize,
) -> (bool, bool) {
    let mut msm_res = bls381G1Projective::zero();
    let chunk = LARGE_PARAM;
    for _ in 0..nof_elements / chunk {
        msm_res = msm_res.add(results[results.len() - 2]);
    }
    if nof_elements % chunk > 0 && results.len() >= chunk {
        msm_res = msm_res.add(results[nof_elements % chunk - 1]);
    }

    let proj_x_field = bls381Fq::from_le_bytes_mod_order(&result[96..144]);
    let proj_y_field = bls381Fq::from_le_bytes_mod_order(&result[48..96]);
    let proj_z_field = bls381Fq::from_le_bytes_mod_order(&result[0..48]);

    let z_inv = proj_z_field.inverse().unwrap();
    let aff_x = proj_x_field.mul(z_inv);
    let aff_y = proj_y_field.mul(z_inv);

    let point = bls381G1Affine::new(aff_x, aff_y, false);
    log::debug!("Result affine point on BLS12_381: {:}", point.to_string());
    if nof_elements < chunk {
        log::debug!("Expected MSM result: {:}\n", msm_result.into_affine());
        (
            point.is_on_curve(),
            point.to_string() == msm_result.into_affine().to_string(),
        )
    } else {
        log::debug!("Expected MSM result: {:}\n", msm_res.into_affine());
        (
            point.is_on_curve(),
            point.to_string() == msm_res.into_affine().to_string(),
        )
    }
}
