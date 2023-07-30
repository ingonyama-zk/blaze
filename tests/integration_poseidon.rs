use std::{sync::Arc, thread, time::{Duration, Instant}};

use async_std::{channel::{Sender, Receiver, self}, stream::StreamExt};
use dotenv::dotenv;
use ingo_blaze::{
    driver_client::dclient::*,
    ingo_hash::utils::{num_of_elements_in_base_layer, TreeMode},
    ingo_hash::{
        dma_buffer::{Align4K, DmaBuffer},
        poseidon_api::{Hash, PoseidonClient, PoseidonInitializeParameters, PoseidonReadResult},
    },
};
use log::info;
use num::{BigUint, Num};
use log::LevelFilter;
use simple_logging;

const ZERO: u32 = 0;
const ONE: u32 = 1;
const TREE_HEIGHT_4_NUM_OF_NODES: usize = 585;
const TEST_SCALAR: &str =
    "15338226384362629345253584946022322145063321004547266825580649561525819500264";

fn get_instruction_path() -> String {
    dotenv().ok();
    //std::env::var("INSTRUCTION_PATH").expect("INSTRUCTION_PATH must be set.")

    return "/root/poseidon/Poseidon/Releases/U250/V1.00@25.6.2023/python_driver/PoseidonFullWPRC.csv".to_string()
}

fn get_input_outputs(tree_size: u32) -> (usize, usize) {
    let nof_elements = 11 * (u64::pow(8, tree_size));

    let mut nof_results = 0;
    for layer in 0..tree_size + 1 {
        let results_in_layer = u64::pow(8, tree_size - layer);
        nof_results += results_in_layer;
    }

    (
        nof_elements.try_into().unwrap(),
        nof_results.try_into().unwrap(),
    )
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

    let dclient = DriverClient::new("0", DriverConfig::driver_client_u250_cfg());
    let poseidon: Arc<PoseidonClient> = Arc::new(PoseidonClient::new(Hash::Poseidon, dclient));

    let params = PoseidonInitializeParameters {
        tree_height: 3,
        tree_mode: TreeMode::TreeC,
        instruction_path,
    };

    let (nof_elements, results_size) = get_input_outputs(params.tree_height);

    poseidon.log_api();
    
    poseidon.initialize(params);


    poseidon.log_api();

    poseidon.loaded_binary_parameters();

    let scalar: Vec<u8> = BigUint::from_str_radix(TEST_SCALAR, 10)
        .unwrap()
        .to_bytes_le();

    let input_buf_size: usize = (nof_elements as u64 * (scalar.len() * 11) as u64)
        .try_into()
        .unwrap();
    let mut input_buffer = DmaBuffer::new::<Align4K>(input_buf_size);
    let mut output_buffer = DmaBuffer::new::<Align4K>(results_size * 64);

    for _ in 0..(input_buf_size) {
        output_buffer.get_mut().push(0);
    }

    for _ in 0..nof_elements {
        for _ in 0..11 {
            input_buffer.extend_from_slice(scalar.as_slice());
        }
    }

    assert_eq!(input_buffer.get().len(), nof_elements * 11 * 32);
    assert_eq!((input_buffer.get_mut().as_mut_ptr() as u64) % 4096, 0);

    poseidon.set_data(input_buffer.as_slice());

    thread::sleep(Duration::from_millis(10000));
    poseidon.log_api();

    // shouldnt panic at unwrap
    let result = poseidon
        .result(Some(PoseidonReadResult {
            expected_result: TREE_HEIGHT_4_NUM_OF_NODES,
            result_store_buffer: &mut output_buffer,
        }))
        .unwrap()
        .unwrap();

    poseidon.log_api();

    assert_eq!(result.len(), results_size);

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

        for _ in 0..(results_size * 64) {
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

            poseidon_c.log_api();
            log::debug!("done writing");
        });

        scope.spawn_fifo(move |_s| {
            assert_eq!((output_buffer.get_mut().as_mut_ptr() as u64) % 4096, 0);

            let result = poseidon
                .result(Some(PoseidonReadResult {
                    expected_result: output_buffer.len() / 64,
                    result_store_buffer: &mut output_buffer,
                }))
                .unwrap()
                .unwrap();

            poseidon.log_api();

            assert_eq!(result.len(), output_buffer.len() / 64);

            log::debug!("result {:?}", result.len());
            log::debug!("done");
        });
    });
}

