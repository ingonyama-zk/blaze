use std::{fs::OpenOptions, os::unix::prelude::{OpenOptionsExt, FileExt}, time::Duration, ops::Mul};
use std::time::{Instant};

use crate::curve::{G1Affine as GAffine, Fq, Fr};
use ark_ec::AffineCurve;
use ark_ff::{Field, BigInteger256, Fp256, Zero};
use num_bigint::BigUint;

const CHUNK_SIZE: usize = 1024;
#[cfg(feature = "bls12-377")]
const BYTE_SIZE_POINT_COORD: usize = 48;
#[cfg(feature = "bn254")]
const BYTE_SIZE_POINT_COORD: usize = 32;

const BYTE_SIZE_SCALAR: usize = 32;
const INGO_MSM_CTRL_BASEADDR: u64 = 0x0010_0000;
const DMA_SCALARS_BASEADDR: u64 = 0x0000_0010_0000_0000;
const DMA_POINTS_BASEADDR: u64 = 0x0000_0011_0000_0000;
const DFX_DECOUPLER_BASEADDR: u64 = 0x0005_0000; 
const DMA: &str = "/dev/xdma0_h2c_0"; 
const AXI: &str = "/dev/xdma0_user"; 

fn div_up(a: usize, b: usize) -> usize {
    (a + (b - 1))/b
}

pub fn as_u32_le(arr: Vec<u8>) -> Vec<u32> {
    let mut extended: Vec<u8> = Vec::new();  
    let arr_len = arr.len();
    extended.extend(arr);
    extended.extend(vec![0;arr_len%4]);
    let mut result: Vec<u32> = Vec::new();  
    for i in 0..arr_len/4{
        let four_bytes_to_one_u32 = ((extended[4*i + 0] as u32) <<  0) +
        ((extended[4*i + 1] as u32) <<  8) +
        ((extended[4*i + 2] as u32) << 16) +
        ((extended[4*i + 3] as u32) << 24); 
        result.push(four_bytes_to_one_u32);
    }
    return result; 
}

pub fn u32_vec_to_u8_vec(u32_vec: &Vec<u32>) -> Vec<u8> {
    let mut u8_vec: Vec<u8> = Vec::new();
    for i in 0..u32_vec.len(){
        let bytes_array = u32_vec[i].to_le_bytes().to_vec();
        u8_vec.extend(bytes_array);
    }
    u8_vec
}

/// Reset to the device
///
pub fn init(){
    // println!("Open Device Channels...");
    let axi = open_axi_channel();
    // println!("Reset Device...");
    set_dfx_decoupling(&axi, true);
    set_dfx_decoupling(&axi, false);
}

/// Returns the label of the current task
///
pub fn get_msm_label() -> u8{
    // println!("Open Device Channels...");
    let axi = open_axi_channel();
    let task_label = get_ingo_msm_task_label(&axi)[0];
    // println!("Task label: {}", task_label);
    return task_label; 
}

pub fn msm_calc(point_bytes: &[u8], scalar_bytes: &[u8], size: usize) -> (Vec<Vec<u8>>,Duration, u8) {
    let chunks: usize = div_up(size,CHUNK_SIZE);
    println!("Open Device Channels...");
    let axi = open_axi_channel();
    let h2c = open_dma_channel();
    println!("Task label: {}", get_ingo_msm_task_label(&axi)[0]);
    println!("Setting DMA Source...");
    set_ingo_msm_coeffs_source(&axi,0);
    set_ingo_msm_bases_source(&axi,0);
    println!("Setting NOF Elements  = {}...", size);
    set_ingo_msm_nof_elements(&axi, size);
    println!("Pushing Task Signal...");
    set_ingo_msm_push_task(&axi);
    println!("Writing Task...");
    let start = Instant::now();
    write_msm_to_fifo(&point_bytes, &scalar_bytes, h2c,chunks);
    println!("Waiting for result...");
    wait_for_valid_result(&axi);
    let duration = start.elapsed();
    println!("Result label: {}", get_ingo_msm_result_label(&axi)[0]);
    let result_label = get_ingo_msm_result_label(&axi)[0]; 
    println!("Received result...");
    println!("Time elapsed is: {:?} for size: {}", duration, size);
    let result = read_result(&axi);
    let z_chunk = &result[0..BYTE_SIZE_POINT_COORD];
    let y_chunk = &result[BYTE_SIZE_POINT_COORD..BYTE_SIZE_POINT_COORD*2];
    let x_chunk = &result[BYTE_SIZE_POINT_COORD*2..];
    println!("X bytes {:02X?}", x_chunk);
    println!("Y bytes {:02X?}", y_chunk);
    println!("Z bytes {:02X?}", z_chunk);
    println!("Pop result...");
    set_ingo_msm_pop_task(axi);
    (vec![x_chunk.to_vec(),y_chunk.to_vec(),z_chunk.to_vec()],duration,result_label)
}

