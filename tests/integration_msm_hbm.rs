use crate::msm::RunResults;
use ingo_blaze::{driver_client::*, ingo_msm::*};
use num_traits::Pow;
use std::{
    env,
    thread::sleep,
    time::{Duration, Instant},
};

pub mod msm;

#[test]
fn hbm_msm_bls12_381_precomp_test() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::try_init().expect("Invalid logger initialisation");
    let id = env::var("ID").unwrap_or_else(|_| 0.to_string());
    let low_exp: u32 = 1;
    let max_exp: u32 = 1;
    let base = 2;

    log::debug!("Timer generation start");
    let start_gen = Instant::now();
    let (points, scalars, _, results) =
        msm::input_generator_bls12_381(Pow::pow(base, max_exp) as usize, PRECOMPUTE_FACTOR);
    let duration_gen = start_gen.elapsed();
    log::debug!("Time elapsed in input generation is: {:?}", duration_gen);

    let mut run_results: Vec<RunResults> = Vec::new();
    for iter in low_exp..=max_exp {
        let msm_size = Pow::pow(base, iter) as usize;
        log::debug!("MSM size: {}", msm_size);
        let mut points_to_run = vec![0; msm_size * 8 * 96];
        let mut scalars_to_run = vec![0; msm_size * 32];

        points_to_run.copy_from_slice(&points[0..msm_size * 8 * 96]);
        scalars_to_run.copy_from_slice(&scalars[0..msm_size * 32]);

        log::info!("Create Driver API instance");
        let dclient = DriverClient::new(&id, DriverConfig::driver_client_cfg(CardType::C1100));
        let driver = MSMClient::new(
            MSMInit {
                mem_type: PointMemoryType::DMA,
                is_precompute: true,
                curve: Curve::BLS381,
            },
            dclient,
        );
        driver.reset()?;

        let hbm_addr: u64 = 0x0;
        let offset: u64 = 0x0;
        // log::debug!("writing data to HBM");
        // let points_to_hbm = points_to_run.clone();
        // driver.load_data_to_hbm(&points_to_hbm, hbm_addr, offset)?;
        // let comp_points = driver.get_data_from_hbm(&points, hbm_addr, offset);
        // assert_eq!(points_to_hbm, comp_points.unwrap());
        // log::debug!("HBM Test OK");

        log::info!("Checking MSM core is ready: ");
        driver.is_msm_engine_ready()?;
        driver.task_label()?;
        driver.driver_client.firewalls_status();

        log::info!("Starting to initialize task and set number of elements: ");
        let msm_params = MSMParams {
            nof_elements: msm_size as u32,
            hbm_point_addr: Some((hbm_addr, offset)),
        };

        let _ = driver.initialize(msm_params);
        driver.driver_client.firewalls_status();

        log::info!("Starting to calculate MSM: ");
        log::debug!("Timer start");
        let start_set_data = Instant::now();
        let start_full = Instant::now();
        let _ = driver.set_data(MSMInput {
            points: None,
            scalars: scalars_to_run,
            params: msm_params,
        });
        driver.get_api();
        let dur_set = start_set_data.elapsed();
        let start_get = Instant::now();
        driver.driver_client.firewalls_status();
        log::info!("Waiting MSM result: ");
        driver.wait_result()?;
        let duration_wait = start_get.elapsed();

        let mres = driver.result(None).unwrap().unwrap();

        let duration = start_full.elapsed();

        let pos: usize = if msm_size <= 257 {
            msm_size
        } else if msm_size / 256 >= 256 && msm_size > 257 {
            results.len() - 1_usize
        } else {
            msm_size / 256
        };

        let (is_on_curve, is_eq) =
            msm::result_check_bls12_381(mres.result, results[pos - 1], results.clone(), msm_size);

        run_results.push(RunResults {
            msm_size,
            on_curve: is_on_curve,
            correct: is_eq,
            dur_set_data: dur_set,
            dur_get_result: duration_wait,
            dur_full_comput: duration,
        });
    }

    log::info!("RESULT: {:?}", run_results);
    sleep(Duration::from_secs(1));
    Ok(())
}

