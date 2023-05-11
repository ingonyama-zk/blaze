use std::{
    sync::{Arc},
    thread,
    time::Duration,
};

use dotenv::dotenv;
use ingo_blaze::{
    driver_client::dclient::*,
    ingo_hash::{poseidon_api::{Hash, PoseidonClient, PoseidonInitializeParameters, PoseidonReadResult}, dma_buffer::{DmaBuffer, Align4K}},
    ingo_hash::utils::{num_of_elements_in_base_layer, TreeMode},
};
use log::info;
use num::{BigUint, Num};

const ZERO: u32 = 0;
const ONE: u32 = 1;
const TREE_HEIGHT_4_NUM_OF_NODES: usize = 585;
const TEST_SCALAR: &str = "15338226384362629345253584946022322145063321004547266825580649561525819500264";

fn get_instruction_path() -> String {
    dotenv().ok();
    std::env::var("INSTRUCTION_PATH").expect("INSTRUCTION_PATH must be set.")
}

fn get_input_outputs(tree_size: u32) -> (usize, usize) {
    let nof_elements = 11 * (u64::pow(8,tree_size));

    let mut nof_results = 0;
    for layer in 0..tree_size + 1 {
        let results_in_layer = u64::pow(8,tree_size - layer);
        nof_results += results_in_layer;
    }

    (nof_elements.try_into().unwrap(), nof_results.try_into().unwrap())
}