/// Returns MSM result of elements in bls12_377 in projective form
///
/// # Arguments
///
/// * `points` - &Vec<BigUint> of BigUint of size at most 48 bytes describing points coordinates. For each point, the y coordinate should come first, followed by the x coordinate.
/// * `scalars` - &Vec<BigUint> of BigUint of size at most 32 bytes (the length of this vector should be twice as the length of the points vector).
/// 
/// # Output
/// Returns a tuple of 3 elements with types ([BigUint; 3],Duration, u8):
/// * An array of 3 BigUint (48 bytes each), representing the result in projective coordinates.
/// * Duration of the computation. 
/// * The label of the result that was read. 
pub fn msm_calc_biguint(points: &Vec<BigUint>, scalars: &Vec<BigUint>, size: usize) -> ([BigUint; 3],Duration, u8) {
    let nof_elements: usize = size;
    let chunks: usize = div_up(nof_elements,CHUNK_SIZE);
    println!("Open Device Channels...");
    let axi = open_axi_channel();
    let h2c = open_dma_channel();
    println!("Task label: {}", get_ingo_msm_task_label(&axi)[0]);
    println!("Format Inputs...");
    let points_bytes = get_formatted_unified_points_from_biguint(points);  
    let scalars_bytes = get_formatted_unified_scalars_from_biguint(scalars);
    println!("Setting DMA Source...");
    set_ingo_msm_coeffs_source(&axi,0);
    set_ingo_msm_bases_source(&axi,0);
    println!("Setting NOF Elements  = {}...", nof_elements);
    set_ingo_msm_nof_elements(&axi, nof_elements);
    println!("Pushing Task Signal...");
    set_ingo_msm_push_task(&axi);
    println!("Writing Task...");
    let start = Instant::now();
    write_msm_to_fifo(&points_bytes, &scalars_bytes, h2c,chunks);
    println!("Waiting for result...");
    wait_for_valid_result(&axi);
    let duration = start.elapsed();
    println!("Result label: {}", get_ingo_msm_result_label(&axi)[0]);
    let result_label = get_ingo_msm_result_label(&axi)[0]; 
    println!("Received result...");
    println!("Time elapsed is: {:?} for size: {}", duration, nof_elements);
    let result = read_result(&axi);
    let z_chunk = &result[0..BYTE_SIZE_POINT_COORD];
    let y_chunk = &result[BYTE_SIZE_POINT_COORD..BYTE_SIZE_POINT_COORD*2];
    let x_chunk = &result[BYTE_SIZE_POINT_COORD*2..];
    println!("X bytes {:02X?}", x_chunk);
    println!("Y bytes {:02X?}", y_chunk);
    println!("Z bytes {:02X?}", z_chunk);
    println!("Pop result...");
    set_ingo_msm_pop_task(axi);
    return ([BigUint::from_bytes_le(x_chunk),BigUint::from_bytes_le(y_chunk),BigUint::from_bytes_le(z_chunk)],duration,result_label)
}

pub fn write_points_to_hbm(points: &Vec<u32>, size: usize) -> () {
    println!("Format Input...");
    let points_bytes = get_formatted_unified_points_from_u32(points);  
    println!("Open Device Channels...");
    let axi = open_axi_channel();
    let h2c = open_dma_channel();
    set_ingo_msm_bases_source(&axi,1);
    h2c.write_all_at(&points_bytes, 0x0000_0000_0000_0000).expect("Faild to write to dma");
}

pub fn msm_calc_u32_only_scalars(scalars: &Vec<u32>, size: usize) -> ([Vec<u32> ;3],Duration) {
    println!("Format Input...");
    let scalars_bytes = get_formatted_unified_scalars_from_u32(scalars);
    let (duration, z_chunk, y_chunk, x_chunk) = msm_core_scalar_only(size, scalars_bytes);
    return ([as_u32_le(x_chunk),as_u32_le(y_chunk),as_u32_le(z_chunk)],duration)
}

fn msm_core_scalar_only(size: usize, scalars_bytes: Vec<u8>) -> (Duration, Vec<u8>, Vec<u8>, Vec<u8>) {
    let nof_elements: usize = size;
    let chunks: usize = div_up(nof_elements,CHUNK_SIZE);
    println!("Open Device Channels...");
    let axi = open_axi_channel();
    let h2c = open_dma_channel();
    println!("Setting DMA Source...");
    set_ingo_msm_coeffs_source(&axi,0);
    println!("Setting NOF Elements  = {}...", nof_elements);
    set_ingo_msm_nof_elements(&axi, nof_elements);
    println!("Pushing Task Signal...");
    set_ingo_msm_push_task(&axi);
    println!("Task label: {}", get_ingo_msm_task_label(&axi)[0]);
    println!("Writing Task...");
    let start = Instant::now();
    write_scalar_only_to_fifo(scalars_bytes, h2c,chunks);
    println!("Waiting for result...");
    wait_for_valid_result(&axi);
    let duration = start.elapsed();
    println!("Result label: {}", get_ingo_msm_result_label(&axi)[0]);
    println!("Received result...");
    println!("Time elapsed is: {:?} for size: {}", duration, nof_elements);
    let result = read_result(&axi);
    let z_chunk = &result[0..48];
    let y_chunk = &result[48..96];
    let x_chunk = &result[96..144];
    println!("X bytes {:02X?}", x_chunk);
    println!("Y bytes {:02X?}", y_chunk);
    println!("Z bytes {:02X?}", z_chunk);
    println!("Pop result...");
    set_ingo_msm_pop_task(axi);
    (duration, z_chunk.to_vec(), y_chunk.to_vec(), x_chunk.to_vec())
}

