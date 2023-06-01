use ingo_blaze::{driver_client::dclient::*, ingo_msm::*, utils::*};
use num_traits::Pow;
use std::{
    env,
    fmt::Display,
    thread::sleep,
    time::{Duration, Instant},
};

pub mod msm;

#[test]
fn load_msm_binary_test() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::try_init().expect("Invalid logger initialisation");
    let id = env::var("ID").unwrap_or_else(|_| 0.to_string());
    let bin_file = env::var("FILENAME").unwrap();

    let msm_size = env::var("MSM_SIZE")
        .unwrap_or_else(|_| 8192.to_string())
        .parse::<u32>()
        .unwrap();

    log::info!("ID: {}", id);
    log::info!("MSM Size: {}", msm_size);

    log::info!("Create Driver API instance");
    let dclient = DriverClient::new(&id, DriverConfig::driver_client_c1100_cfg());
    let driver = msm_api::MSMClient::new(
        msm_api::MSMInit {
            mem_type: msm_api::PointMemoryType::DMA,
            is_precompute: false,
            curve: msm_api::Curve::BLS381,
        },
        dclient,
    );

    log::info!("Start to loading binary file...");
    let buf = read_binary_file(&bin_file)?;
    log::info!("Buffer size: {:?}", buf.len());

    driver.driver_client.firewalls_status();

    log::info!("Loading Binary File...");
    driver.driver_client.setup_before_load_binary()?;
    let ret = driver.driver_client.load_binary(buf.as_slice());
    log::info!("Load binary return HBICAP code: {:?}", ret);
    driver.driver_client.unblock_firewalls()?;

    driver.driver_client.firewalls_status();

    let params = driver.loaded_binary_parameters();
    log::info!(
        "Driver parameters: [{:?}, {:032b}]",
        params[0],
        params[1].reverse_bits()
    );
    let params_parce = msm_api::MSMImageParametrs::parse_image_params(params[1]);
    params_parce.debug_information();

    log::info!("Checking MSM core is ready: ");
    driver.is_msm_engine_ready()?;
    driver.task_label()?;

    let (points, scalars, msm_result, results) =
        msm::input_generator_bls12_381(msm_size as usize, msm_api::PRECOMPUTE_FACTOR_BASE);

    log::info!("Starting to initialize task and set number of elements: ");
    let msm_params = msm_api::MSMParams {
        nof_elements: msm_size,
        hbm_point_addr: None,
    };

    let _ = driver.initialize(msm_params);
    log::info!("Starting to calculate MSM: ");
    let _ = driver.set_data(msm_api::MSMInput {
        points: Some(points),
        scalars,
        params: msm_params,
    });
    driver.wait_result()?;
    let mres = driver.result(None).unwrap().unwrap();
    let (is_on_curve, is_eq) =
        msm::result_check_bls12_381(mres.result, msm_result, results, msm_size as usize);
    log::info!(
        "Is point on the {:?} curve {}",
        msm_api::Curve::BLS377,
        is_on_curve
    );
    log::info!("Is Result Equal To Expected {}", is_eq);
    assert!(is_on_curve);
    assert!(is_eq);
    sleep(Duration::from_secs(1));
    Ok(())
}

