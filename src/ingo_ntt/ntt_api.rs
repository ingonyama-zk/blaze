use crate::{
    driver_client::dclient::*, error::*, ingo_ntt::ntt_hw_code::*, ingo_ntt::ntt_utils::*,
    utils::deserialize_hex, utils::read_binary_file,
};
use packed_struct::prelude::*;
use serde::{Deserialize, Deserializer};
use std::{fmt::Debug, fs::File, io::Write, os::unix::fs::FileExt, str::FromStr};

pub const NOF_ADDRS: usize = 16;

#[derive(Deserialize, Debug, Copy, Clone)]
pub struct NTTAddrs {
    #[serde(deserialize_with = "deserialize_hex")]
    pub hbm_ss_baseaddr: u64,
    #[serde(deserialize_with = "deserialize_hex_array")]
    pub mmu_addrs: [u64; NOF_ADDRS],
    #[serde(deserialize_with = "deserialize_hex_array")]
    pub hbm_addrs: [u64; NOF_ADDRS],
}

#[derive(Deserialize, Debug, Copy, Clone)]
pub struct NTTParams {
    /// Number of MMU
    pub nof_mmu_per_nttc: u32,
    pub nof_nttc: u32,

    // HBM address in *bytes*
    pub hbm_bank_size: u32,
    pub ntt_word_size: u32,

    // FPGA addresses memory in 32B *words*
    // suffix _words indicates the address is in 32B *words*
    #[serde(skip)]
    pub ntt_buffer_size_words: u32,
    #[serde(skip)]
    pub ntt_buffer_size: u32,
}

pub fn deserialize_ntt_params<'de, D>(deserializer: D) -> std::result::Result<NTTParams, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    pub struct NTTParamsDTO {
        nof_mmu_per_nttc: u32,
        nof_nttc: u32,
        hbm_bank_size: u32,
        ntt_word_size: u32,
    }

    let source = NTTParamsDTO::deserialize(deserializer)?;

    let ntt_buf_size = source.hbm_bank_size / 2;
    Ok(NTTParams {
        nof_mmu_per_nttc: source.nof_mmu_per_nttc,
        nof_nttc: source.nof_nttc,
        hbm_bank_size: source.hbm_bank_size,
        ntt_word_size: source.ntt_word_size,
        ntt_buffer_size: ntt_buf_size,
        ntt_buffer_size_words: ntt_buf_size / source.ntt_word_size,
    })
}

#[derive(Deserialize, Debug, Copy, Clone)]
pub struct NTTConfig {
    pub ntt_addrs: NTTAddrs,
    #[serde(deserialize_with = "deserialize_ntt_params")]
    pub ntt_params: NTTParams,
}

impl NTTConfig {
    fn ntt_cfg() -> Self {
        let file = std::fs::File::open("configs/ntt_cfg.json").expect("");
        let reader = std::io::BufReader::new(file);
        serde_json::from_reader(reader).unwrap()
    }

    pub fn hbm_bank_start_addr(&self, bank_num: usize) -> u64 {
        *self.ntt_addrs.hbm_addrs.get(bank_num).unwrap()
    }

    pub fn ntt_buffer_start_addr(&self, bank_num: usize, buf_num: u32) -> u64 {
        self.hbm_bank_start_addr(bank_num) + (self.ntt_params.ntt_buffer_size * buf_num) as u64
    }
}

pub struct NTTClient {
    pub ntt_cfg: NTTConfig,
    pub driver_client: DriverClient,
}
pub enum NTT {
    Ntt,
}

pub struct NttInit {
    pub enable_debug_program: u32,
    pub debug_program: Vec<u64>,
}

#[derive(Deserialize, Debug, Clone)]
// DEBUG DATA
pub struct NTTInput {
    pub buf_num: u8,
}

#[derive(Deserialize, Debug, Clone)]
// DEBUG DATA
pub struct NTTOutput {
    pub info_read: Vec<u32>,
    pub info_write: Vec<u32>,
    pub timer_read: Vec<u32>,
    pub timer_write: Vec<u32>,
    pub info_program: Option<u32>,
}

impl DriverPrimitive<NTT, NttInit, NTTInput, NTTOutput> for NTTClient {
    fn new(_ptype: NTT, dclient: DriverClient) -> Self {
        NTTClient {
            ntt_cfg: NTTConfig::ntt_cfg(),
            driver_client: dclient,
        }
    }

    fn loaded_binary_parameters(&self) -> Vec<u32> {
        todo!()
    }

    fn reset(&self) -> Result<()> {
        todo!()
    }

    fn initialize(&self, input: NttInit) -> Result<()> {
        self.hbm_ss_set_debug_input(input)
    }

