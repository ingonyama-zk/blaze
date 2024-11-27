extern crate ingo_blaze;

use ingo_blaze::{driver_client::*, ingo_ntt::*, utils};
use log::info;
use std::{env, error::Error, fs::File, io::Read};

#[test]
fn ntt_test_correctness() -> Result<(), Box<dyn Error>> {
    env_logger::try_init().expect("Invalid logger initialisation");
    let id = env::var("ID").unwrap_or_else(|_| 0.to_string());

    //let input_fname = env::var("INFNAME").unwrap();
    let input_fname = env::var("INFNAME").unwrap_or_else(|_| {
        "/home/administrator/ekaterina/blaze/tests/test_data/in_bin_00.dat".to_string()
    });

    let mut in_f = File::open(input_fname).expect("no file found");
    let mut in_vec: Vec<u8> = Default::default();
    in_f.read_to_end(&mut in_vec)?;

    //let output_fname = env::var("OUTFNAME").unwrap();
    let output_fname = env::var("OUTFNAME").unwrap_or_else(|_| {
        "/home/administrator/ekaterina/blaze/tests/test_data/ref_bin_00.dat".to_string()
    });
    let mut out_f = File::open(output_fname).expect("no file found");
    let mut out_vec: Vec<u8> = Default::default();
    out_f.read_to_end(&mut out_vec)?;

    let buf_host = 0;
    let buf_kernel = 0;

    info!("Create Driver API instance");
    let dclient = DriverClient::new(&id, DriverConfig::driver_client_cfg(CardType::C1100));
    //if std::env::var("BIN").is_ok() {
    //let bin_fname = std::env::var("BIN").unwrap();
    let bin_fname = "/home/administrator/eli/fpga-bin/ntt/user.bin";
    info!("Start reading binary");
    let bin = utils::read_binary_file(&bin_fname)?;
    info!("Start setup FPGA");
    dclient.setup_before_load_binary()?;
    info!("Start loading driver");
    dclient.load_binary(&bin)?;
    //}

    let driver = NTTClient::new(NTT::Ntt, dclient);
    log::info!("Starting set NTT data");
    driver.set_data(NTTInput {
        buf_host,
        data: in_vec,
    })?;
    log::info!("Successfully set NTT data");
    driver.driver_client.initialize_cms()?;
    driver.driver_client.reset_sensor_data()?;

    for i in 0..1 {
        log::info!("Starting NTT: {:?}", i);
        driver.initialize(NttInit {})?;
        driver.start_process(Some(buf_kernel))?;
        driver.wait_result()?;
        driver.driver_client.reset()?;
        log::info!("Finishing NTT: {:?}", i);
    }

    log::info!("Try to get NTT result");
    let res = driver.result(Some(buf_kernel))?.unwrap();
    log::info!("Get NTT result of size: {:?}", res.len());
    assert_eq!(res, out_vec);

    Ok(())
}

#[test]
fn ntt_parallel_test_correctness() -> Result<(), Box<dyn Error>> {
    env_logger::try_init().expect("Invalid logger initialisation");
    const NOF_VECTORS: usize = 3;
    let id = env::var("ID").unwrap_or_else(|_| 0.to_string());

    let in_dir = env::var("INDIR").unwrap();
    let mut in_vecs: Vec<Vec<u8>> = vec![Default::default(); NOF_VECTORS];

    for (i, in_vec) in in_vecs.iter_mut().enumerate().take(NOF_VECTORS) {
        let input_fname = format! {"{}/in_bin_{:02?}.dat", in_dir, i};
        let mut in_f = File::open(&input_fname).expect("no file found");
        in_f.read_to_end(in_vec)?;
        log::info!("Read input from file {:?}", input_fname);
    }

    let ref_dir = env::var("REFDIR").unwrap();
    let mut ref_vecs: Vec<Vec<u8>> = vec![Default::default(); NOF_VECTORS];
    for (i, ref_vec) in ref_vecs.iter_mut().enumerate().take(NOF_VECTORS) {
        let ref_fname = format! {"{}/ref_bin_{:02?}.dat", ref_dir, i};
        let mut ref_f = File::open(&ref_fname).expect("no file found");
        ref_f.read_to_end(ref_vec)?;
        log::info!("Read reference from file {:?}", ref_fname);
    }

    info!("Create Driver API instance");
    let dclient = DriverClient::new(&id, DriverConfig::driver_client_cfg(CardType::C1100));
    if std::env::var("BIN").is_ok() {
        let bin_fname = std::env::var("BIN").unwrap();
        info!("Start reading binary");
        let bin = utils::read_binary_file(&bin_fname)?;
        info!("Start setup FPGA");
        dclient.setup_before_load_binary()?;
        info!("Start loading driver");
        dclient.load_binary(&bin)?;
    }

    let driver = NTTClient::new(NTT::Ntt, dclient);
    driver.initialize(NttInit {})?;

    let mut outputs: Vec<Vec<u8>> = Vec::new();
    for i in 0..(NOF_VECTORS + 2) {
        let buf_host = i % 2;
        let buf_kernel = 1 - buf_host;
        info!("Cycle {}: host = {}, kernel = {}", i, buf_host, buf_kernel);
        log::info!("Starting process {:?} on kernel {:?}", i, buf_kernel);
        driver.start_process(Some(buf_kernel))?;

        log::info!("Try to get NTT result");
        let res = driver.result(Some(buf_host))?.unwrap();
        log::info!("Get NTT result of size: {:?}", res.len());
        if i >= 2 {
            log::info!("Save result {}", i - 2);
            outputs.push(res)
        }

        let host_wr_idx = i;
        let host_wr_idx_adj = if host_wr_idx > NOF_VECTORS - 1 {
            NOF_VECTORS - 1
        } else {
            host_wr_idx
        };
        log::info!(
            "Starting set NTT data with params: [{:?}, {:?}]",
            host_wr_idx,
            host_wr_idx_adj
        );
        driver.set_data(NTTInput {
            buf_host,
            data: in_vecs[host_wr_idx_adj].clone(),
        })?;
        log::info!("Successfully set NTT data");
        driver.wait_result()?;
        log::info!("Finishing Cycle: {:?}", i);
    }

    log::info!("Starting to check correctness");
    for (i, out_vec) in outputs.into_iter().enumerate() {
        log::info!("Checking output: {:?}", i);
        assert_eq!(out_vec, ref_vecs[i].clone());
        log::info!("Result {:?} correct", i);
    }

    Ok(())
}