#[test]
fn msm_bls12_377_test() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::try_init().expect("Invalid logger initialisation");
    let id = env::var("ID").unwrap_or_else(|_| 0.to_string());
    let msm_size = env::var("MSM_SIZE")
        .unwrap_or_else(|_| 8192.to_string())
        .parse::<u32>()
        .unwrap();

    log::info!("Create Driver API instance");
    let dclient = DriverClient::new(&id, DriverConfig::driver_client_c1100_cfg());
    let driver = msm_api::MSMClient::new(
        msm_api::MSMInit {
            mem_type: msm_api::PointMemoryType::DMA,
            is_precompute: false,
            curve: msm_api::Curve::BLS377,
        },
        dclient,
    );

    let params = driver.loaded_binary_parameters();
    let params_parce = msm_api::MSMImageParametrs::parse_image_params(params[1]);
    params_parce.debug_information();
    log::info!("Checking MSM core is ready: ");
    driver.is_msm_engine_ready()?;
    driver.task_label()?;

    let (points, scalars, msm_result, results) =
        msm::input_generator_bls12_377(msm_size as usize, msm_api::PRECOMPUTE_FACTOR_BASE);

    log::info!("Starting to initialize task and set number of elements: ");
    let msm_params = msm_api::MSMParams {
        nof_elements: msm_size,
        hbm_point_addr: None,
    };

    let _ = driver.initialize(msm_params);
    log::info!("Starting to calculate MSM: ");
    let _ = driver.set_data(msm_api::MSMInput {
        points: Some(points),
        scalars,
        params: msm_params,
    });
    driver.wait_result()?;
    let mres = driver.result(None).unwrap().unwrap();
    let (is_on_curve, is_eq) =
        msm::result_check_bls12_377(mres.result, msm_result, results, msm_size as usize);
    log::info!(
        "Is point on the {:?} curve {}",
        msm_api::Curve::BLS377,
        is_on_curve
    );
    log::info!("Is Result Equal To Expected {}", is_eq);
    assert!(is_on_curve);
    assert!(is_eq);
    Ok(())
}

#[test]
fn msm_bls12_381_test() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::try_init().expect("Invalid logger initialisation");
    let id = env::var("ID").unwrap_or_else(|_| 0.to_string());
    let msm_size = env::var("MSM_SIZE")
        .unwrap_or_else(|_| 8192.to_string())
        .parse::<u32>()
        .unwrap();

    let (points, scalars, msm_result, results) =
        msm::input_generator_bls12_381(msm_size as usize, msm_api::PRECOMPUTE_FACTOR_BASE);

    log::info!("Create Driver API instance");
    let dclient = DriverClient::new(&id, DriverConfig::driver_client_c1100_cfg());
    let driver = msm_api::MSMClient::new(
        msm_api::MSMInit {
            mem_type: msm_api::PointMemoryType::DMA,
            is_precompute: false,
            curve: msm_api::Curve::BLS381,
        },
        dclient,
    );

    let params = driver.loaded_binary_parameters();
    let params_parce = msm_api::MSMImageParametrs::parse_image_params(params[1]);
    params_parce.debug_information();
    log::info!("Checking MSM core is ready: ");
    driver.is_msm_engine_ready()?;
    driver.task_label()?;
    driver.driver_client.firewalls_status();

    log::info!("Starting to initialize task and set number of elements: ");
    let msm_params = msm_api::MSMParams {
        nof_elements: msm_size,
        hbm_point_addr: None,
    };

    let _ = driver.initialize(msm_params);
    log::info!("Starting to calculate MSM: ");
    let _ = driver.set_data(msm_api::MSMInput {
        points: Some(points),
        scalars,
        params: msm_params,
    });
    driver.driver_client.firewalls_status();
    driver.task_label()?;
    log::info!("Waiting MSM result: ");
    driver.wait_result()?;
    let mres = driver.result(None).unwrap().unwrap();
    let (is_on_curve, is_eq) =
        msm::result_check_bls12_381(mres.result, msm_result, results, msm_size as usize);
    log::info!(
        "Is point on the {:?} curve {}",
        msm_api::Curve::BLS381,
        is_on_curve
    );
    log::info!("Is Result Equal To Expected {}", is_eq);
    assert!(is_on_curve);
    assert!(is_eq);
    sleep(Duration::from_secs(1));
    Ok(())
}