#[test]
fn hbm_msm_bls12_377_precomp_test() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::try_init().expect("Invalid logger initialisation");
    let id = env::var("ID").unwrap_or_else(|_| 0.to_string());
    let low_exp: u32 = 1;
    let max_exp: u32 = 1;
    let base = 2;

    log::debug!("Timer generation start");
    let start_gen = Instant::now();
    let (points, scalars, _, results) =
        msm::input_generator_bls12_377(Pow::pow(base, max_exp) as usize, PRECOMPUTE_FACTOR);
    let duration_gen = start_gen.elapsed();
    log::debug!("Time elapsed in input generation is: {:?}", duration_gen);

    let mut run_results: Vec<RunResults> = Vec::new();
    for iter in low_exp..=max_exp {
        let msm_size = Pow::pow(base, iter) as usize;
        log::debug!("MSM size: {}", msm_size);
        let mut points_to_run = vec![0; msm_size * 8 * 96];
        let mut scalars_to_run = vec![0; msm_size * 32];

        points_to_run.copy_from_slice(&points[0..msm_size * 8 * 96]);
        scalars_to_run.copy_from_slice(&scalars[0..msm_size * 32]);

        log::info!("Create Driver API instance");
        let dclient = DriverClient::new(&id, DriverConfig::driver_client_cfg(CardType::C1100));
        let driver = MSMClient::new(
            MSMInit {
                mem_type: PointMemoryType::HBM,
                is_precompute: true,
                curve: Curve::BLS377,
            },
            dclient,
        );
        driver.reset()?;

        // log::debug!("writing data to HBM");
        let hbm_addr: u64 = 0x0;
        let offset: u64 = 0x0;
        // let points_to_hbm = points_to_run.clone();
        // driver.load_data_to_hbm(&points_to_hbm, hbm_addr, offset)?;
        // let comp_points = driver.get_data_from_hbm(&points, hbm_addr, offset);
        // assert_eq!(points_to_hbm, comp_points.unwrap());
        // log::debug!("HBM Test OK");
        // driver.load_data_to_hbm(&points_to_hbm, hbm_addr, offset)?;

        log::info!("Checking MSM core is ready: ");
        driver.is_msm_engine_ready()?;
        driver.task_label()?;
        driver.driver_client.firewalls_status();

        log::info!("Starting to initialize task and set number of elements: ");
        let msm_params = MSMParams {
            nof_elements: msm_size as u32,
            hbm_point_addr: Some((hbm_addr, offset)),
        };

        let _ = driver.initialize(msm_params);
        driver.driver_client.firewalls_status();

        log::info!("Starting to calculate MSM: ");
        log::debug!("Timer start");
        let start_set_data = Instant::now();
        let start_full = Instant::now();
        let _ = driver.set_data(MSMInput {
            points: None,
            scalars: scalars_to_run,
            params: msm_params,
        });
        let dur_set = start_set_data.elapsed();
        let start_get = Instant::now();
        driver.driver_client.firewalls_status();
        log::info!("Waiting MSM result: ");
        driver.wait_result()?;
        let duration_wait = start_get.elapsed();

        let mres = driver.result(None).unwrap().unwrap();

        let duration = start_full.elapsed();

        let pos: usize = if msm_size <= 257 {
            msm_size
        } else if msm_size / 256 >= 256 && msm_size > 257 {
            results.len() - 1_usize
        } else {
            msm_size / 256
        };

        let (is_on_curve, is_eq) =
            msm::result_check_bls12_377(mres.result, results[pos - 1], results.clone(), msm_size);

        run_results.push(RunResults {
            msm_size,
            on_curve: is_on_curve,
            correct: is_eq,
            dur_set_data: dur_set,
            dur_get_result: duration_wait,
            dur_full_comput: duration,
        });
    }

    log::info!("RESULT: {:?}", run_results);
    sleep(Duration::from_secs(1));
    Ok(())
}