    fn start_process(&self) -> Result<()> {
        self.driver_client.ctrl_write_u32(
            self.ntt_cfg.ntt_addrs.hbm_ss_baseaddr,
            INGO_NTT_SUPER_PROGRAM_ADDR::XHBM_SS_CONTROL_ADDR_AP_CTRL,
            1,
        )
    }

    fn set_data(&self, _input: NTTInput) -> Result<()> {
        todo!()
    }

    fn wait_result(&self) -> Result<()> {
        let mut result_valid = [0, 0, 0, 0];
        let mut done = false;
        log::debug!("Waiting ready signal from offset: XHBM_SS_CONTROL_ADDR_AP_CTRL");
        while !done {
            self.driver_client
                .ctrl
                .read_exact_at(
                    &mut result_valid,
                    self.driver_client.cfg.ctrl_baseaddr
                        + INGO_NTT_SUPER_PROGRAM_ADDR::XHBM_SS_CONTROL_ADDR_AP_CTRL as u64,
                )
                .map_err(|e| DriverClientError::ReadError {
                    offset: "XHBM_SS_CONTROL_ADDR_AP_CTRL".to_string(),
                    source: e,
                })?;
            done = (result_valid[0] & 0x2) == 0x2;
            self.hbm_ss_debug_volatile()?;
        }
        Ok(())
    }

    fn result(&self, _param: Option<usize>) -> Result<Option<NTTOutput>> {
        let res = self.hbm_ss_debug_output()?;
        Ok(Some(res))
    }
}

impl NTTClient {
    fn debug_value(&self, info_type: &str, mmu: u32) -> Result<u32> {
        let ctrl = self.driver_client.ctrl_read_u32(
            self.ntt_cfg.ntt_addrs.hbm_ss_baseaddr,
            INGO_NTT_SUPER_PROGRAM_ADDR::from_str(&format!(
                "XHBM_SS_CONTROL_ADDR_{}{:02}_CTRL",
                info_type, mmu
            ))
            .unwrap(),
        )?;

        let data = self.driver_client.ctrl_read_u32(
            self.ntt_cfg.ntt_addrs.hbm_ss_baseaddr,
            INGO_NTT_SUPER_PROGRAM_ADDR::from_str(&format!(
                "XHBM_SS_CONTROL_ADDR_{}{:02}_DATA",
                info_type, mmu
            ))
            .unwrap(),
        )?;

        log::debug!(
            "{}:\n mmu:{}\n ctrl:{}\n info:{:X?}",
            info_type,
            mmu,
            ctrl,
            data
        );
        Ok(data)
    }

    fn hbm_ss_debug_volatile(&self) -> Result<NTTOutput> {
        let info_program = self.driver_client.ctrl_read_u32(
            self.ntt_cfg.ntt_addrs.hbm_ss_baseaddr,
            INGO_NTT_SUPER_PROGRAM_ADDR::XHBM_SS_CONTROL_ADDR_INFO_PROGRAM_DATA,
        )?;

        let mut info_read = Vec::new();
        let mut info_write = Vec::new();
        // let mut timer_read = Vec::new();
        // let mut timer_write = Vec::new();

        for mmu in 0u32..16 {
            info_read.push(self.debug_value("INFO_WRITE", mmu)?);
            info_write.push(self.debug_value("INFO_READ", mmu)?);
            // timer_write.push(self.debug_value("TIMER_WRITE", mmu)?);
            // timer_read.push(self.debug_value("TIMER_READ", mmu)?);
        }
        Ok(NTTOutput {
            info_read,
            info_write,
            timer_read: vec![0; 0],
            timer_write: vec![0; 0],
            info_program: Some(info_program),
        })
    }

    fn hbm_ss_set_debug_input(&self, input: NttInit) -> Result<()> {
        // 0x00 (defaul - complete) or 0xFF (debug - partial)
        self.driver_client.ctrl_write_u32(
            self.ntt_cfg.ntt_addrs.hbm_ss_baseaddr,
            INGO_NTT_SUPER_PROGRAM_ADDR::XHBM_SS_CONTROL_ADDR_DEBUG_INPUT_ENABLE_DEBUG_PROGRAM_DATA,
            input.enable_debug_program,
        )?;

        for (i, input) in input.debug_program.iter().enumerate() {
            let size = 8; //
            self.driver_client.ctrl_write(
                self.ntt_cfg.ntt_addrs.hbm_ss_baseaddr + (size * i as u64),
                INGO_NTT_SUPER_PROGRAM_ADDR::XHBM_SS_CONTROL_ADDR_DEBUG_INPUT_DEBUG_PROGRAM_BASE,
                &input.to_le_bytes(),
            )?;
        }

        Ok(())
    }

