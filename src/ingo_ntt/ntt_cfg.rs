use num_bigint::BigInt;
use std::{
    env,
    fs::{read_to_string, File},
    io::{BufReader, Read},
};

pub(super) const NOF_BANKS: usize = 16;

#[derive(Debug, Copy, Clone)]
pub(super) struct NTTAddrs {
    pub hbm_ss_baseaddr: u64,
    pub hbm_addrs: [u64; NOF_BANKS],
}

fn ntt_addrs() -> NTTAddrs {
    NTTAddrs {
        hbm_ss_baseaddr: 0x0,
        hbm_addrs: [
            0x000000000,
            0x020000000,
            0x040000000,
            0x060000000,
            0x080000000,
            0x0A0000000,
            0x0C0000000,
            0x0E0000000,
            0x100000000,
            0x120000000,
            0x140000000,
            0x160000000,
            0x180000000,
            0x1A0000000,
            0x1C0000000,
            0x1E0000000,
        ],
    }
}

#[derive(Debug, Copy, Clone)]
pub(super) struct NTTConfig {
    pub ntt_addrs: NTTAddrs,
}

impl NTTConfig {
    // const HBM_BANK_SIZE: u32 = 536870912; // 2**29
    // const NTT_WORD_SIZE: u32 = 32;
    // const NTT_BUFFER_SIZE_WORDS: u32 = 8388608; // ntt_buffer_size//ntt_word_size
    pub const NTT_BUFFER_SIZE: usize = 268435456; // ntt_buffer_size = hbm_bank_size // 2

    pub fn ntt_cfg() -> Self {
        NTTConfig {
            ntt_addrs: ntt_addrs(),
        }
    }

    pub(super) fn hbm_bank_start_addr(&self, bank_num: usize) -> u64 {
        *self.ntt_addrs.hbm_addrs.get(bank_num).unwrap()
    }

    pub(super) fn ntt_bank_start_addr(&self, bank_num: usize, buf_num: usize) -> u64 {
        self.hbm_bank_start_addr(bank_num) + (Self::NTT_BUFFER_SIZE * buf_num) as u64
    }
}

#[derive(Debug, Clone, Default)]
pub(super) struct NTTBanks {
    pub banks: [Vec<u8>; NOF_BANKS],
}

impl NTTBanks {
    const NTT_WORD_SIZE: u32 = 32;

    pub(super) fn preprocess(input: Vec<u8>, fname: String) -> Self {
        let mut banks: Vec<Vec<u8>> = Vec::with_capacity(NOF_BANKS);
        for _ in 0..NOF_BANKS {
            banks.push(Default::default());
        }
        // for line in read_to_string(fname).unwrap().lines() {
            
        // }
        // let mut lines = read_to_string(fname).unwrap().lines() ;
        // for group in 0..512 {
        //     for slice in 0..2 {
        //         for batch in 0..16 {
        //             for subntt in 0..8 {
        //                 for cores in [[0usize..8], [8..16]] {
        //                     for row in 0..64 {
        //                         for bank_num in cores.clone().into_iter() {
        //                             let element = BigInt::parse_bytes(lines.next().unwrap().as_bytes(), 10)
        //                                 .unwrap()
        //                                 .to_bytes_le()
        //                                 .1;
        //                             banks[bank_num][0].extend(element);
        //                         }
        //                     }
        //                 }
        //             }
        //         }
        //     }
        //     println!("Group {} is ready", group)
        // }
        // BigInt::parse_bytes(n, 10).unwrap().to_bytes_le().1

        NTTBanks {
            banks: already_preprocess().try_into().unwrap(),
        }
    }

    pub(super) fn postprocess(&self) -> Vec<u8> {
        // for group in 0..512 {
        //     for slice in 0..2 {
        //         for batch in 0..16 {
        //             for subntt in 0..8 {
        //                 for cores in self.banks.chunks(8) {
        //                     for row in 0..64 {
        //                         for bank in cores {}
        //                     }
        //                 }
        //             }
        //         }
        //     }
        // }
        // self.banks
        //     .iter()
        //     .flat_map(|v| v.to_vec())
        //     .collect::<Vec<u8>>()

        for vb in self.banks.iter() {
            log::info!("Length: {}", vb.len());
        }
        check_result(self.banks.to_vec());
        Default::default()
    }
}

fn already_preprocess() -> Vec<Vec<u8>> {
    let mut banks: Vec<Vec<u8>> = Vec::with_capacity(NOF_BANKS);
    for _ in 0..NOF_BANKS {
        banks.push(Default::default());
    }
    for i in 0..16 {
        let fname = format!(
            "/home/administrator/ekaterina/blaze/tests/test_data/inbank{:02}.dat",
            i
        );

        println!("{}", fname);
        let mut f = File::open(&fname).expect("no file found");
        f.read_to_end(&mut banks[i]);
    }

    banks
}

fn check_result(banks: Vec<Vec<u8>>) {
    for i in 0..16 {
        let mut banks_exp: Vec<u8> = Default::default();
        let fname = format!(
            "/home/administrator/ekaterina/blaze/tests/test_data/outbank{:02}.dat",
            i
        );

        println!("{}", fname);
        let mut f = File::open(&fname).expect("no file found");
        f.read_to_end(&mut banks_exp);

        if banks_exp == banks[i] {
            log::info!("Bank {} is correct", i);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::env;

    use num_bigint::BigInt;
    use serde_json::from_str;

    use super::{already_preprocess, NTTBanks};

    #[test]
    fn it_works() {
        let exp = already_preprocess();
        let got = NTTBanks::preprocess(
            vec![0; 0],
            "/home/administrator/ekaterina/blaze/tests/test_data/in.txt".to_string(),
        );
        for i in 0..16 {
            if exp[i] == got.banks[i] {
                println!("Bank {} is correct", i);
            }
        }
    }
}
