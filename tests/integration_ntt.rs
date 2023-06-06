use ingo_blaze::{driver_client::dclient::*, ingo_ntt::ntt_api::*};
use log::info;
use std::{env, error::Error};

mod ntt;

#[test]
fn loopback1_test() -> Result<(), Box<dyn Error>> {
    env_logger::try_init().expect("Invalid logger initialisation");
    let id = env::var("ID").unwrap_or_else(|_| 0.to_string());

    let fillename = env::var("CONFIG").unwrap_or_else(|_| "configs/ntt_cfg.json".to_string());
    let file = std::fs::File::open(fillename).expect("");
    let reader = std::io::BufReader::new(file);

    let ntt_cfg: NTTConfig = serde_json::from_reader(reader).unwrap();
    info!("ID: {}", id);
    info!("HBM bank size: {}", ntt_cfg.ntt_params.hbm_bank_size);
    info!(
        "Number of mmu: {}",
        ntt_cfg.ntt_params.nof_mmu_per_nttc * ntt_cfg.ntt_params.nof_nttc
    );

    info!("Create Driver API instance");
    let dclient = DriverClient::new(&id, DriverConfig::driver_client_c1100_cfg());
    let driver = NTTClient::new(NTT::Ntt, dclient);

    let buf_host = 0;
    // let buf_kernel = 0;

    let stage = 0;
    let group = 0;
    let filename = "/tmp/group.bin";

    driver.load_group(buf_host, filename, stage, group)?;
    let debug_input = NttInit {
        enable_debug_program: 0x00,
        debug_program: vec![
            0b0000000000000000000000000000000001111111,
            0b1111111100000000000000000000000000000000,
            0b1111111100000000000000000000000000000000,
            0b1111111100000000000000000000000000000000,
        ],
    };

    driver.initialize(debug_input)?;
    driver.start_process()?;
    driver.wait_result()?;

    Ok(())
}
