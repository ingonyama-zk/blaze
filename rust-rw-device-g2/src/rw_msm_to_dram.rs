use std::{fs::OpenOptions, os::unix::prelude::{OpenOptionsExt, FileExt}, time::Duration};
use std::time::{Instant};

use num_bigint::BigUint;

const CHUNK_SIZE: usize = 1024;
const BYTE_SIZE_POINT_COORD: usize = 32; // changed 
const BYTE_SIZE_SCALAR: usize = 32;
const INGO_MSM_CTRL_BASEADDR: u64 = 0x0010_0000;
const DMA_SCALARS_BASEADDR: u64 = 0x0000_0010_0000_0000;
const DMA_POINTS_BASEADDR: u64 = 0x0000_0011_0000_0000;
const DFX_DECOUPLER_BASEADDR: u64 = 0x0005_0000; 
const DMA: &str = "/dev/xdma1_h2c_0";
const AXI: &str = "/dev/xdma1_user";

fn div_up(a: usize, b: usize) -> usize {
    (a + (b - 1))/b
}

/// Reset to the device
///
pub fn init(){
    println!("Open Device Channels...");
    let axi = open_axi_channel();
    println!("Reset Device...");
    set_dfx_decoupling(&axi, true);
    set_dfx_decoupling(&axi, false);
}

/// Returns the label of the current task
///
pub fn get_msm_label() -> u8{
    println!("Open Device Channels...");
    let axi = open_axi_channel();
    let task_label = get_ingo_msm_task_label(&axi)[0];
    println!("Task label: {}", task_label);
    return task_label; 
}

pub fn msm_calc_biguint(points: &Vec<BigUint>, scalars: &Vec<BigUint>, size: usize) -> ([Vec<u8>; 6],Duration,u8) {
    println!("Format Inputs...");
    let points_bytes = get_formatted_unified_points_from_biguint(points);  
    let scalars_bytes = get_formatted_unified_scalars_from_biguint(scalars);
    let (result_vector, duration, result_label) = msm_core(&points_bytes, &scalars_bytes, size);
    
    return (
        [result_vector[0..32].to_vec()
        ,result_vector[32..64].to_vec(),
        result_vector[64..96].to_vec(),
        result_vector[96..128].to_vec()
        ,result_vector[128..160].to_vec(),
        result_vector[160..192].to_vec()],
        duration,result_label)
}

pub fn msm_calc(points: &[u8], scalars: &[u8], size: usize) -> (Vec<Vec<u8>>,Duration,u8){ //TODO: 
    let (result_vector, duration, result_label) = msm_core(&points, &scalars, size);
    
    return (
        vec![result_vector[0..32].to_vec() //TODO: can be rearranged here to support Fq2::from_random_bytes and avoid concat()
        ,result_vector[32..64].to_vec(),
        result_vector[64..96].to_vec(),
        result_vector[96..128].to_vec()
        ,result_vector[128..160].to_vec(),
        result_vector[160..192].to_vec()],
        duration,result_label)
}

