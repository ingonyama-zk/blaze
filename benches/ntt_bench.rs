use std::{env, fs::File, io::Read};

use criterion::*;
use ingo_blaze::{driver_client::*, ingo_ntt::*};
use log::info;

fn bench_ntt_calc(c: &mut Criterion) {
    env_logger::try_init().expect("Invalid logger initialisation");
    let id = env::var("ID").unwrap_or_else(|_| 0.to_string());
    let fname = env::var("FNAME").unwrap();
    let mut f = File::open(fname).expect("no file found");
    let mut in_vec: Vec<u8> = Default::default();
    let _ = f.read_to_end(&mut in_vec);

    let buf_host = 0;
    let buf_kernel = 0;

    info!("Create Driver API instance");
    let dclient = DriverClient::new(&id, DriverConfig::driver_client_cfg(CardType::C1100));
    let driver = NTTClient::new(NTT::Ntt, dclient);
    log::info!("Starting set NTT data");
    let _ = driver.set_data(NTTInput {
        buf_host,
        data: in_vec,
    });
    log::info!("Successfully set NTT data");
    let _ = driver.driver_client.initialize_cms();
    let _ = driver.driver_client.reset_sensor_data();

    let mut group = c.benchmark_group("NTT computation");

    log::info!("Starting NTT");
    group.bench_function("NTT", |b| {
        b.iter(|| {
            let _ = driver.initialize(NttInit {});
            let _ = driver.start_process(Some(buf_kernel));
            let _ = driver.wait_result();
            let _ = driver.driver_client.reset();
        })
    });
    group.finish();
    log::info!("Finishing NTT");

    log::info!("Try to get NTT result");
    let res = driver.result(Some(buf_kernel)).unwrap();
    log::info!("NTT result: {:?}", res.unwrap().len());
}

criterion_group! {
    name = benches;
    config = Criterion::default().sample_size(10);
    targets = bench_ntt_calc
}
criterion_main!(benches);