#[test]
fn msm_bn254_test() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::try_init().expect("Invalid logger initialisation");
    let id = env::var("ID").unwrap_or_else(|_| 0.to_string());
    let msm_size = env::var("MSM_SIZE")
        .unwrap_or_else(|_| 8192.to_string())
        .parse::<u32>()
        .unwrap();

    log::info!("Create Driver API instance");
    let dclient = DriverClient::new(&id, DriverConfig::driver_client_c1100_cfg());
    let driver = msm_api::MSMClient::new(
        msm_api::MSMInit {
            mem_type: msm_api::PointMemoryType::DMA,
            is_precompute: false,
            curve: msm_api::Curve::BN254,
        },
        dclient,
    );

    let params = driver.loaded_binary_parameters();
    let params_parce = msm_api::MSMImageParametrs::parse_image_params(params[1]);
    params_parce.debug_information();
    log::info!("Checking MSM core is ready: ");
    driver.is_msm_engine_ready()?;
    driver.task_label()?;

    let (points, scalars, msm_result, results) =
        msm::input_generator_bn254(msm_size as usize, msm_api::PRECOMPUTE_FACTOR_BASE);

    log::info!("Starting to initialize task and set number of elements: ");
    let msm_params = msm_api::MSMParams {
        nof_elements: msm_size,
        hbm_point_addr: None,
    };

    let _ = driver.initialize(msm_params);
    log::info!("Starting to calculate MSM: ");
    let _ = driver.set_data(msm_api::MSMInput {
        points: Some(points),
        scalars,
        params: msm_params,
    });
    driver.wait_result()?;
    let mres = driver.result(None).unwrap().unwrap();
    let (is_on_curve, is_eq) =
        msm::result_check_bn254(mres.result, msm_result, results, msm_size as usize);
    log::info!(
        "Is point on the {:?} curve {}",
        msm_api::Curve::BN254,
        is_on_curve
    );
    log::info!("Is Result Equal To Expected {}", is_eq);
    assert!(is_on_curve);
    assert!(is_eq);
    sleep(Duration::from_secs(1));
    Ok(())
}

#[derive(Debug)]
struct RunResults {
    msm_size: usize,
    dur_set_data: Duration,
    dur_wait_result: Duration,
    dur_full_comput: Duration,
    on_curve: bool,
    correct: bool,
}
impl Display for RunResults {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "MSM size: {}\nDuration while set data: {:?}\nDuration while get result: {:?}\nDuration all: {:?}\nOn curve: {}\nCorrect: {}",
            self.msm_size, self.dur_set_data,self.dur_wait_result, self.dur_full_comput, self.on_curve, self.correct
        )
    }
}

