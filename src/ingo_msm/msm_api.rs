use super::{msm_cfg::*, msm_hw_code::*};
use crate::{driver_client::*, error::*};

use packed_struct::prelude::*;
use std::{os::unix::fs::FileExt, thread::sleep, time::Duration};
use strum::IntoEnumIterator;

pub struct MSMClient {
    mem_type: PointMemoryType,
    // If precompute factor set to 1 is the basic MSM computation without optimization
    precompute_factor: u32,
    msm_cfg: MSMConfig,
    pub driver_client: DriverClient,
}

pub struct MSMInit {
    pub mem_type: PointMemoryType,
    pub is_precompute: bool,
    pub curve: Curve,
}

#[derive(Debug, Copy, Clone)]
pub struct MSMParams {
    pub nof_elements: u32,
    pub hbm_point_addr: Option<(u64, u64)>,
}

pub struct MSMInput {
    pub points: Option<Vec<u8>>,
    pub scalars: Vec<u8>,
    pub params: MSMParams,
}
#[derive(Debug, Clone)]
pub struct MSMResult {
    pub result: Vec<u8>,
    pub result_label: u32,
}

pub const PRECOMPUTE_FACTOR_BASE: u32 = 1;
pub const PRECOMPUTE_FACTOR: u32 = 8;

impl DriverPrimitive<MSMInit, MSMParams, MSMInput, MSMResult> for MSMClient {
    /// Creates a new [`MSMClient`].
    fn new(init: MSMInit, dclient: DriverClient) -> Self {
        MSMClient {
            mem_type: init.mem_type,
            precompute_factor: if init.is_precompute {
                PRECOMPUTE_FACTOR
            } else {
                PRECOMPUTE_FACTOR_BASE
            },
            msm_cfg: MSMConfig::msm_cfg(init.curve, init.mem_type),
            driver_client: dclient,
        }
    }

    fn loaded_binary_parameters(&self) -> Vec<u32> {
        [
            INGO_MSM_ADDR::ADDR_HIF2CPU_C_IMAGE_ID,
            INGO_MSM_ADDR::ADDR_HIF2CPU_C_IMAGE_PARAMTERS,
        ]
        .map(|offset| {
            self.driver_client
                .ctrl_read_u32(self.driver_client.cfg.ctrl_baseaddr, offset)
                .map_err(|_| DriverClientError::InvalidPrimitiveParam)
                .unwrap()
        })
        .into_iter()
        .collect::<Vec<u32>>()
    }

    fn reset(&self) -> Result<()> {
        self.driver_client.set_dfx_decoupling(1)?;
        self.driver_client.set_dfx_decoupling(0)?;
        sleep(Duration::from_millis(100));
        Ok(())
    }

    fn initialize(&self, params: MSMParams) -> Result<()> {
        log::info!("Start initialize driver");

        if self.mem_type == PointMemoryType::DMA && params.hbm_point_addr.is_none() {
            log::info!("Setup DMA bases");
            self.driver_client.ctrl_write_u32(
                self.driver_client.cfg.ctrl_baseaddr,
                INGO_MSM_ADDR::ADDR_CPU2HIF_C_BASES_SOURCE,
                0,
            )?;
        } else if self.mem_type == PointMemoryType::HBM || params.hbm_point_addr.is_some() {
            log::info!("Setup HBM bases");
            let addr = params.hbm_point_addr.unwrap().0;
            self.driver_client.ctrl_write_u32(
                self.driver_client.cfg.ctrl_baseaddr,
                INGO_MSM_ADDR::ADDR_CPU2HIF_C_BASES_SOURCE,
                1,
            )?;
            self.driver_client.ctrl_write(
                self.driver_client.cfg.ctrl_baseaddr,
                INGO_MSM_ADDR::ADDR_CPU2HIF_C_BASES_HBM_START_ADDRESS_LO,
                &addr.to_le_bytes(),
            )?;
        }

        self.driver_client.ctrl_write_u32(
            self.driver_client.cfg.ctrl_baseaddr,
            INGO_MSM_ADDR::ADDR_CPU2HIF_C_COEFFICIENTS_SOURCE,
            0,
        )?;

        log::info!("Set NOF Elements: {}", params.nof_elements);
        self.driver_client.ctrl_write_u32(
            self.driver_client.cfg.ctrl_baseaddr,
            INGO_MSM_ADDR::ADDR_CPU2HIF_C_NUMBER_OF_MSM_ELEMENTS,
            params.nof_elements,
        )?;

        log::info!("Pushing Task Signal");
        self.driver_client.ctrl_write_u32(
            self.driver_client.cfg.ctrl_baseaddr,
            INGO_MSM_ADDR::ADDR_CPU2HIF_E_PUSH_MSM_TASK_TO_QUEUE,
            1,
        )?;

        Ok(())
    }

