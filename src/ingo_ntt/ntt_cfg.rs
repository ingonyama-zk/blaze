use std::{fs::File, io::Read};

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
    const NTT_WORD_SIZE: usize = 32;
    const NTT_NOF_MMU_IN_CORE: usize = 8;

    const NTT_NOF_GROUPS: usize = 512;
    const NTT_NOF_SLICE: usize = 2;
    const NTT_NOF_BATCH: usize = 16;
    const NTT_NOF_SUBNTT: usize = 8;
    const NTT_NOF_ROW: usize = 64;

    pub(super) fn preprocess(input: Vec<u8>) -> Self {
        let mut banks: Vec<Vec<u8>> = Vec::with_capacity(NOF_BANKS);
        for _ in 0..NOF_BANKS {
            banks.push(Default::default());
        }
        let mut addr = 0;
        for group in 0..Self::NTT_NOF_GROUPS {
            for _ in 0..Self::NTT_NOF_SLICE {
                for _ in 0..Self::NTT_NOF_BATCH {
                    for _ in 0..Self::NTT_NOF_SUBNTT {
                        for cores in [[0, 1, 2, 3, 4, 5, 6, 7], [8, 9, 10, 11, 12, 13, 14, 15]] {
                            for row in 0..Self::NTT_NOF_ROW {
                                for bank_num in cores.into_iter() {
                                    let buf: &[u8] = &input[(bank_num % 8 + row * 8) * 32 + addr
                                        ..(bank_num % 8 + row * 8 + 1) * 32 + addr];
                                    banks[bank_num].extend_from_slice(buf);
                                }
                            }
                            addr +=
                                Self::NTT_NOF_MMU_IN_CORE * Self::NTT_NOF_ROW * Self::NTT_WORD_SIZE;
                        }
                    }
                }
            }
            log::debug!("Group {} is ready", group)
        }

        NTTBanks {
            banks: banks.try_into().unwrap(),
        }
    }

    pub(super) fn postprocess(&self) -> Vec<u8> {
        for vb in self.banks.iter() {
            log::info!("Length: {}", vb.len());
        }
        // TODO: in postprocessing now result is checking with ready data
        check_result(self.banks.to_vec());
        Default::default()
    }
}

fn check_result(banks: Vec<Vec<u8>>) {
    for (i, bank) in banks.iter().enumerate().take(16) {
        let mut banks_exp: Vec<u8> = Default::default();
        let fname = format!(
            "/home/administrator/ekaterina/blaze/tests/test_data/outbank{:02}.dat",
            i
        );

        println!("{}", fname);
        let mut f = File::open(&fname).expect("no file found");
        let _ = f.read_to_end(&mut banks_exp);

        if banks_exp.eq(bank) {
            log::info!("Bank {} is correct", i);
        }
    }
}
#[cfg(test)]
mod tests {
    use std::{fs::File, io::Read};

    use super::{NTTBanks, NOF_BANKS};

    #[test]
    fn preprocess_correctness() {
        let exp = already_preprocess();
        let fname =
            "/home/administrator/ekaterina/blaze/tests/test_data/in_prepare.dat".to_string();
        let mut f = File::open(&fname).expect("no file found");
        let mut in_vec: Vec<u8> = Default::default();
        let _ = f.read_to_end(&mut in_vec);
        let got = NTTBanks::preprocess(in_vec);

        for (i, expb) in exp.iter().enumerate().take(16) {
            if got.banks[i].eq(expb) {
                println!("Bank {} is correct", i);
            }
        }
    }

    fn already_preprocess() -> Vec<Vec<u8>> {
        let mut banks: Vec<Vec<u8>> = Vec::with_capacity(NOF_BANKS);
        for _ in 0..NOF_BANKS {
            banks.push(Default::default());
        }
        for (i, bank) in banks.iter_mut().enumerate().take(16) {
            let fname = format!(
                "/home/administrator/ekaterina/blaze/tests/test_data/inbank{:02}.dat",
                i
            );
            println!("Read {}", fname);
            let mut f = File::open(&fname).expect("no file found");
            let _ = f.read_to_end(bank);
        }

        banks
    }
}