pub fn quick_pop(){
    let axi = open_axi_channel();
    set_ingo_msm_pop_task(axi);
}

pub fn check_if_points_are_on_curv(point: &Vec<u8>) -> bool {
    assert_eq!(point.len(), 96);

    let y = Fq::from(BigUint::from_bytes_le(&point[0.. 32]));
    let x = Fq::from(BigUint::from_bytes_le(&point[32.. 64]));

    let point = GAffine::new(x, y, false);

    return point.is_on_curve();
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
pub fn msm_core(point_bytes: &[u8], scalar_bytes: &[u8],size: usize) -> (Vec<u8>,Duration,u8) {
    let nof_elements: usize = size;
    let chunks: usize = div_up(nof_elements,CHUNK_SIZE);
    // println!("points: {:?}", n_scalars_bytes);
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
    write_msm_to_fifo(&point_bytes, &scalar_bytes, h2c,chunks);
    println!("Waiting for result...");
    wait_for_valid_result(&axi);
    let duration = start.elapsed();
    println!("Received result...");
    println!("Time elapsed is: {:?} for size: {}", duration, nof_elements);
    let result = read_result(&axi);
    let result_label = get_ingo_msm_result_label(&axi)[0]; 
    println!("Result label: {}", result_label);
    let z_chunk = &result[0..BYTE_SIZE_POINT_COORD];
    let y_chunk = &result[BYTE_SIZE_POINT_COORD..2 * BYTE_SIZE_POINT_COORD];
    let x_chunk = &result[2 * BYTE_SIZE_POINT_COORD..3 * BYTE_SIZE_POINT_COORD];
    // println!("X bytes {:02X?}", x_chunk);
    // println!("Y bytes {:02X?}", y_chunk);
    // println!("Z bytes {:02X?}", z_chunk);
    // println!("Pop result...");
    set_ingo_msm_pop_task(axi);
    //is_projective_point_curve(z_chunk.to_vec(), y_chunk.to_vec(), x_chunk.to_vec());
    let mut result_vector = Vec::new();
    result_vector.extend(z_chunk.to_vec());
    result_vector.extend(y_chunk.to_vec());
    result_vector.extend(x_chunk.to_vec());


    (result_vector,
    duration, result_label)
}

fn is_projective_point_curve(z_chunk: Vec<u8>, y_chunk: Vec<u8>, x_chunk: Vec<u8>) {
    let x = Fq::from(BigUint::from_bytes_le(&x_chunk));
    let y = Fq::from(BigUint::from_bytes_le(&y_chunk));
    let z = Fq::from(BigUint::from_bytes_le(&z_chunk));

    let inverse_z = Fq::inverse(&z).unwrap();

    let n_x = Fq::mul(x, inverse_z);
    let n_y = Fq::mul(y, inverse_z);

    let point = GAffine::new(n_x, n_y, false);

    println!("IS FINAL POINT ON CURV: {:?}", point.is_on_curve());
    println!("{}", n_x.to_string());
}

fn read_result(axi: &std::fs::File) -> Vec<u8> {
    let mut result: Vec<u8> = Vec::new();
    for i in 0..36{
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
    let payload_size_points: usize = CHUNK_SIZE * 32 * 2;
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


fn set_ingo_msm_pop_task(axi: std::fs::File) {
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

fn get_formatted_unified_scalars_from_u32(scalars: &Vec<u32>) -> Vec<u8> {
    let mut scalars_bytes: Vec<u8> = Vec::new();
    for i in 0..scalars.len()/8{
        let mut bytes_array = u32_vec_to_u8_vec(&scalars[i*8 .. i*8 + 8].to_vec());
        bytes_array.extend(std::iter::repeat(0).take(BYTE_SIZE_SCALAR-bytes_array.len()));
        scalars_bytes.extend(bytes_array);
    }
    scalars_bytes
}

fn get_formatted_unified_points_from_u32(points: &Vec<u32>) -> Vec<u8> {
    let mut points_bytes: Vec<u8> = Vec::new();
    const PAD_SIZE: usize = BYTE_SIZE_POINT_COORD / 4;

    for i in 0..points.len() / PAD_SIZE {
        let mut bytes_array = u32_vec_to_u8_vec(&points[i * PAD_SIZE..i * PAD_SIZE + PAD_SIZE].to_vec());
        bytes_array.extend(std::iter::repeat(0).take(BYTE_SIZE_POINT_COORD - bytes_array.len()));
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