    /// This function sets data for compute MSM and has three different cases depending on the input parameters.
    /// 1. DMA only mode:
    ///     - Addres for point in [`MSMConfig`].
    /// ```
    /// MSMInput = {
    ///     points: Some(points),
    ///     scalars,
    ///     nof_elements: msm_size,
    ///     hbm_point_addr: None,
    /// }
    /// ```
    /// 2. HBM mode set points to HBM and scalars by DMA:
    ///     - Points will be loaded on hbm at address hbm_addr with an offset.
    /// ```
    /// MSMInput = {
    ///     points: Some(points),
    ///     scalars,
    ///     nof_elements: msm_size,
    ///     hbm_point_addr: Some(hbm_addr, offset),
    /// }
    /// ```
    ///
    /// 3. HBM mode set only scalars:
    ///     - Points were loaded in previous iteretion on HBM.
    /// ```
    /// MSMInput = {
    ///     points: None,
    ///     scalars,
    ///     nof_elements: msm_size,
    ///     hbm_point_addr: Some(hbm_addr, offset),
    /// }
    /// ```
    ///
    fn set_data(&self, data: MSMInput) -> Result<()> {
        const CHUNK_SIZE: usize = 2048;
        let chunks = (data.params.nof_elements as usize + (CHUNK_SIZE - 1)) / CHUNK_SIZE;

        let payload_size_scalars = CHUNK_SIZE * self.msm_cfg.scalar_size;
        // Scalar addres can be loaded from configuration file or setup by user in input parametrs
        let s_addr = self.msm_cfg.dma_scalars_addr.unwrap();

        if data.points.is_none() && data.params.hbm_point_addr.is_some() {
            log::debug!("Set only scalars");
            for i in 0..chunks {
                let (s_start, s_end) = if i != chunks - 1 {
                    (i * payload_size_scalars, (i + 1) * payload_size_scalars)
                } else {
                    (i * payload_size_scalars, data.scalars.len())
                };
                let s_chunk = &data.scalars[s_start..s_end];
                self.driver_client
                    .dma_write(s_addr, DMA_RW::OFFSET, s_chunk)?;
            }
        } else if data.points.is_some() && data.params.hbm_point_addr.is_none() {
            log::debug!("Set points and scalars");
            let mut payload_size_points = CHUNK_SIZE * self.msm_cfg.point_size.unwrap();
            payload_size_points *= self.precompute_factor as usize;

            let p_addr = self.msm_cfg.dma_points_addr.unwrap();
            let p = data.points.as_ref().unwrap();

            for i in 0..chunks {
                let (p_start, p_end) = if i != chunks - 1 {
                    (i * payload_size_points, (i + 1) * payload_size_points)
                } else {
                    (i * payload_size_points, p.len())
                };
                let p_chunk = &p[p_start..p_end];

                let (s_start, s_end) = if i != chunks - 1 {
                    (i * payload_size_scalars, (i + 1) * payload_size_scalars)
                } else {
                    (i * payload_size_scalars, data.scalars.len())
                };
                let s_chunk = &data.scalars[s_start..s_end];

                self.driver_client
                    .dma_write(s_addr, DMA_RW::OFFSET, s_chunk)?;
                self.driver_client
                    .dma_write(p_addr, DMA_RW::OFFSET, p_chunk)?;
            }
        } else if data.points.is_some() && data.params.hbm_point_addr.is_some() {
            let hbm_addr = data.params.hbm_point_addr.unwrap();
            self.load_data_to_hbm(data.points.as_ref().unwrap(), hbm_addr.0, hbm_addr.1)?;

            for i in 0..chunks {
                let (s_start, s_end) = if i != chunks - 1 {
                    (i * payload_size_scalars, (i + 1) * payload_size_scalars)
                } else {
                    (i * payload_size_scalars, data.scalars.len())
                };
                let s_chunk = &data.scalars[s_start..s_end];
                self.driver_client
                    .dma_write(s_addr, DMA_RW::OFFSET, s_chunk)?;
            }
        }

        Ok(())
    }

