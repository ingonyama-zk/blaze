use ark_ec::{AffineCurve, ProjectiveCurve};
use ark_ff::{BigInteger, Field, PrimeField, Zero};
use ark_std::UniformRand;
use criterion::*;
use ingo_blaze::{driver_client::dclient::*, ingo_msm::*};
use num_traits::Pow;
use std::{env, time::Instant};

use ::std::ops::{Add, Mul};
use ark_bls12_377::{
    Fq as bls377Fq, Fr as bls377Fr, G1Affine as bls377G1Affine, G1Projective as bls377G1Projective,
};

const SCALAR_SIZE_BLS377: u32 = 32;
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

fn bench_throughput_read(c: &mut Criterion) {
    env_logger::try_init().expect("Invalid logger initialisation");
    let id = env::var("ID").unwrap_or_else(|_| 0.to_string());

    let low_exp: u32 = 10;
    let max_exp: u32 = 20;
    let base = 2;

    log::debug!("Timer generation start");
    let start_gen = Instant::now();
    let (points, scalars, _, _) =
        input_generator_bls12_377(Pow::pow(base, max_exp) as usize, msm_api::PRECOMPUTE_FACTOR);
    let duration_gen = start_gen.elapsed();
    log::debug!("Time elapsed in input generation is: {:?}", duration_gen);

    log::info!("Create Driver API instance");
    let dclient = DriverClient::new(&id, DriverConfig::driver_client_c1100_cfg());
    let driver = msm_api::MSMClient::new(
        msm_api::MSMInit {
            mem_type: msm_api::PointMemoryType::DMA,
            is_precompute: true,
            curve: msm_api::Curve::BLS377,
        },
        dclient,
    );

    for iter in low_exp..=max_exp {
        let msm_size = Pow::pow(base, iter) as usize;
        log::debug!("MSM size: {}", msm_size);
        let mut points_to_run = vec![0; msm_size * 8 * 96];
        let mut scalars_to_run = vec![0; msm_size * 32];

        points_to_run.copy_from_slice(&points[0..msm_size * 8 * 96]);
        scalars_to_run.copy_from_slice(&scalars[0..msm_size * 32]);

        let mut group = c.benchmark_group("MSM computation");
        group.bench_function(format!("MSM of size: {}", msm_size), |b| {
            b.iter(|| {
                let _ = driver.is_msm_engine_ready();

                let msm_params = msm_api::MSMParams {
                    nof_elements: msm_size as u32,
                    hbm_point_addr: None,
                };
                let _ = driver.initialize(msm_params);

                let _ = driver.set_data(msm_api::MSMInput {
                    points: Some(points_to_run.clone()),
                    scalars: scalars_to_run.clone(),
                    params: msm_params,
                });

                let _ = driver.wait_result();
                let _ = driver.result(None).unwrap().unwrap();
                // let pos: usize = if msm_size <= 257 {
                //     msm_size
                // } else if msm_size / 256 >= 256 && msm_size > 257 {
                //     results.len() - 1_usize
                // } else {
                //     msm_size / 256
                // };

                // let (is_on_curve, is_eq) =
                //     result_check_bls12_377(mres.result, results[pos - 1], results.clone(), msm_size);
                // assert!(is_on_curve);
                // assert!(is_eq);
            })
        });

        group.finish();
        let _ = driver.reset();
    }
}

criterion_group! {
    name = benches;
    config = Criterion::default().sample_size(1000);
    targets = bench_throughput_read
}
criterion_main!(benches);

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
    for i in 1..precompute_factor {
        current_point = base;
        let coeff = bls377Fr::from(two.clone().pow(SCALAR_SIZE_BLS377 * i));
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
        (
            point.is_on_curve(),
            point.to_string() == msm_res.into_affine().to_string(),
        )
    }
}
