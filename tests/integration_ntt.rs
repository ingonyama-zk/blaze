use ingo_blaze::{driver_client::dclient::*, ingo_ntt::*};
use log::info;
use std::{env, error::Error};

#[test]
fn ntt_test() -> Result<(), Box<dyn Error>> {
    env_logger::try_init().expect("Invalid logger initialisation");
    let id = env::var("ID").unwrap_or_else(|_| 0.to_string());
    let buf_host = 0;
    let buf_kernel = 0;

    info!("Create Driver API instance");
    let dclient = DriverClient::new(&id, DriverConfig::driver_client_c1100_cfg());
    let driver = NTTClient::new(NTT::Ntt, dclient);
    log::info!("Starting set NTT data");
    driver.set_data(NTTInput {
        buf_host: buf_host,
        // data: vec![0; 0],
        fname: "test".to_string(),
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
    let res = driver.result(Some(buf_kernel))?;
    log::info!("NTT result: {:?}", res.unwrap().len());

    Ok(())
}