    fn wait_result(&self) -> Result<()> {
        let mut result_valid = [0, 0, 0, 0];
        while result_valid == [0, 0, 0, 0] {
            self.driver_client
                .ctrl
                .read_exact_at(
                    &mut result_valid,
                    self.driver_client.cfg.ctrl_baseaddr
                        + INGO_MSM_ADDR::ADDR_HIF2CPU_C_RESULT_VALID as u64,
                )
                .map_err(|e| DriverClientError::ReadError {
                    offset: "ADDR_HIF2CPU_C_RESULT_VALID".to_string(),
                    source: e,
                })?;
        }
        Ok(())
    }

    fn result(&self, _param: Option<usize>) -> Result<Option<MSMResult>> {
        log::info!("Received result...");
        let mut result: Vec<u8> = Vec::new();
        // Divide size into chunks of 4 bytes
        for i in 0..(self.msm_cfg.result_point_size / 4) {
            let mut read_chunk = [1, 1, 1, 1];
            self.driver_client
                .ctrl
                .read_exact_at(
                    &mut read_chunk,
                    self.driver_client.cfg.ctrl_baseaddr
                        + INGO_MSM_ADDR::ADDR_HIF2CPU_C_RESULT as u64
                        + (i * 4) as u64,
                )
                .map_err(|e| DriverClientError::ReadError {
                    offset: format!("ADDR_HIF2CPU_C_RESULT in {:?} chunk", i * 4),
                    source: e,
                })?;
            result.extend(read_chunk);
        }
        let result_label = self.driver_client.ctrl_read_u32(
            self.driver_client.cfg.ctrl_baseaddr,
            INGO_MSM_ADDR::ADDR_HIF2CPU_C_RESULT_LABEL,
        );
        log::info!("Pop result...");
        self.driver_client.ctrl_write_u32(
            self.driver_client.cfg.ctrl_baseaddr,
            INGO_MSM_ADDR::ADDR_CPU2HIF_E_POP_RESULT,
            1,
        )?;
        Ok(Some(MSMResult {
            result,
            result_label: result_label.unwrap(),
        }))
    }
}

impl MSMClient {
    pub fn task_label(&self) -> Result<u32> {
        self.driver_client.ctrl_read_u32(
            self.driver_client.cfg.ctrl_baseaddr,
            INGO_MSM_ADDR::ADDR_HIF2CPU_C_MSM_TASK_LABEL,
        )
    }

    pub fn nof_elements(&self) -> Result<u32> {
        self.driver_client.ctrl_read_u32(
            self.driver_client.cfg.ctrl_baseaddr,
            INGO_MSM_ADDR::ADDR_CPU2HIF_C_NUMBER_OF_MSM_ELEMENTS,
        )
    }

