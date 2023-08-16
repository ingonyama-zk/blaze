use std::{
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

use dotenv::dotenv;
use ingo_blaze::{
    driver_client::*,
    ingo_hash::{
        num_of_elements_in_base_layer, Hash, PoseidonClient, PoseidonInitializeParameters,
        PoseidonResult, TreeMode,
    },
};
use log::info;
use num::{BigUint, Num};

fn get_instruction_path() -> String {
    dotenv().ok();
    std::env::var("INSTRUCTION_PATH").expect("INSTRUCTION_PATH must be set.")
}

const TREE_HEIGHT_4_NUM_OF_NODES: usize = 585;
const TEST_SCALAR: &str =
    "15338226384362629345253584946022322145063321004547266825580649561525819500264";
const ZERO: u32 = 0;
const ONE: u32 = 1;

#[test]
fn test_sanity_check() {
    let instruction_path = get_instruction_path();

    let dclient = DriverClient::new("0", DriverConfig::driver_client_cfg(CardType::U250));
    let poseidon: PoseidonClient = PoseidonClient::new(Hash::Poseidon, dclient);

    poseidon.dclient.reset().expect_err("Failed while reset");
    let params = poseidon.loaded_binary_parameters();
    info!("Driver parameters: [{:?}, {:032b}]", params[0], params[1]);

    // assertion is inside function
    let _ = poseidon.initialize(PoseidonInitializeParameters {
        tree_height: 8,
        tree_mode: TreeMode::TreeC,
        instruction_path,
    });

    let _ = poseidon.set_data(&ZERO.to_le_bytes());
    let first = poseidon.get_last_element_sent_to_ring();
    let _ = poseidon.set_data(&ONE.to_le_bytes());
    let next = poseidon.get_last_element_sent_to_ring();

    let f = first.unwrap();
    let n = next.unwrap();

    assert_ne!(f, n);
    assert_eq!(n, f + 1);
}

#[test]
fn test_build_small_tree_par() {
    let instruction_path = get_instruction_path();

    env_logger::try_init().expect("Invalid logger initialization");

    let dclient = DriverClient::new("0", DriverConfig::driver_client_cfg(CardType::U250));
    let poseidon: PoseidonClient = PoseidonClient::new(Hash::Poseidon, dclient);

    let poseidon = Arc::new(Mutex::new(poseidon));

    let params = PoseidonInitializeParameters {
        tree_height: 4,
        tree_mode: TreeMode::TreeC,
        instruction_path,
    };
    let nof_elements = num_of_elements_in_base_layer(params.tree_height);

    let _ = poseidon.lock().unwrap().initialize(params);

    poseidon.lock().unwrap().loaded_binary_parameters();

    rayon::scope_fifo(|scope| {
        let poseidon_c = poseidon.clone();

        scope.spawn_fifo(move |_| {
            let mut results: Vec<PoseidonResult> = vec![];

            loop {
                thread::sleep(Duration::from_nanos(10));
                let free_poseidon = poseidon_c.lock().unwrap();
                let num_of_pending_results = free_poseidon.get_num_of_pending_results();

                if results.len() >= TREE_HEIGHT_4_NUM_OF_NODES {
                    break;
                }
                let res = free_poseidon.get_raw_results(num_of_pending_results.unwrap());

                let mut result = PoseidonResult::parse_poseidon_hash_results(res.unwrap());
                results.append(&mut result);
            }

            assert_eq!(results.len(), TREE_HEIGHT_4_NUM_OF_NODES);
        });

        scope.spawn_fifo(move |_| {
            let scalar: Vec<u8> = BigUint::from_str_radix(TEST_SCALAR, 10)
                .unwrap()
                .to_bytes_le();

            for _ in 0..nof_elements {
                for _ in 0..11 {
                    thread::sleep(Duration::from_nanos(10));
                    let free_poseidon = poseidon.lock().unwrap();
                    let _ = free_poseidon.set_data(scalar.as_slice());
                    free_poseidon.log_api_values();
                }
            }
            poseidon.lock().unwrap().log_api_values();
        });
    });
}

#[test]
fn test_build_small_tree() {
    let instruction_path = get_instruction_path();

    env_logger::try_init().expect("Invalid logger initialization");

    let dclient = DriverClient::new("0", DriverConfig::driver_client_cfg(CardType::U250));
    let poseidon: PoseidonClient = PoseidonClient::new(Hash::Poseidon, dclient);

    let params = PoseidonInitializeParameters {
        tree_height: 4,
        tree_mode: TreeMode::TreeC,
        instruction_path,
    };

    let nof_elements = num_of_elements_in_base_layer(params.tree_height);

    poseidon.log_api_values();

    let _ = poseidon.initialize(params);

    poseidon.log_api_values();

    poseidon.loaded_binary_parameters();

    let scalar: Vec<u8> = BigUint::from_str_radix(TEST_SCALAR, 10)
        .unwrap()
        .to_bytes_le();

    for _ in 0..nof_elements {
        for _ in 0..11 {
            let _ = poseidon.set_data(scalar.as_slice());
        }
    }

    // shouldnt panic at unwrap
    let result = poseidon
        .result(Some(TREE_HEIGHT_4_NUM_OF_NODES))
        .unwrap()
        .unwrap();

    poseidon.log_api_values();

    assert_eq!(result.len(), TREE_HEIGHT_4_NUM_OF_NODES);

    log::debug!("{:?}", result.len());
    log::debug!("done");
}