#[test]
fn test_build_tree_size_10() {
    let instruction_path = get_instruction_path();

    let dclient = DriverClient::new("0", DriverConfig::driver_client_u250_cfg());
    let poseidon: Arc<PoseidonClient> = Arc::new(PoseidonClient::new(Hash::Poseidon, dclient));

    let params = PoseidonInitializeParameters {
        tree_height: 10,
        tree_mode: TreeMode::TreeC,
        instruction_path,
    };

    let (nof_elements, results_size) = get_input_outputs(params.tree_height);

    poseidon.initialize(params);
    poseidon.loaded_binary_parameters();

    let scalar: Vec<u8> = BigUint::from_str_radix(TEST_SCALAR, 10)
        .unwrap()
        .to_bytes_le();

    let mut input_buffer = DmaBuffer::new::<Align4K>(nof_elements * 11);
    let mut output_buffer = DmaBuffer::new::<Align4K>(results_size  * 64);

    for _ in 0..(results_size  * 64) {
        output_buffer.get_mut().push(0);
    }

    for _ in 0..nof_elements {
        for _ in 0..11 {
            input_buffer.extend_from_slice(scalar.as_slice());
        }
    }

    assert_eq!(input_buffer.get().len(), nof_elements * 11 * scalar.len());
    assert_eq!((input_buffer.get_mut().as_mut_ptr() as u64) % 4096, 0);

    assert_eq!(output_buffer.get().len(), results_size  * 64);
    assert_eq!((output_buffer.get_mut().as_mut_ptr() as u64) % 4096, 0);

    poseidon.set_data(input_buffer.as_slice());

    thread::sleep(Duration::from_millis(10000));
    poseidon.log_api();

    // shouldnt panic at unwrap
    let result = poseidon
        .result(Some(PoseidonReadResult {
            expected_result: results_size  * 64,
            result_store_buffer: &mut output_buffer,
        }))
        .unwrap()
        .unwrap();

    poseidon.log_api();

    assert_eq!(result.len(), results_size);

    log::debug!("result {:?}", result.len());
    log::debug!("done");
}