    pub fn is_msm_engine_ready(&self) -> Result<u32> {
        self.driver_client.ctrl_read_u32(
            self.driver_client.cfg.ctrl_baseaddr,
            INGO_MSM_ADDR::ADDR_HIF2CPU_C_MSM_ENGINE_READY,
        )
    }

    pub fn load_data_to_hbm(&self, points: &[u8], addr: u64, offset: u64) -> Result<()> {
        log::debug!("HBM adress: {:#X?}", &addr);
        self.driver_client.ctrl_write_u32(
            self.driver_client.cfg.ctrl_baseaddr,
            INGO_MSM_ADDR::ADDR_CPU2HIF_C_BASES_SOURCE,
            1,
        )?;
        self.driver_client.ctrl_write(
            self.driver_client.cfg.ctrl_baseaddr,
            INGO_MSM_ADDR::ADDR_CPU2HIF_C_BASES_HBM_START_ADDRESS_LO,
            &addr.to_le_bytes(),
        )?;
        self.driver_client.dma_write(addr, offset, points)?;
        Ok(())
    }

    pub fn get_data_from_hbm(&self, data: &[u8], addr: u64, offset: u64) -> Result<Vec<u8>> {
        log::debug!("HBM adress: {:#X?}", addr);
        log::debug!("Data length: {:#X?}", data.len());
        let res = self.driver_client.dma_read(addr, offset, data.len());
        log::debug!("Successfully read data from hbm");
        res
    }

    pub fn get_api(&self) {
        for api_val in INGO_MSM_ADDR::iter() {
            self.driver_client
                .ctrl_read_u32(self.driver_client.cfg.ctrl_baseaddr, api_val)
                .unwrap();
        }
    }
}

#[derive(PackedStruct, Debug)]
#[packed_struct(bit_numbering = "msb0")]
pub struct MSMImageParametrs {
    #[packed_field(bits = "28..=31", endian = "lsb")]
    pub hif2cpu_c_is_stub: u8,
    #[packed_field(bits = "20..=27", endian = "lsb")]
    pub hif2_cpu_c_curve: u8,
    #[packed_field(bits = "16..=19", endian = "lsb")]
    pub hif2_cpu_c_number_of_ec_adders: u8,
    #[packed_field(bits = "8..=15", endian = "lsb")]
    pub hif2_cpu_c_buckets_mem_addr_width: u8,
    #[packed_field(bits = "4..=7", endian = "lsb")]
    pub hif2_cpu_c_number_of_segments: u8,
    #[packed_field(bits = "0..=3", endian = "lsb")]
    pub hif2_cpu_c_place_holder: u8,
}

impl ParametersAPI for MSMImageParametrs {
    fn parse_image_params(params: u32) -> MSMImageParametrs {
        let buf = params.reverse_bits().to_be_bytes();
        MSMImageParametrs::unpack(&buf).unwrap()
    }

    fn debug_information(&self) {
        log::debug!("Is Stub: {:?}", self.hif2cpu_c_is_stub);
        log::debug!("Is curve complex {:?}", self.hif2_cpu_c_curve & 1 == 1);
        match self.hif2_cpu_c_curve & 0b1111100 {
            0 => log::debug!("This is BLS12_377 curve"),
            1 => log::debug!("This is BN254 curve"),
            2 => log::debug!("This is BLS12_381 curve"),
            _ => log::debug!("This is UNKNOWN curve"),
        }
        log::debug!(
            "Number of EC addreses: {:?}",
            self.hif2_cpu_c_number_of_ec_adders
        );
        log::debug!(
            "Width of buckets memory adrreses: {:?}",
            self.hif2_cpu_c_buckets_mem_addr_width
        );
        log::debug!(
            "Number of segmemts: {:?}",
            self.hif2_cpu_c_number_of_segments
        );
        log::debug!("Place Holder: {:?}", self.hif2_cpu_c_place_holder);
    }
}