#[test]
fn msm_bls12_377_precompute_test() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::try_init().expect("Invalid logger initialisation");
    let id = env::var("ID").unwrap_or_else(|_| 0.to_string());
    let low_exp: u32 = 1;
    let max_exp: u32 = 1;
    let base = 2;

    log::debug!("Timer generation start");
    let start_gen = Instant::now();
    let (points, scalars, _, results) = msm::input_generator_bls12_377(
        Pow::pow(base, max_exp) as usize,
        msm_api::PRECOMPUTE_FACTOR,
    );
    let duration_gen = start_gen.elapsed();
    log::debug!("Time elapsed in input generation is: {:?}", duration_gen);

    let mut run_results: Vec<RunResults> = Vec::new();
    for iter in low_exp..=max_exp {
        let msm_size = Pow::pow(base, iter) as usize;
        log::debug!("MSM size: {}", msm_size);
        let mut points_to_run = vec![0; msm_size * 96 * msm_api::PRECOMPUTE_FACTOR as usize];
        let mut scalars_to_run = vec![0; msm_size * 32];

        points_to_run
            .copy_from_slice(&points[0..msm_size * 96 * msm_api::PRECOMPUTE_FACTOR as usize]);
        scalars_to_run.copy_from_slice(&scalars[0..msm_size * 32]);

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
        driver.reset()?;

        log::info!("Checking MSM core is ready: ");
        driver.is_msm_engine_ready()?;
        driver.task_label()?;
        driver.driver_client.firewalls_status();

        log::info!("Starting to initialize task and set number of elements: ");
        let msm_params = msm_api::MSMParams {
            nof_elements: msm_size as u32,
            hbm_point_addr: None,
        };
        let _ = driver.initialize(msm_params);

        driver.driver_client.firewalls_status();

        log::info!("Starting to calculate MSM: ");
        log::debug!("Timer start");
        let start_set_data = Instant::now();
        let start_full = Instant::now();
        let _ = driver.set_data(msm_api::MSMInput {
            points: Some(points_to_run),
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
        assert!(is_on_curve);
        assert!(is_eq);

        run_results.push(RunResults {
            msm_size,
            on_curve: is_on_curve,
            correct: is_eq,
            dur_set_data: dur_set,
            dur_wait_result: duration_wait,
            dur_full_comput: duration,
        });
    }

    log::info!("RESULT: {:?}", run_results);
    sleep(Duration::from_secs(1));
    Ok(())
}

#[test]
fn msm_bls12_377_precompute_max_test() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::try_init().expect("Invalid logger initialisation");
    let id = env::var("ID").unwrap_or_else(|_| 0.to_string());
    let msm_size = 67108864; // 2**26

    log::debug!("Timer start to generate test data");
    let start_gen = Instant::now();
    let (points, scalars, msm_result, results) =
        msm::input_generator_bls12_377(msm_size as usize, msm_api::PRECOMPUTE_FACTOR);
    let duration_gen = start_gen.elapsed();
    log::debug!("Time elapsed in generate test data is: {:?}", duration_gen);

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
    driver.reset()?;

    let params = driver.loaded_binary_parameters();
    let params_parce = msm_api::MSMImageParametrs::parse_image_params(params[1]);
    params_parce.debug_information();
    log::info!("Checking MSM core is ready: ");
    driver.is_msm_engine_ready()?;
    driver.task_label()?;
    driver.driver_client.firewalls_status();

    log::info!("Starting to initialize task and set number of elements: ");
    let msm_params = msm_api::MSMParams {
        nof_elements: msm_size,
        hbm_point_addr: None,
    };
    let _ = driver.initialize(msm_params);

    driver.driver_client.firewalls_status();
    driver.task_label()?;
    driver.nof_elements()?;

    log::info!("Starting to calculate MSM: ");
    log::debug!("Timer start");
    let start_set_data = Instant::now();
    let start_full = Instant::now();
    let _ = driver.set_data(msm_api::MSMInput {
        points: Some(points),
        scalars,
        params: msm_params,
    });
    let dur_set = start_set_data.elapsed();

    let start_wait = Instant::now();
    driver.driver_client.firewalls_status();
    log::info!("Waiting MSM result: ");
    driver.wait_result()?;

    let duration_wait = start_wait.elapsed();
    let mres = driver.result(None).unwrap().unwrap();
    let duration = start_full.elapsed();

    let (is_on_curve, is_eq) =
        msm::result_check_bls12_377(mres.result, msm_result, results, msm_size as usize);
    assert!(is_on_curve);
    assert!(is_eq);

    let time_res = RunResults {
        msm_size: msm_size as usize,
        on_curve: is_on_curve,
        correct: is_eq,
        dur_set_data: dur_set,
        dur_wait_result: duration_wait,
        dur_full_comput: duration,
    };

    log::info!("RESULT: {:?}", time_res);
    sleep(Duration::from_secs(1));
    Ok(())
}

#[test]
fn msm_bls12_381_precompute_test() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::try_init().expect("Invalid logger initialisation");
    let id = env::var("ID").unwrap_or_else(|_| 0.to_string());
    let low_exp: u32 = 16;
    let max_exp: u32 = 18;
    let base = 2;

    log::debug!("Timer generation start");
    let start_gen = Instant::now();
    let (points, scalars, _, results) = msm::input_generator_bls12_381(
        Pow::pow(base, max_exp) as usize,
        msm_api::PRECOMPUTE_FACTOR,
    );
    let duration_gen = start_gen.elapsed();
    log::debug!("Time elapsed in input generation is: {:?}", duration_gen);

    let mut run_results: Vec<RunResults> = Vec::new();
    for iter in low_exp..=max_exp {
        let msm_size = Pow::pow(base, iter) as usize;
        log::debug!("MSM size: {}", msm_size);
        let mut points_to_run = vec![0; msm_size * 96 * msm_api::PRECOMPUTE_FACTOR as usize];
        let mut scalars_to_run = vec![0; msm_size * 32];

        points_to_run
            .copy_from_slice(&points[0..msm_size * 96 * msm_api::PRECOMPUTE_FACTOR as usize]);
        scalars_to_run.copy_from_slice(&scalars[0..msm_size * 32]);

        log::info!("Create Driver API instance");
        let dclient = DriverClient::new(&id, DriverConfig::driver_client_c1100_cfg());
        let driver = msm_api::MSMClient::new(
            msm_api::MSMInit {
                mem_type: msm_api::PointMemoryType::DMA,
                is_precompute: true,
                curve: msm_api::Curve::BLS381,
            },
            dclient,
        );
        driver.reset()?;

        log::info!("Checking MSM core is ready: ");
        driver.is_msm_engine_ready()?;
        driver.task_label()?;
        // driver.driver_client.firewalls_status();

        log::info!("Starting to initialize task and set number of elements: ");
        let msm_params = msm_api::MSMParams {
            nof_elements: msm_size as u32,
            hbm_point_addr: None,
        };
        let _ = driver.initialize(msm_params);

        driver.driver_client.firewalls_status();

        log::info!("Starting to calculate MSM: ");
        log::debug!("Timer start");
        let start_set_data = Instant::now();
        let start_full = Instant::now();
        let _ = driver.set_data(msm_api::MSMInput {
            points: Some(points_to_run),
            scalars: scalars_to_run,
            params: msm_params,
        });
        // driver.get_api();
        let dur_set = start_set_data.elapsed();
        let start_get = Instant::now();
        driver.driver_client.firewalls_status();
        log::info!("Waiting MSM result: ");
        driver.wait_result()?;
        let duration_wait = start_get.elapsed();

        let mres = driver.result(None).unwrap().unwrap();

        let duration = start_full.elapsed();
        // driver.get_api();
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
            dur_wait_result: duration_wait,
            dur_full_comput: duration,
        });
    }

    for rr in run_results.iter() {
        log::info!("{}", rr)
    }
    sleep(Duration::from_secs(1));
    Ok(())
}