#[test]
fn test_build_tree_1gb() {
    use rayon::prelude::*;//numactl –localalloc --physcpubind=0-23 ./myprogram
    // cargo test --package ingo-blaze --test integration_poseidon -- test_build_tree_1gb --exact --nocapture 
    use std::sync::{mpsc, Arc, Mutex};
    use std::time::Duration;

    let instruction_path = get_instruction_path();

    //env_logger::try_init().expect("Invalid logger initialization");
    simple_logging::log_to_file("test.log", LevelFilter::Info);

    let dclient = DriverClient::new("0", DriverConfig::driver_client_u250_cfg());
    let poseidon: Arc<PoseidonClient> = Arc::new(PoseidonClient::new(Hash::Poseidon, dclient));

    let tree_height = 10;
    let (input_size, results_size) = get_input_outputs(tree_height);

    let params = PoseidonInitializeParameters {
        tree_height,
        tree_mode: TreeMode::TreeC,
        instruction_path,
    };

    //let nof_elements = num_of_elements_in_base_layer(params.tree_height);

    poseidon.initialize(params);

    poseidon.loaded_binary_parameters();

    poseidon.log_api();

    // 32 BYTES
    let scalar: Vec<u8> = BigUint::from_str_radix(TEST_SCALAR, 10)
        .unwrap()
        .to_bytes_le();

    const BUFFER_SIZE: usize = 377957122048; // 377GB

    let mut input_buffer = DmaBuffer::new::<Align4K>(BUFFER_SIZE);
    let mut output_buffer = DmaBuffer::new::<Align4K>(results_size * 64);

    //unsafe { output_buffer.get_mut().set_len(results_size * 64) };
    let mut gg = Vec::with_capacity(89 * 599479);

    for _ in 0..(89 * 599479) {
        gg.extend_from_slice(&[0 as u8])
    }

    for _ in 0..(results_size * 64)/(89 * 599479) {
        output_buffer.extend_from_slice(gg.as_slice())
    }

    let mut column = Vec::with_capacity(32 * 11);
    for _ in 0..11 {
        // 32 byte scalars
        column.extend_from_slice(scalar.as_slice());
    }

    assert_eq!(column.len(), 32 * 11);

    for _ in 0..BUFFER_SIZE / (32 * 11) {
        input_buffer.extend_from_slice(column.as_slice());
    }

    let poseidon_c = poseidon.clone();
    let poseidon_temp = poseidon.clone();

    //let (tx, rx) = mpsc::sync_channel(1);

    //let buffer1 = Arc::new(Mutex::new(DmaBuffer::new::<Align4K>(BUFFER_SIZE)));
    //let buffer2 = Arc::new(Mutex::new(DmaBuffer::new::<Align4K>(BUFFER_SIZE)));

    //unsafe { buffer1.lock().unwrap().get_mut().set_len(BUFFER_SIZE) };
    //unsafe { buffer2.lock().unwrap().get_mut().set_len(BUFFER_SIZE) };

    poseidon.log_api();

    rayon::scope_fifo(|s| {
        // print temp
        s.spawn_fifo(move |_| {
            use std::thread;
            use std::time::Duration;

            poseidon_temp.dclient.enable_hbm_temp_monitoring();

            loop {
                thread::sleep(Duration::from_millis(100));
                let (temp_inst, temp_avg, temp_max) =
                    poseidon_temp.dclient.monitor_temperature().unwrap();
                log::info!(
                    "temp_inst: {} temp_avg: {} temp_max: {}",
                    temp_inst,
                    temp_avg,
                    temp_max
                );
                let (max, avg, inst) =
                    poseidon_temp.dclient.monitor_power().unwrap();
                log::info!(
                    "pwr_max: {} pwr_avg: {} pwr_inst: {}",
                    max,
                    avg,
                    inst
                );

                //poseidon_temp.log_api_value(&[
                //    ingo_blaze::ingo_hash::hash_hw_code::INGO_POSEIDON_ADDR::ADDR_HIF2CPU_C_NOF_ELEMENTS_PENDING_ON_DMA_FIFO,
                //    ingo_blaze::ingo_hash::hash_hw_code::INGO_POSEIDON_ADDR::ADDR_HIF2CPU_C_NOF_RESULTS_PENDING_ON_DMA_FIFO,
                //    ingo_blaze::ingo_hash::hash_hw_code::INGO_POSEIDON_ADDR::ADDR_HIF2CPU_C_MAX_RECORDED_PENDING_RESULTS
                //]);
            }
        });

        // First thread: Read data from the 100 MB buffer and send buffer pointers
        //s.spawn_fifo(move |_| {
            //let mut current_buffer = 1;

            //let total_num_of_chunks = input_size * 32 / BUFFER_SIZE;
            //for chunk in 0..total_num_of_chunks {
                //let (buffer_to_send, buffer_to_fill) = match current_buffer {
                //    1 => (Arc::clone(&buffer1), Arc::clone(&buffer2)),
                //    2 => (Arc::clone(&buffer2), Arc::clone(&buffer1)),
                //    _ => unreachable!(),
                //};

                //log::info!("sending chunk {} / {}", chunk, total_num_of_chunks);
                //tx.send(buffer_to_send).unwrap();
                //{
                //    let mut buffer_to_fill = buffer_to_fill.lock().unwrap();
                //    buffer_to_fill
                //        .get_mut()
                //        .copy_from_slice(&input_buffer.get());
                //}
//
                //drop(buffer_to_fill); // Unlock the mutex before sending
//
                //current_buffer = if current_buffer == 1 { 2 } else { 1 };
            //}
            
            //log::info!("dropping tx");
            //drop(tx); // Close the channel to signal the end of incoming data
        //});

        // Second thread: Receive and process the buffer pointers
        s.spawn_fifo(move |_| {
            let start = Instant::now();
            //while let Ok(buffer_ptr) = rx.recv() {
            //    log::info!("got new chucnk");
            //    assert_eq!(
            //        (buffer_ptr.lock().unwrap().get_mut().as_mut_ptr() as u64) % 4096,
            //        0
            //    );
            //    assert_eq!(
            //        (buffer_ptr
            //            .lock()
            //            .unwrap()
            //            .get_mut()
            //            .as_mut_slice()
            //            .as_mut_ptr() as u64)
            //            % 4096,
            //        0
            //    );
//
            //    log::info!("Got some data");
            //    let start_inner = Instant::now();
            //    poseidon_c.set_data(buffer_ptr.lock().unwrap().get_mut().as_mut_slice());
            //    let duration_inner = start_inner.elapsed();
            //    log::info!("Writing to fpga, took: {:?}", duration_inner);
            //}

            log::info!("got new chucnk");
            assert_eq!(
                (input_buffer.get_mut().as_mut_ptr() as u64) % 4096,
                0
            );
            assert_eq!(
                (input_buffer
                    .get_mut()
                    .as_mut_slice()
                    .as_mut_ptr() as u64)
                    % 4096,
                0
            );

            log::info!("Got some data");
            let start_inner = Instant::now();
            poseidon_c.dclient.reset_sensor_data();
            poseidon_c.set_data(input_buffer.get_mut().as_mut_slice());
            let duration_inner = start_inner.elapsed();
            log::info!("Writing to fpga, took: {:?}", duration_inner);

            let duration = start.elapsed();
            log::info!("Completed writing all data to FPGA. Compute time is: {:?}", duration);
        });

        s.spawn_fifo(move |_s| {
            assert_eq!((output_buffer.get_mut().as_mut_ptr() as u64) % 4096, 0);
            
            println!("Waiting for results");

            let start = Instant::now();
            let result = poseidon
                .result(Some(PoseidonReadResult {
                    expected_result: output_buffer.len() / 64,
                    result_store_buffer: &mut output_buffer,
                }))
                .unwrap()
                .unwrap();

            assert_eq!(result.len(), output_buffer.len() / 64);
            let duration = start.elapsed();

            poseidon.log_api();
            println!("Computed a result of length {:?}, took: {:?}", result.len(), duration);
        });
    });
}

