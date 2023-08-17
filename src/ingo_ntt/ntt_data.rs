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
    // In essence, the HBM is subdivided into two rows and two columns.
    // The two rows account for the HBM double buffer and
    // the two columns account for the left and right NTTC sides.
    pub const NTT_BUFFER_SIZE: usize = 268435456; // 2**28 - size of one buffer into HBM

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
    const NTT_SIZE: usize = 134217728; // Size of NTT = 2**27
    const NTT_WORD_SIZE: usize = 32; // Size of one element in NTT in bytes
    const NTT_NOF_MMU_IN_CORE: usize = 8; // Number of MMUs into which one subNTT splits into

    // The NTT data (corresponding to a single buffer) consists of 512 Groups (NTT_NOF_GROUPS),
    // each Group consisting of two Slices (NTT_NOF_SLICE),
    // each Slice consisting of 16 Batches (NTT_NOF_BATCH),
    // and each Batch consisting of 16 subNTTs (NTT_NOF_SUBNTT),
    // each subNTTs consisting of 64 rows (NTT_NOF_ROW).
    const NTT_NOF_GROUPS: usize = 512;
    const NTT_NOF_SLICE: usize = 2;
    const NTT_NOF_BATCH: usize = 16;
    const NTT_NOF_SUBNTT: usize = 8;
    const NTT_NOF_ROW: usize = 64;

    pub(super) fn preprocess(input: Vec<u8>) -> Self {
        log::info!("Start preparing the input vector before NTT");
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
            log::trace!("Group {} is ready", group)
        }

        NTTBanks {
            banks: banks.try_into().unwrap(),
        }
    }

    pub(super) fn postprocess(&self) -> Vec<u8> {
        log::info!("Start processing the result after NTT");
        let mut res = vec![0u8; Self::NTT_SIZE * Self::NTT_WORD_SIZE];
        log::debug!("Allocate vector of size: {}", res.len());

        let mut group_start = [0, 0];
        let offset = [0, 512];
        let mut bank_offset = vec![0usize; 16];
        for group in 0..Self::NTT_NOF_GROUPS {
            let mut block = 0;
            for i in 0..2 {
                group_start[i] = offset[i] + group;
            }
            for _ in 0..Self::NTT_NOF_SLICE {
                for _ in 0..Self::NTT_NOF_BATCH {
                    for _ in 0..Self::NTT_NOF_SUBNTT {
                        for icore in 0..2 {
                            let isubntt = group_start[icore] + 1024 * block;
                            let mut i = 0;
                            for _ in 0..Self::NTT_NOF_ROW {
                                let cores = if group % 2 == 0 {
                                    [[0, 1, 2, 3, 4, 5, 6, 7], [8, 9, 10, 11, 12, 13, 14, 15]]
                                } else {
                                    [[8, 9, 10, 11, 12, 13, 14, 15], [0, 1, 2, 3, 4, 5, 6, 7]]
                                };
                                for bank_num in cores[icore].into_iter() {
                                    let addr = 512 * isubntt + i;
                                    res[addr * 32..(addr + 1) * 32].copy_from_slice(
                                        &self.banks[bank_num]
                                            [bank_offset[bank_num]..bank_offset[bank_num] + 32],
                                    );
                                    bank_offset[bank_num] += 32;
                                    i += 1;
                                }
                            }
                        }
                        block += 1;
                    }
                }
            }
            log::trace!("Group {} is ready", group)
        }
        res
    }
}

#[cfg(test)]
mod tests {
    use std::{env, fs::File, io::Read};

    use super::{NTTBanks, NOF_BANKS};

    #[test]
    fn preprocess_correctness() {
        let fdir = env::var("FDIR").unwrap();
        let exp = already_preprocess(fdir);
        let fname = env::var("FNAME").unwrap();
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

    fn already_preprocess(fdir: String) -> Vec<Vec<u8>> {
        let mut banks: Vec<Vec<u8>> = Vec::with_capacity(NOF_BANKS);
        for _ in 0..NOF_BANKS {
            banks.push(Default::default());
        }
        for (i, bank) in banks.iter_mut().enumerate().take(16) {
            let fname = format!("{}/inbank{:02}.dat", fdir, i);
            println!("Read {}", fname);
            let mut f = File::open(&fname).expect("no file found");
            let _ = f.read_to_end(bank);
        }

        banks
    }

    #[test]
    fn postprocess_correctness() {
        let fdir = env::var("FDIR").unwrap();
        let fname = env::var("FNAME").unwrap();
        let in_banks: Vec<Vec<u8>> = already_postprocess(fdir);
        let ntt_banks = NTTBanks {
            banks: in_banks.try_into().unwrap(),
        };
        let got = ntt_banks.postprocess();
        println!("Got result of size: {}", got.len());

        let mut f = File::open(&fname).expect("no file found");
        let mut exp_out_vec: Vec<u8> = Default::default();
        let _ = f.read_to_end(&mut exp_out_vec);
        println!("Result is read from: {}", fname);

        if exp_out_vec.eq(&got) {
            println!("Result is correct");
        }
    }

    fn already_postprocess(fdir: String) -> Vec<Vec<u8>> {
        let mut banks: Vec<Vec<u8>> = Vec::with_capacity(NOF_BANKS);
        for _ in 0..NOF_BANKS {
            banks.push(Default::default());
        }
        for (i, bank) in banks.iter_mut().enumerate().take(16) {
            let fname = format!("{}/outbank{:02}.dat", fdir, i);
            println!("Read {}", fname);
            let mut f = File::open(&fname).expect("no file found");
            let _ = f.read_to_end(bank);
        }

        banks
    }
}