#[test]
fn test_sanity_check() {
    let instruction_path = get_instruction_path();

    let dclient = DriverClient::new("0", DriverConfig::driver_client_c1100_cfg());
    let poseidon: PoseidonClient = PoseidonClient::new(Hash::Poseidon, dclient);

    poseidon.reset();
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
fn test_build_small_tree() {
    let instruction_path = get_instruction_path();

    env_logger::try_init().expect("Invalid logger initialization");

    let dclient = DriverClient::new("0", DriverConfig::driver_client_c1100_cfg());
    let poseidon: Arc<PoseidonClient> = Arc::new(PoseidonClient::new(Hash::Poseidon, dclient));

    let params = PoseidonInitializeParameters {
        tree_height: 3,
        tree_mode: TreeMode::TreeC,
        instruction_path,
    };

    let nof_elements = num_of_elements_in_base_layer(params.tree_height);

    poseidon.log_api_values();

    poseidon.initialize(params);
    poseidon.dclient.initialize_cms();

    poseidon.log_api_values();

    poseidon.loaded_binary_parameters();

    let scalar: Vec<u8> = BigUint::from_str_radix(TEST_SCALAR, 10)
        .unwrap()
        .to_bytes_le();

    let input_buf_size: usize = (nof_elements * (scalar.len() * 11) as u32).try_into().unwrap();
    let mut input_buffer = DmaBuffer::new::<Align4K>(input_buf_size);
    let mut output_buffer = DmaBuffer::new::<Align4K>(585 * 64);

    for _ in 0.. (585 * 64) {
        output_buffer.get_mut().push(0);
    }

    for _ in 0..nof_elements {
        for _ in 0..11 {
            input_buffer.extend_from_slice(scalar.as_slice());
        }
    }

    assert_eq!(input_buffer.get().len(), 180224);
    assert_eq!((input_buffer.get_mut().as_mut_ptr() as u64) % 4096, 0);

    poseidon.set_data(input_buffer.as_slice());

    thread::sleep(Duration::from_millis(10000));
    poseidon.log_api_values();

    // shouldnt panic at unwrap
    let result = poseidon
        .result(Some(PoseidonReadResult { expected_result: TREE_HEIGHT_4_NUM_OF_NODES, result_store_buffer: &mut output_buffer }))
        .unwrap().unwrap();

    poseidon.log_api_values();

    assert_eq!(result.len(), TREE_HEIGHT_4_NUM_OF_NODES);

    log::debug!("result {:?}", result.len());
    log::debug!("done");
}

#[test]
fn test_build_small_tree_parllel() {
    rayon::scope_fifo(|scope| {
        let instruction_path = get_instruction_path();

        env_logger::try_init().expect("Invalid logger initialization");
        
        let dclient = DriverClient::new("0", DriverConfig::driver_client_c1100_cfg());
        let poseidon: Arc<PoseidonClient> = Arc::new(PoseidonClient::new(Hash::Poseidon, dclient));

        let tree_height = 7;
        let (input_size, results_size) = get_input_outputs(tree_height);

        let params = PoseidonInitializeParameters {
            tree_height,
            tree_mode: TreeMode::TreeC,
            instruction_path,
        };

        let nof_elements = num_of_elements_in_base_layer(params.tree_height);


        poseidon.initialize(params);


        poseidon.loaded_binary_parameters();

        let scalar: Vec<u8> = BigUint::from_str_radix(TEST_SCALAR, 10)
            .unwrap()
            .to_bytes_le();

        let mut input_buffer = DmaBuffer::new::<Align4K>(input_size * 32);
        let mut output_buffer = DmaBuffer::new::<Align4K>(results_size * 64);

        for _ in 0.. (results_size * 64) {
            output_buffer.get_mut().push(0);
        }

        for _ in 0..nof_elements {
            for _ in 0..11 {
                input_buffer.extend_from_slice(scalar.as_slice());
            }
        }

        let poseidon_c = poseidon.clone();

        scope.spawn_fifo(move |_s| {
            assert_eq!((input_buffer.get_mut().as_mut_ptr() as u64) % 4096, 0);

            poseidon_c.set_data(input_buffer.as_slice());

            poseidon_c.log_api_values();
            log::debug!("done writing");
        });

        scope.spawn_fifo(move |_s| {
            assert_eq!((output_buffer.get_mut().as_mut_ptr() as u64) % 4096, 0);

            let result = poseidon
                .result(Some(PoseidonReadResult {
                    expected_result: output_buffer.len() / 64,
                    result_store_buffer: &mut output_buffer,
                }))
                .unwrap().unwrap();


            poseidon.log_api_values();

            assert_eq!(result.len(), output_buffer.len() / 64);

            log::debug!("result {:?}", result.len());
            log::debug!("done");
        });
    });
}

#[test]
fn test_build_tree_1gb() {
    use rayon::prelude::*;
    use std::sync::{mpsc, Arc, Mutex};
    use std::time::Duration;

    let instruction_path = get_instruction_path();

    env_logger::try_init().expect("Invalid logger initialization");

    let dclient = DriverClient::new("0", DriverConfig::driver_client_c1100_cfg());
    let poseidon: Arc<PoseidonClient> = Arc::new(PoseidonClient::new(Hash::Poseidon, dclient));

    let tree_height = 10;
    let (input_size, results_size) = get_input_outputs(tree_height);

    let params = PoseidonInitializeParameters {
        tree_height,
        tree_mode: TreeMode::TreeC,
        instruction_path,
    };

    let nof_elements = num_of_elements_in_base_layer(params.tree_height);

    poseidon.initialize(params);


    poseidon.loaded_binary_parameters();

    // 32 BYTES
    let scalar: Vec<u8> = BigUint::from_str_radix(TEST_SCALAR, 10)
        .unwrap()
        .to_bytes_le();

    const BUFFER_SIZE: usize = 999999968; // 1GB

    let mut input_buffer = DmaBuffer::new::<Align4K>(BUFFER_SIZE); // 1 gb
    let mut output_buffer = DmaBuffer::new::<Align4K>(results_size * 64); // 78 gb

    unsafe { output_buffer.get_mut().set_len(results_size * 64) };

    let mut column = Vec::with_capacity(32 * 11);
    for _ in 0..11 {
        // 32 byte scalars
        column.extend_from_slice(scalar.as_slice());
    }

    assert_eq!(column.len(), 32 * 11);

    for _ in 0.. BUFFER_SIZE / (32 * 11) {
        input_buffer.extend_from_slice(column.as_slice());
    }

    let poseidon_c = poseidon.clone();
    let poseidon_temp = poseidon.clone();

    let (tx, rx) = mpsc::sync_channel(1);

    let buffer1 = Arc::new(Mutex::new(DmaBuffer::new::<Align4K>(BUFFER_SIZE)));
    let buffer2 = Arc::new(Mutex::new(DmaBuffer::new::<Align4K>(BUFFER_SIZE)));

    unsafe { buffer1.lock().unwrap().get_mut().set_len(BUFFER_SIZE) };
    unsafe { buffer2.lock().unwrap().get_mut().set_len(BUFFER_SIZE) };

    rayon::scope_fifo(|s| {
        s.spawn_fifo(move |_| {
            use std::thread;
            use std::time::Duration;

            poseidon_temp.dclient.enable_hbm_temp_monitoring();
            loop {
                thread::sleep(Duration::from_millis(250));
                let (temp_inst, temp_avg, temp_max) = poseidon_temp.dclient.monitor_temperature().unwrap();
                log::debug!("temp_inst: {} temp_avg: {} temp_max: {}", temp_inst, temp_avg, temp_max);
            }
        });
        
        // First thread: Read data from the 100 MB buffer and send buffer pointers
        s.spawn_fifo(move |_| {
            let mut current_buffer = 1;

            let total_num_of_chunks = input_size*32 / BUFFER_SIZE;
            for chunk in 0..total_num_of_chunks  {
                let (buffer_to_send, buffer_to_fill) = match current_buffer {
                    1 => (Arc::clone(&buffer1), Arc::clone(&buffer2)),
                    2 => (Arc::clone(&buffer2), Arc::clone(&buffer1)),
                    _ => unreachable!(),
                };

                log::debug!("sending chunk {} / {}", chunk, total_num_of_chunks);
                tx.send(buffer_to_send).unwrap();
                {
                    let mut buffer_to_fill = buffer_to_fill.lock().unwrap();
                    buffer_to_fill.get_mut().copy_from_slice(&input_buffer.get());
                }

                drop(buffer_to_fill); // Unlock the mutex before sending
                
                current_buffer = if current_buffer == 1 { 2 } else { 1 };
            }
            drop(tx); // Close the channel to signal the end of incoming data
        });

        // Second thread: Receive and process the buffer pointers
        s.spawn_fifo(move |_| {
            while let Ok(buffer_ptr) = rx.recv() {
                assert_eq!((buffer_ptr.lock().unwrap().get_mut().as_mut_ptr() as u64) % 4096, 0);
                assert_eq!((buffer_ptr.lock().unwrap().get_mut().as_mut_slice().as_mut_ptr() as u64) % 4096, 0);
                

                poseidon_c.set_data(buffer_ptr.lock().unwrap().get_mut().as_mut_slice());
    
                poseidon_c.log_api_values();
                log::debug!("Completed writing all data to FPGA");
            }
        });

        s.spawn_fifo(move |_s| {
            assert_eq!((output_buffer.get_mut().as_mut_ptr() as u64) % 4096, 0);

            let result = poseidon
                .result(Some(PoseidonReadResult {
                    expected_result: output_buffer.len() / 64,
                    result_store_buffer: &mut output_buffer,
                }))
                .unwrap().unwrap();


            poseidon.log_api_values();

            assert_eq!(result.len(), output_buffer.len() / 64);

            log::debug!("Computed a result of length {:?}", result.len());
        });
    });
}