/// Returns MSM result of elements in bls12_377 in projective form
///
/// # Arguments
///
/// * `points` - &Vec<u8> of BigUint of size at most 48 bytes describing points coordinates. For each point, the y coordinate should come first, followed by the x coordinate.
/// * `scalars` - &Vec<u8> of BigUint of size at most 32 bytes (the length of this vector should be twice as the length of the points vector).
/// 
/// # Output
/// Returns a tuple of 3 elements with types ([Vec<u8>; 3],Duration, u8):
/// * An array of 3 Vec<u8> (48 bytes each), representing the result in projective coordinates.
/// * Duration of the computation. 
/// * The label of the result that was read. 
pub fn msm_core(points_bytes: &[u8], scalars_bytes: &[u8], size: usize) -> (Vec<u8>, Duration,u8) {
    let nof_elements: usize = size;
    let chunks: usize = div_up(nof_elements,CHUNK_SIZE);
    println!("Open Device Channels...");
    let axi = open_axi_channel();
    let h2c = open_dma_channel();
    println!("Setting DMA Source...");
    set_ingo_msm_coeffs_source(&axi,0);
    set_ingo_msm_bases_source(&axi,0);
    println!("Setting NOF Elements  = {}...", nof_elements);
    set_ingo_msm_nof_elements(&axi, nof_elements);
    println!("Pushing Task Signal...");
    set_ingo_msm_push_task(&axi);
    println!("Task label: {}", get_ingo_msm_task_label(&axi)[0]);
    println!("Writing Task...");
    let start = Instant::now();
    write_msm_to_fifo(points_bytes, scalars_bytes, h2c,chunks);
    println!("Waiting for result...");
    wait_for_valid_result(&axi);
    let duration = start.elapsed();
    println!("Received result...");
    println!("Time elapsed is: {:?} for size: {}", duration, nof_elements);
    
    let result = read_result(&axi);
    let z_chunk_re = &result[0..32];
    let y_chunk_re = &result[32..64];
    let x_chunk_re = &result[64..96];
    println!("Pop Re...");
    set_ingo_msm_pop_task(&axi);

    let result = read_result(&axi);
    let z_chunk_im = &result[0..32];
    let y_chunk_im = &result[32..64];
    let x_chunk_im = &result[64..96];
    println!("Pop Im...");
    set_ingo_msm_pop_task(&axi);

    let result_label = get_ingo_msm_result_label(&axi)[0]; 
    println!("Result label: {}", result_label);

    println!("X Re bytes {:02X?}", x_chunk_re);
    println!("X Im bytes {:02X?}", x_chunk_im);
    println!("Y Re bytes {:02X?}", y_chunk_re);
    println!("Y Im bytes {:02X?}", y_chunk_im);
    println!("Z Re bytes {:02X?}", z_chunk_re);
    println!("Z Im bytes {:02X?}", z_chunk_im);

    let mut result_vector = Vec::new();
    result_vector.extend(z_chunk_im.to_vec());
    result_vector.extend(y_chunk_im.to_vec());
    result_vector.extend(x_chunk_im.to_vec());
    result_vector.extend(z_chunk_re.to_vec());
    result_vector.extend(y_chunk_re.to_vec());
    result_vector.extend(x_chunk_re.to_vec());

    (result_vector,
    duration, result_label)
}


fn read_result(axi: &std::fs::File) -> Vec<u8> {
    let mut result: Vec<u8> = Vec::new();
    for i in 0..24{
        let mut read_chunk = [1,1,1,1];
        axi.read_exact_at(&mut read_chunk, INGO_MSM_CTRL_BASEADDR + 0x38 + i*4).expect("Faild to read from axi");
        result.extend(read_chunk);
    }
    result
}

fn wait_for_valid_result(axi: &std::fs::File) {
    let mut result_valid = [0,0,0,0];
    axi.read_exact_at(&mut result_valid, INGO_MSM_CTRL_BASEADDR + 0x30).expect("Faild to read from axi");
    while result_valid == [0,0,0,0]{
        axi.read_exact_at(&mut result_valid, INGO_MSM_CTRL_BASEADDR + 0x30).expect("Faild to read from axi");
    }
}

fn get_ingo_msm_result_label(axi: &std::fs::File) -> [u8;4]{
    let mut result_label = [0,0,0,0];
    axi.read_exact_at(&mut result_label, INGO_MSM_CTRL_BASEADDR + 0x34).expect("Faild to read from axi");
    return result_label
}

fn get_ingo_msm_task_label(axi: &std::fs::File) -> [u8;4]{
    let mut task_label = [0,0,0,0];
    axi.read_exact_at(&mut task_label, INGO_MSM_CTRL_BASEADDR + 0xc).expect("Faild to read from axi");
    return task_label
}

fn write_msm_to_fifo(points_bytes: &[u8], scalars_bytes: &[u8], h2c: std::fs::File, chunks: usize) {
    let payload_size_scalars: usize = CHUNK_SIZE * 32;
    let payload_size_points: usize = CHUNK_SIZE * 32 * 4;
    for i in 0..chunks{
        let p_chunk: &[u8];
        let s_chunk: &[u8];
        if i != chunks - 1{
            p_chunk = &points_bytes[i*payload_size_points.. (i+1)*payload_size_points];
            s_chunk = &scalars_bytes[i*payload_size_scalars ..(i+1)*payload_size_scalars];
        }
        else{
            p_chunk = &points_bytes[i*payload_size_points..];
            s_chunk = &scalars_bytes[i*payload_size_scalars ..];
        }
        // println!("p_chunk {:02X?}", p_chunk);
        // println!("s_chunk {:02X?}", s_chunk);
        h2c.write_all_at(p_chunk, DMA_POINTS_BASEADDR).expect("Faild to write to dma");
        h2c.write_all_at(s_chunk, DMA_SCALARS_BASEADDR).expect("Faild to write to dma");
    }
}