    fn hbm_ss_debug_output(&self) -> Result<NTTOutput> {
        let mut info_read = Vec::new();
        let mut info_write = Vec::new();
        let mut timer_read = Vec::new();
        let mut timer_write = Vec::new();

        for mmu in 0u32..16 {
            info_read.push(self.debug_value("INFO_WRITE", mmu)?);
            info_write.push(self.debug_value("INFO_READ", mmu)?);
            timer_read.push(self.debug_value("TIMER_READ", mmu)?);
            timer_write.push(self.debug_value("TIMER_WRITE", mmu)?);
        }
        Ok(NTTOutput {
            info_read,
            info_write,
            timer_read,
            timer_write,
            info_program: None,
        })
    }

    pub fn load_group(&self, buf_num: u32, filename: &str, stage: u32, group: u32) -> Result<()> {
        log::info!("Load group: {:?}", filename);

        let page = self.ntt_cfg.ntt_params.ntt_word_size * 2 * 32;
        let data = read_binary_file(filename)?;

        let array: Vec<&[u8]> = data.chunks(page as usize).collect();
        let mut i = 0;
        for mmu in 0..16 {
            let core = if mmu < 8 { 0 } else { 1 };
            let mut group2 = group;
            let mut core2 = core;
            if stage == 1 {
                group2 = group >> 1;
                if (group2 & 1) == 1 {
                    core2 = core ^ 1;
                }
            }
            for slice in 0..2 {
                for batch in 0..16 {
                    for batch_subntt in 0..8 {
                        let params = SubNTTParams {
                            group2: group2 as u16,
                            slice,
                            batch,
                            subntt_per_nttc: batch_subntt,
                        };
                        let subntt = params.compute_subntt(stage as usize, core2);
                        let offset =
                            self.ntt_cfg.ntt_buffer_start_addr(mmu, buf_num) + (64 * subntt as u64);
                        self.driver_client.dma_write(
                            self.driver_client.cfg.dma_baseaddr,
                            offset,
                            array[i],
                        )?;
                        i += 1;
                    }
                }
            }
        }

        Ok(())
    }

    pub fn store_group(&self, buf_num: u32, filename: &str, stage: u32, group: u32) -> Result<()> {
        log::info!("Store group: {:?}", filename);

        let page = self.ntt_cfg.ntt_params.ntt_word_size * 2 * 32;
        let mut file = File::open(filename)?;

        for mmu in 0..16 {
            let core = if mmu < 8 { 0 } else { 1 };
            let mut group2 = group;
            let mut core2 = core;
            if stage == 1 {
                group2 = group >> 1;
                if (group2 & 1) == 1 {
                    core2 = core ^ 1;
                }
            }
            for slice in 0..2 {
                for batch in 0..16 {
                    for batch_subntt in 0..8 {
                        let params = SubNTTParams {
                            group2: group2 as u16,
                            slice,
                            batch,
                            subntt_per_nttc: batch_subntt,
                        };
                        let subntt = params.compute_subntt(stage as usize, core2);
                        let offset =
                            self.ntt_cfg.ntt_buffer_start_addr(mmu, buf_num) + (64 * subntt as u64);
                        let res = self.driver_client.dma_read(
                            self.driver_client.cfg.dma_baseaddr,
                            offset,
                            page as usize,
                        )?;
                        file.write_all(res.as_slice())?;
                    }
                }
            }
        }

        Ok(())
    }
}

#[derive(PackedStruct, Debug)]
#[packed_struct(bit_numbering = "msb0")]
pub struct SubNTTParams {
    #[packed_field(bits = "23..=31", endian = "lsb")]
    pub group2: u16,
    #[packed_field(bits = "22", endian = "lsb")]
    pub slice: u8,
    #[packed_field(bits = "18..=21", endian = "lsb")]
    pub batch: u8,
    #[packed_field(bits = "15..=17", endian = "lsb")]
    pub subntt_per_nttc: u8,
}

impl SubNTTParams {
    pub fn compute_subntt(&self, stage: usize, core: usize) -> u32 {
        const NOF_SUBNTT_PER_MMU_BITS: u32 = 17;
        let subntt_pattern = [[[0u32, 0], [0, 0]], [[0, 9], [256, 9]], [[0, 0], [0, 0]]];

        let offset = subntt_pattern[stage][core][0];
        let stride_bits = subntt_pattern[stage][core][0];

        let buf = SubNTTParams::pack(self).unwrap();
        let counter = u32::from_le_bytes([0, buf[2], buf[1], buf[0]]);
        offset + (counter << stride_bits) + (counter >> (NOF_SUBNTT_PER_MMU_BITS - stride_bits))
    }
}
