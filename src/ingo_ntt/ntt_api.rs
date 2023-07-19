use super::{
    ntt_cfg::{NTTBanks, NTTConfig, NOF_BANKS},
    ntt_hw_code::*,
};
use crate::{driver_client::*, error::*};
use std::{fmt::Debug, os::unix::fs::FileExt};

pub enum NTT {
    Ntt,
}

pub struct NTTClient {
    ntt_cfg: NTTConfig,
    pub driver_client: DriverClient,
}

pub struct NttInit {}

#[derive(Debug, Clone)]
pub struct NTTInput {
    pub buf_host: usize,
    /// Vector of size 2**27
    pub data: Vec<u8>,
}

impl DriverPrimitive<NTT, NttInit, NTTInput, Vec<u8>> for NTTClient {
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
        self.driver_client.set_dfx_decoupling(1)?;
        self.driver_client.set_dfx_decoupling(0)?;
        Ok(())
    }

    fn initialize(&self, _: NttInit) -> Result<()> {
        let enable_debug_program = 0x00;
        let debug_program: Vec<u64> = vec![
            0b1111111100000000000000000000000000000000,
            0b1111111100000000000000000000000000000000,
            // 0b1111111100000000000000000000000000000000,
            // 0b1111111100000000000000000000000000000000,
        ];

        self.driver_client.ctrl_write_u32(
            self.ntt_cfg.ntt_addrs.hbm_ss_baseaddr,
            INGO_NTT_SUPER_PROGRAM_ADDR::XHBM_SS_CONTROL_ADDR_HIF_INPUT_ENABLE_DEBUG_PROGRAM_DATA,
            enable_debug_program,
        )?;

        for (i, input) in debug_program.iter().enumerate() {
            let size = 8;
            self.driver_client.ctrl_write(
                self.ntt_cfg.ntt_addrs.hbm_ss_baseaddr + (size * i as u64),
                INGO_NTT_SUPER_PROGRAM_ADDR::XHBM_SS_CONTROL_ADDR_HIF_INPUT_DEBUG_PROGRAM_BASE,
                &input.to_le_bytes(),
            )?;
        }
        Ok(())
    }

    fn start_process(&self, buf_kernel: Option<usize>) -> Result<()> {
        self.driver_client.ctrl_write_u32(
            self.ntt_cfg.ntt_addrs.hbm_ss_baseaddr,
            INGO_NTT_SUPER_PROGRAM_ADDR::XHBM_SS_CONTROL_ADDR_HIF_INPUT_BUFFER_DATA,
            buf_kernel.unwrap().try_into().unwrap(),
        )?;

        self.driver_client.ctrl_write_u32(
            self.ntt_cfg.ntt_addrs.hbm_ss_baseaddr,
            INGO_NTT_SUPER_PROGRAM_ADDR::XHBM_SS_CONTROL_ADDR_AP_CTRL,
            1,
        )
    }

    fn set_data(&self, input: NTTInput) -> Result<()> {
        let data_banks = NTTBanks::preprocess(input.data);

        data_banks
            .banks
            .into_iter()
            .enumerate()
            .try_for_each(|(i, data_in)| {
                let offset = self.ntt_cfg.ntt_bank_start_addr(i, input.buf_host);
                self.driver_client.dma_write(
                    self.driver_client.cfg.dma_baseaddr,
                    offset,
                    data_in.as_slice(),
                )
            })
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
        }
        Ok(())
    }

    fn result(&self, buf_kernel: Option<usize>) -> Result<Option<Vec<u8>>> {
        let mut res_banks: NTTBanks = Default::default();
        for i in 0..NOF_BANKS {
            let offset = self.ntt_cfg.ntt_bank_start_addr(i, buf_kernel.unwrap());
            res_banks.banks[i] = vec![0; NTTConfig::NTT_BUFFER_SIZE];
            self.driver_client.dma_read(
                self.driver_client.cfg.dma_baseaddr,
                offset,
                &mut res_banks.banks[i],
            )?;
        }
        log::info!("Get NTT result");

        let res = res_banks.postprocess();
        Ok(Some(res))
    }
}
