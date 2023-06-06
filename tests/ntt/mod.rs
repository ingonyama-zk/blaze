use log::{debug, info};

use ingo_blaze::utils::*;

pub fn generate_test_bytes(base_number: u32, num_words: u32, word_size: u32) -> Vec<u8> {
    debug!("Number of words:{:?}", num_words);
    debug!("Word size:{:?}", word_size);
    (0..num_words)
        .flat_map(|_| u32_to_u8_vec_resize(base_number, word_size as usize, 0))
        .collect::<Vec<u8>>()
}

// use std::ops::Not;
// pub fn generate_reverse_test_bytes(base_number: u32, num_words: u32, word_size: u32) -> Vec<u8> {
//     debug!("Number of words:{:?}", num_words);
//     debug!("Word size:{:?}", word_size);
//     (0..num_words)
//         .flat_map(|_| u32_to_u8_vec_resize(base_number.not(), word_size as usize, 255))
//         .collect::<Vec<u8>>()
// }

pub fn generate_inc_test_bytes(base_number: u32, num_words: u32, word_size: u32) -> Vec<u8> {
    debug!("Number of words:{:?}", num_words);
    debug!("Word size:{:?}", word_size);
    (0..num_words)
        .flat_map(|_| u32_to_u8_vec_resize(base_number + 1, word_size as usize, 0))
        .collect::<Vec<u8>>()
}

pub fn generate_test(num_words: u32, word_size: u32, nof_mmu: u32) -> (Vec<Vec<u8>>, Vec<Vec<u8>>) {
    info!("Generate test bytes...");
    let payloads = (0..nof_mmu)
        .map(|n| generate_test_bytes(n, num_words, word_size))
        .collect::<Vec<Vec<u8>>>();

    let size_of_payloads: usize = payloads.iter().map(|payload| payload.len()).sum();
    info!("Bytes to write: {}", size_of_payloads);

    info!("Generate expected result bytes...");
    let result = (0..nof_mmu)
        .map(|n| generate_inc_test_bytes(n, num_words, word_size))
        .collect::<Vec<Vec<u8>>>();

    (payloads, result)
}