#[test]
fn msm_bls12_381_precompute_max_test() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::try_init().expect("Invalid logger initialisation");
    let id = env::var("ID").unwrap_or_else(|_| 0.to_string());
    let msm_size = 67108864; // 2**26

    log::debug!("Timer start to generate test data");
    let start_gen = Instant::now();
    let (points, scalars, msm_result, results) =
        msm::input_generator_bls12_381(msm_size as usize, msm_api::PRECOMPUTE_FACTOR);
    let duration_gen = start_gen.elapsed();
    log::debug!("Time elapsed in generate test data is: {:?}", duration_gen);

    log::info!("Create Driver API instance");
    let dclient = DriverClient::new(&id, DriverConfig::driver_client_c1100_cfg());
    let driver = msm_api::MSMClient::new(
        msm_api::MSMInit {
            mem_type: msm_api::PointMemoryType::DMA,
            is_precompute: true,
            curve: msm_api::Curve::BLS381,
        },
        dclient,
    );
    driver.reset()?;

    let params = driver.loaded_binary_parameters();
    let params_parce = msm_api::MSMImageParametrs::parse_image_params(params[1]);
    params_parce.debug_information();
    log::info!("Checking MSM core is ready: ");
    driver.is_msm_engine_ready()?;
    driver.task_label()?;
    driver.driver_client.firewalls_status();

    log::info!("Starting to initialize task and set number of elements: ");
    let msm_params = msm_api::MSMParams {
        nof_elements: msm_size,
        hbm_point_addr: None,
    };
    let _ = driver.initialize(msm_params);

    driver.driver_client.firewalls_status();
    driver.task_label()?;
    driver.nof_elements()?;

    log::info!("Starting to calculate MSM: ");
    log::debug!("Timer start");
    let start_set_data = Instant::now();
    let start_full = Instant::now();
    let _ = driver.set_data(msm_api::MSMInput {
        points: Some(points),
        scalars,
        params: msm_params,
    });
    let dur_set = start_set_data.elapsed();

    let start_wait = Instant::now();
    driver.driver_client.firewalls_status();
    log::info!("Waiting MSM result: ");
    driver.wait_result()?;

    let duration_wait = start_wait.elapsed();
    let mres = driver.result(None).unwrap().unwrap();
    let duration = start_full.elapsed();

    let (is_on_curve, is_eq) =
        msm::result_check_bls12_381(mres.result, msm_result, results, msm_size as usize);
    assert!(is_on_curve);
    assert!(is_eq);

    let time_res = RunResults {
        msm_size: msm_size as usize,
        on_curve: is_on_curve,
        correct: is_eq,
        dur_set_data: dur_set,
        dur_wait_result: duration_wait,
        dur_full_comput: duration,
    };

    log::info!("RESULT: {:?}", time_res);
    sleep(Duration::from_secs(1));
    Ok(())
}