/*
#[test]
fn test_build_tree_multi_buffering() {
    use std::sync::{Mutex, mpsc};

    simple_logging::log_to_file("test.log", LevelFilter::Info);
    
    let dclient = DriverClient::new("0", DriverConfig::driver_client_u250_cfg());
    let poseidon: Arc<PoseidonClient> = Arc::new(PoseidonClient::new(Hash::Poseidon, dclient));

    let tree_height = 10;
    let (input_size, results_size) = get_input_outputs(tree_height);

    let params = PoseidonInitializeParameters {
        tree_height,
        tree_mode: TreeMode::TreeC,
        instruction_path: get_instruction_path(),
    };

    //let nof_elements = num_of_elements_in_base_layer(params.tree_height);

    poseidon.initialize(params);

    poseidon.loaded_binary_parameters();

    poseidon.log_api();


    // The offset will keep track of where we are in the input_buffer
    let mut offset = 0;
    const NUM_BUFFERS: usize = 3;
    const CHUNK_SIZE: usize = 256_000_000;
    const INPUT_BUFFER_SIZE: usize = 377957122048; // 377GB

    let mut input_buffer = DmaBuffer::new::<Align4K>(INPUT_BUFFER_SIZE);
    // 32 BYTES
    let scalar: Vec<u8> = BigUint::from_str_radix(TEST_SCALAR, 10)
        .unwrap()
        .to_bytes_le();

    log::info!("begin: allocating test data");
    let mut column = Vec::with_capacity(32 * 11);
    for _ in 0..11 {
        // 32 byte scalars
        column.extend_from_slice(scalar.as_slice());
    }

    assert_eq!(column.len(), 32 * 11);

    for _ in 0..INPUT_BUFFER_SIZE / (32 * 11) {
        input_buffer.extend_from_slice(column.as_slice());
    }
    log::info!("end: allocating test data");

    let buffers: Vec<_> = (0..NUM_BUFFERS).map(|_| Arc::new(Mutex::new(DmaBuffer::new::<Align4K>(CHUNK_SIZE)))).collect();

    for ele in &buffers {
        ele.lock().ok().unwrap().set_len(CHUNK_SIZE);
    }

    use async_std::task;
    use std::sync::Arc;
    use std::collections::VecDeque;


    struct BufferPool {
        pool: VecDeque<DmaBuffer>,
    }

    impl BufferPool {
        fn new(size: usize, buffer_size: usize) -> Self {
            let pool: VecDeque<DmaBuffer> = (0..size).map(|_| DmaBuffer::new::<Align4K>(buffer_size)).collect();

            pool.iter_mut().for_each(|buffer| {
                buffer.set_len(buffer_size)
            });

            Self { pool }
        }

        fn get(&mut self) -> Option<DmaBuffer> {
            self.pool.pop_front()
        }

        fn put(&mut self, buffer: DmaBuffer) {
            self.pool.push_back(buffer);
        }
    }

    async fn producer_task(input_buffer: &[u8], sender: Sender<Vec<u8>>, buffer_pool: Arc<async_std::sync::Mutex<BufferPool>>) {
        let num_chunks = input_buffer.len() / CHUNK_SIZE;

        for i in 0..num_chunks {
            let start = i * CHUNK_SIZE;
            let end = start + CHUNK_SIZE;

            let mut buffer_pool = buffer_pool.lock().await;
            let mut chunk = buffer_pool.get().unwrap();
            chunk.copy_from_slice(&input_buffer[start..end]);
            sender.send(chunk).await;

        }
    }

    async fn consumer_task(poseidon: Arc<PoseidonClient>, receiver: Receiver<Vec<u8>>, buffer_pool: Arc<async_std::sync::Mutex<BufferPool>>) {
        while let Some(chunk) = &mut receiver.next().await {
            // Simulate sending chunk to FPGA.
            // Replace this with your actual code for sending the data to the FPGA.
            println!("Sending chunk of size {} to FPGA.", chunk.len());

            let mut buffer_pool = buffer_pool.lock().await;
            poseidon.set_data(chunk);
        } 
    }

    task::block_on(async {
        let input_buffer = vec![0u8; 377_957_122_048];  // Simulate a 377GB input buffer.
        let buffer_pool = Arc::new(async_std::sync::Mutex::new(BufferPool::new(10, CHUNK_SIZE)));

        let (sender, receiver) = channel::bounded(10);  // Create a channel for communication between tasks.

        let producer = task::spawn(producer_task(&input_buffer, sender, buffer_pool.clone()));
        let consumer = task::spawn(consumer_task(poseidon, receiver, buffer_pool));

        // Wait for both tasks to finish.
        producer.await;
        consumer.await;
    });
}
 */