fn write_scalar_only_to_fifo(scalars_bytes: Vec<u8>, h2c: std::fs::File, chunks: usize) {
    let payload_size_scalars: usize = CHUNK_SIZE * 32;
    for i in 0..chunks{
        let s_chunk: &[u8];
        if i != chunks - 1{
            s_chunk = &scalars_bytes[i*payload_size_scalars ..(i+1)*payload_size_scalars];
        }
        else{
            s_chunk = &scalars_bytes[i*payload_size_scalars ..];
        }
        h2c.write_all_at(s_chunk, DMA_SCALARS_BASEADDR).expect("Faild to write to dma");
    }
}


fn set_ingo_msm_pop_task(axi: &std::fs::File) {
    axi.write_all_at(&[1,0,0,0], INGO_MSM_CTRL_BASEADDR + 0xc8).expect("Faild to write to axi");
}

fn set_ingo_msm_nof_elements(axi: &std::fs::File, nof_elements: usize) {
    let bytes_array = u32::try_from(nof_elements).unwrap().to_le_bytes();
    axi.write_all_at(&bytes_array, INGO_MSM_CTRL_BASEADDR + 0x28).expect("Faild to write to axi");
}

fn set_ingo_msm_hbm_points_address(axi: &std::fs::File, hbm_address: u64) {
    let bytes_array = u32::try_from(hbm_address).unwrap().to_le_bytes();
    axi.write_all_at(&bytes_array, INGO_MSM_CTRL_BASEADDR + 0x14).expect("Faild to write to axi");
}

fn set_ingo_msm_push_task(axi: &std::fs::File) {
    axi.write_all_at(&[1,0,0,0], INGO_MSM_CTRL_BASEADDR + 0x2c).expect("Faild to write to axi");
}

fn set_ingo_msm_bases_source(axi: &std::fs::File, signal: u8) {
    axi.write_all_at(&[signal,0,0,0], INGO_MSM_CTRL_BASEADDR + 0x18).expect("Faild to write to axi");
}

fn set_ingo_msm_coeffs_source(axi: &std::fs::File, signal: u8) {
    axi.write_all_at(&[signal,0,0,0], INGO_MSM_CTRL_BASEADDR + 0x24).expect("Faild to write to axi");
}

fn get_formatted_unified_scalars_from_biguint(scalars: &Vec<BigUint>) -> Vec<u8> {
    let mut scalars_bytes: Vec<u8> = Vec::new();
    for i in 0..scalars.len(){
        let mut bytes_array = scalars[i].to_bytes_le();
        bytes_array.extend(std::iter::repeat(0).take(BYTE_SIZE_SCALAR-bytes_array.len()));
        scalars_bytes.extend(bytes_array);
    }
    scalars_bytes
}

fn get_formatted_unified_points_from_biguint(points: &Vec<BigUint>) -> Vec<u8> {
    let mut points_bytes: Vec<u8> = Vec::new();
    for i in 0..points.len(){
        let mut bytes_array = points[i].to_bytes_le();
        bytes_array.extend(std::iter::repeat(0).take(BYTE_SIZE_POINT_COORD-bytes_array.len()));
        points_bytes.extend(bytes_array);
    }
    points_bytes
}

fn open_dma_channel() -> std::fs::File {
    let mut options = OpenOptions::new();
    options.write(true);
    if cfg!(unix) {
        options.custom_flags(libc::O_SYNC);
        options.custom_flags(libc::O_WRONLY);
    }
    let h2c = options.open(DMA).unwrap();
    h2c
}

fn open_axi_channel() -> std::fs::File {
    let mut options = OpenOptions::new();
    options.write(true);
    options.read(true);
    if cfg!(unix) {
        options.custom_flags(libc::O_SYNC);
        options.custom_flags(libc::O_RDWR);
    }
    let axi = options.open(AXI).unwrap();
    axi
}

fn set_dfx_decoupling(axi: &std::fs::File, decouple: bool){
    if decouple == true{
        axi.write_all_at(&[1,0,0,0], DFX_DECOUPLER_BASEADDR + 0x0).expect("Faild to write to axi");
    }
    else{
        axi.write_all_at(&[0,0,0,0], DFX_DECOUPLER_BASEADDR + 0x0).expect("Faild to write to axi");
    }
}