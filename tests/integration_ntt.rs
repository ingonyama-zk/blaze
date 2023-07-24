use ingo_blaze::{driver_client::*, ingo_ntt::*};
use log::info;
use std::{env, error::Error, fs::File, io::Read};

#[test]
fn ntt_test_correctness() -> Result<(), Box<dyn Error>> {
    env_logger::try_init().expect("Invalid logger initialisation");
    let id = env::var("ID").unwrap_or_else(|_| 0.to_string());

    let input_fname = env::var("INFNAME").unwrap();
    let mut in_f = File::open(input_fname).expect("no file found");
    let mut in_vec: Vec<u8> = Default::default();
    in_f.read_to_end(&mut in_vec)?;

    let output_fname = env::var("OUTFNAME").unwrap();
    let mut out_f = File::open(output_fname).expect("no file found");
    let mut out_vec: Vec<u8> = Default::default();
    out_f.read_to_end(&mut out_vec)?;

    let buf_host = 0;
    let buf_kernel = 0;

    info!("Create Driver API instance");
    let dclient = DriverClient::new(&id, DriverConfig::driver_client_cfg(CardType::U250));
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
        driver.reset()?;
        log::info!("Finishing NTT: {:?}", i);
    }

    log::info!("Try to get NTT result");
    let res = driver.result(Some(buf_kernel))?.unwrap();
    log::info!("Get NTT result of size: {:?}", res.len());
    assert_eq!(res, out_vec);

    Ok(())
}
