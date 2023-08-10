//! This module provides the basic logic for working with an FPGA shell.
//! It includes functions for reading and writing using both the AXI Lite and AXI interfaces,
//! as well as for loading user logic.
//!
//! The trait and struct provided in this module are general in nature and serve as a foundation
//! for custom modules. Each custom module is built on top of this foundation and
//! includes its own specific fields and methods.
//!
use super::{dclient_cfg::*, dclient_code::*};
use crate::{
    error::*,
    utils::{open_channel, AccessFlags},
};
use std::{fmt::Debug, os::unix::fs::FileExt, thread::sleep, time::Duration};

/// A trait for defining functions related to parameters of specific core image.
pub trait ParametersAPI {
    /// Parses image parameters and returns a value of the implementing type.
    fn parse_image_params(params: u32) -> Self;
    /// Print all parameters details in user friendly mode.
    fn debug_information(&self);
}

/// The [`DriverPrimitive`] trait defines a interface that must be implemented by any driver primitive type.
/// The generic type parameters `T`, `P`, `I`, and `O` represent
///  the primitive type, parameter for initialization, input data, and output data respectively.
/// All methods return a `Result` indicating success or failure.
pub trait DriverPrimitive<T, P, I, O> {
    /// The `new` method creates a new instance of the driver primitive type with the given primitive type and driver client.
    fn new(ptype: T, dclient: DriverClient) -> Self;
    /// The `loaded_binary_parameters` method returns
    ///  a vector of 32-bit unsigned integers representing the loaded binary parameters.
    fn loaded_binary_parameters(&self) -> Vec<u32>;

    /// The `initialize` method initializes the driver primitive with the given parameter.
    fn initialize(&self, param: P) -> Result<()>;
    /// The `set_data` method sets the input data for the driver primitive.
    fn set_data(&self, input: I) -> Result<()>;
    /// The `start_process` method starts the driver after setting all controls and data.
    fn start_process(&self, param: Option<usize>) -> Result<()>;
    /// The `wait_result` method waits for the driver primitive to finish processing the input data.
    fn wait_result(&self) -> Result<()>;
    /// The `result` method returns the output data from the driver primitive,
    ///  optionally with a specific parameter. If there is no output data available, it returns `None`.
    fn result(&self, param: Option<usize>) -> Result<Option<O>>;
}

/// The [`DriverClient`] is described bunch of addreses on FPGA which called [`DriverConfig`] also
/// it includes file descriptor for read-from and write-to channels using DMA bus and CTRL bus.
pub struct DriverClient {
    /// Addreses space of current FPGA.
    pub(crate) cfg: DriverConfig,
    /// Write only channel from host memory into custom core using DMA bus.
    pub dma_h2c_write: std::fs::File,
    /// Read only channel from core using DMA bus.
    pub dma_c2h_read: std::fs::File,
    /// Read and write file descriptor for working with a register space that uses AXI-lite protocol.
    pub ctrl: std::fs::File,
}

impl DriverClient {
    /// The function creates a new instance of [`DriverClient`].
    ///
    /// # Arguments
    ///
    /// * `id` - argument is a string reference and represents the number of the FPGA slot.
    /// * `cfg` - argument is of the type [`DriverConfig`] and is used to define
    /// DMA and CTRL addreses space.
    ///
    /// # Example
    ///
    /// The example shows how to initialize a new instance of basic Driver API for FPGA slot `0`
    /// and the current addresses.
    /// ```rust
    /// use ingo_blaze::shell::shell_api::{DriverClient, DriverConfig};
    ///
    /// let dclient = DriverClient::new("0", DriverConfig::driver_client_cfg(CardType::U250));
    /// ```
    pub fn new(id: &str, cfg: DriverConfig) -> Self {
        DriverClient {
            cfg,
            dma_h2c_write: open_channel(&format!("/dev/xdma{}_h2c_0", id), AccessFlags::WrMode),
            dma_c2h_read: open_channel(&format!("/dev/xdma{}_c2h_0", id), AccessFlags::RdMode),
            ctrl: open_channel(&format!("/dev/xdma{}_user", id), AccessFlags::RdwrMode),
        }
    }
    /// The `reset` method resets the driver primitive to its initial state.
    pub fn reset(&self) -> Result<()> {
        self.set_dfx_decoupling(1)?;
        self.set_dfx_decoupling(0)?;
        sleep(Duration::from_millis(100));
        Ok(())
    }

    // ==== DFX ====
    /// Method for checking decouple status.
    pub fn get_dfx_decoupling(&self) -> Result<u32> {
        self.ctrl_read_u32(
            self.cfg.ctrl_dfx_decoupler_baseaddr,
            DFX_DECOUPLER::DECOUPLE,
        )
    }

    /// Setup decouple signal to isolate the user logic during reconfiguration, protecting the shell from spurious signals.
    pub fn set_dfx_decoupling(&self, signal: u32) -> Result<()> {
        self.ctrl_write_u32(
            self.cfg.ctrl_dfx_decoupler_baseaddr,
            DFX_DECOUPLER::DECOUPLE,
            signal,
        )?;
        Ok(())
    }

    // CMS
    pub fn initialize_cms(&self) -> Result<()> {
        self.ctrl_write_u32(
            self.cfg.ctrl_cms_baseaddr,
            CMS_ADDR::ADDR_CPU2HIF_CMS_INITIALIZE,
            1,
        )?;
        sleep(Duration::from_millis(200));
        Ok(())
    }

    /// This method setup 27 bit in CONTROL_REG for enabling hbm temperature monitoring.
    pub fn enable_hbm_temp_monitoring(&self) -> Result<()> {
        let ctrl_reg = self.ctrl_read_u32(
            self.cfg.ctrl_cms_baseaddr + CMS_ADDR::ADDR_HIF2CPU_CMS_REG_MAP as u64,
            CMS_ADDR::ADDR_HIF2CPU_CMS_CONTROL_REG,
        );
        self.ctrl_write_u32(
            self.cfg.ctrl_cms_baseaddr + CMS_ADDR::ADDR_HIF2CPU_CMS_REG_MAP as u64,
            CMS_ADDR::ADDR_HIF2CPU_CMS_CONTROL_REG,
            ctrl_reg.unwrap() | 1 << 27,
        )?;
        Ok(())
    }

    pub fn reset_sensor_data(&self) -> Result<()> {
        let ctrl_reg = self.ctrl_read_u32(
            self.cfg.ctrl_cms_baseaddr + CMS_ADDR::ADDR_HIF2CPU_CMS_REG_MAP as u64,
            CMS_ADDR::ADDR_HIF2CPU_CMS_CONTROL_REG,
        );
        self.ctrl_write_u32(
            self.cfg.ctrl_cms_baseaddr + CMS_ADDR::ADDR_HIF2CPU_CMS_REG_MAP as u64,
            CMS_ADDR::ADDR_HIF2CPU_CMS_CONTROL_REG,
            ctrl_reg.unwrap() | 1,
        )?;
        Ok(())
    }

    // HBICAP
    /// Checking HBICAP status register. Return `true` if zero (previous operation done) and
    /// second (Indicates that the EOS is complete) bit setting to 1.
    pub fn is_hbicap_ready(&self) -> bool {
        let status = self
            .ctrl_read_u32(
                self.cfg.ctrl_hbicap_baseaddr,
                HBICAP_ADDR::ADDR_HIF2CPU_HBICAP_STATUS,
            )
            .unwrap();
        status == 5
    }

    pub fn hbicap_reset(&self) -> Result<()> {
        self.ctrl_write_u32(
            self.cfg.ctrl_hbicap_baseaddr,
            HBICAP_ADDR::ADDR_HIF2CPU_HBICAP_RESET,
            0xC,
        )?;
        Ok(())
    }

    /// This method prepare FPGA before load binary to custom core.
    pub fn setup_before_load_binary(&self) -> Result<()> {
        self.initialize_cms()?;
        self.enable_hbm_temp_monitoring()?;

        self.ctrl_read_u32(
            self.cfg.ctrl_hbicap_baseaddr,
            HBICAP_ADDR::ADDR_HIF2CPU_HBICAP_ABORT_STATUS,
        )?;

        self.set_firewall_block(self.cfg.ctrl_firewall_baseaddr, true)?;
        self.set_firewall_block(self.cfg.dma_firewall_baseaddr, true)?;

        Ok(())
    }

    /// This method load binary configuration to custom core.
    /// Function returns status of HBICAP after loading binary
    ///
    /// # Arguments
    ///
    /// * `binary`: a byte slice containing the data to be written.
    ///
    /// returns: u32
    ///
    /// # Examples
    ///
    /// ```rust
    /// use ingo_blaze::shell::{shell_api::DriverClient, shell_hw_code::*};
    ///
    /// let dclient = DriverClient::new("0", DriverConfig::driver_client_cfg(CardType::U250));
    ///
    /// // read binary data from given filename
    /// let buf = utils::read_binary_file(&filename);
    ///
    /// dclient.setup_before_load_binary()?;
    /// let ret = dclient.load_binary(buf.as_slice())?;
    /// assert_eq!(ret, 0x0);
    /// dclient.unblock_firewalls()?;
    /// ```
    pub fn load_binary(&self, binary: &[u8]) -> Result<u32> {
        if !self.is_hbicap_ready() {
            return Err(DriverClientError::HBICAPNotReady);
        }
        self.set_dfx_decoupling(1)?;
        self.hbicap_reset()?;

        self.ctrl_write_u32(
            self.cfg.ctrl_hbicap_baseaddr,
            HBICAP_ADDR::ADDR_CPU2HIF_HBICAP_TRANSFER_SIZE,
            (binary.len() / 4) as u32,
        )?;
        self.dma_write(self.cfg.dma_hbicap_baseaddr, DMA_RW::OFFSET, binary)?;
        while !self.is_hbicap_ready() {
            sleep(Duration::from_millis(10));
        }
        self.set_dfx_decoupling(0)?;
        self.set_firewall_block(self.cfg.ctrl_firewall_baseaddr, false)?;
        self.set_firewall_block(self.cfg.dma_firewall_baseaddr, false)?;

        self.ctrl_read_u32(
            self.cfg.ctrl_hbicap_baseaddr,
            HBICAP_ADDR::ADDR_HIF2CPU_HBICAP_ABORT_STATUS,
        )
    }

    // ==== Firewall (DMA and CTRL) ====
    /// The function allows you to block or unblock the selected axi firewall.
    ///
    /// User is recommended to check the firewall status after a large transfer to ensure all data has been written correctly.
    /// No further transactions can be made until the firewall is unblocked by writting to the unblock register.
    ///
    /// # Arguments
    ///
    /// * `addr` - The firewall address for which the blocking/unblocking signal is set.
    /// * `block` - A boolean value indicating whether to block (`true`) or unblock (`false`) the firewall.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use ingo_blaze::shell::shell_api::DriverClient;
    ///
    /// let dclient = DriverConfig::driver_client_cfg(CardType::U250);
    /// dclient.set_firewall_block(dclient.cfg.ctrl_firewall_baseaddr, true); // ctrl firewall is now blocked
    /// dclient.set_firewall_block(dclient.cfg.dma_firewall_baseaddr, false); // dma firewall is now unblocked
    /// ```
    pub fn set_firewall_block(&self, addr: u64, block: bool) -> Result<()> {
        if block {
            self.ctrl_write_u32(addr, FIREWALL_ADDR::BLOCK, 0x100_0100)?;
            Ok(())
        } else {
            self.ctrl_write_u32(addr, FIREWALL_ADDR::BLOCK, 0)?;
            self.ctrl_write_u32(addr, FIREWALL_ADDR::UNBLOCK, 1)?;
            Ok(())
        }
    }

    pub fn unblock_firewalls(&self) -> Result<()> {
        self.set_firewall_block(self.cfg.ctrl_firewall_baseaddr, false)?;
        self.set_firewall_block(self.cfg.dma_firewall_baseaddr, false)?;
        self.ctrl_write_u32(
            self.cfg.ctrl_firewall_baseaddr,
            FIREWALL_ADDR::DISABLE_BLOCK,
            0,
        )?;
        Ok(())
    }

    // ==== XDMA CTRL BUS ====
    /// Read a 32-bit value using the axil interface at a given adress and offset.
    /// This is generally used to control and monitor the operations into FPGA.
    ///
    /// # Arguments
    ///
    /// * `base_address`: the base address in the CTRL bus addresses space
    /// * `offset`: an enum which represent the specific offset for given `base_address`.
    ///
    /// returns: u32
    ///
    /// # Examples
    /// ```rust
    /// use ingo_blaze::shell::{shell_api::DriverClient, shell_hw_code::*};
    ///
    /// let dclient = DriverClient::new("0", DriverConfig::driver_client_cfg(CardType::U250));
    ///
    /// let ret = dclient.ctrl_read_u32(
    ///     dclient.cfg.ctrl_firewall_baseaddr,
    ///     FIREWALL_ADDR::STATUS,
    /// );
    /// ```
    pub fn ctrl_read_u32<T: Debug + Into<u64> + Copy>(
        &self,
        base_address: u64,
        offset: T,
    ) -> Result<u32> {
        let mut task_label = [0, 0, 0, 0];
        self.ctrl
            .read_exact_at(&mut task_label, base_address + offset.into())
            .map_err(|e| DriverClientError::ReadError {
                offset: format!("{:?}", offset),
                source: e,
            })?;
        let res = u32::from_le_bytes(task_label);
        log::debug!("Getting data [ {:#X?} ] from label {:?}", res, offset);
        Ok(res)
    }

    /// Read a 64-bit value using the axil interface at a given adress and offset.
    /// This is generally used to control and monitor the operations into FPGA.
    ///
    /// # Arguments
    ///
    /// * `base_address`: the base address in the CTRL bus addresses space
    /// * `offset`: an enum which represent the specific offset for given `base_address`.
    ///
    /// returns: u64
    pub fn ctrl_read_u64<T: Debug + Into<u64> + Copy>(
        &self,
        base_address: u64,
        offset: T,
    ) -> Result<u64> {
        let mut task_label_0 = [0, 0, 0, 0];
        self.ctrl
            .read_exact_at(&mut task_label_0, base_address + offset.into())
            .map_err(|e| DriverClientError::ReadError {
                offset: format!("{:?}", offset),
                source: e,
            })?;

        let mut task_label_1 = [0, 0, 0, 0];
        self.ctrl
            .read_exact_at(&mut task_label_1, base_address + offset.into() + 4)
            .map_err(|e| DriverClientError::ReadError {
                offset: format!("{:?}", offset),
                source: e,
            })?;

        let conc: [u8; 8] = [task_label_0, task_label_1].concat().try_into().unwrap();
        let res = u64::from_le_bytes(conc);
        log::debug!("Getting data [ {:#X?} ] from label {:?}", res, offset);
        Ok(res)
    }

    /// Write a 32-bit value using the axil interface at a given adress and offset.
    /// This is generally used to control and monitor the operations into FPGA.
    ///
    /// # Arguments
    ///
    /// * `base_address`: the base address in the CTRL bus addresses space
    /// * `offset`: an enum which represent the specific offset for given `base_address`.
    /// * `data`: a 32-bit value for writing.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use ingo_blaze::shell::{shell_api::DriverClient, shell_hw_code::*};
    ///
    /// let dclient = DriverClient::new("0", DriverConfig::driver_client_cfg(CardType::U250));
    ///    
    /// dclient.ctrl_write_u32(
    ///     dclient.cfg.ctrl_hbicap_baseaddr,
    ///     HBICAP_ADDR::ADDR_CPU2HIF_HBICAP_TRANSFER_SIZE,
    ///     32,
    /// );
    /// ```
    pub fn ctrl_write_u32<T: Debug + Into<u64> + Copy>(
        &self,
        base_address: u64,
        offset: T,
        data: u32,
    ) -> Result<()> {
        let bytes_array = data.to_le_bytes();
        self.ctrl
            .write_all_at(&bytes_array, base_address + offset.into())
            .map_err(|e| DriverClientError::WriteError {
                offset: format!("{:?}", offset),
                source: e,
            })?;

        log::debug!("Successfully set data [ {:?} ] to label {:?}", data, offset);
        Ok(())
    }

    /// The method for writing data using the axil interface at a given adress and offset.
    /// Axil only works with u32 and therefore larger data will be written in 4 byte chunks.
    ///
    /// This is generally used to control and monitor the operations into FPGA.
    /// Mostly it using with custom logic to setup inner parameters.
    ///
    /// # Arguments
    ///
    /// * `base_address`: the base address in the CTRL bus addresses space
    /// * `offset`: an enum which represent the specific offset for given `base_address`.
    /// * `data`: a byte slice containing the data to be written.
    ///
    pub fn ctrl_write<T: Debug + Into<u64> + Copy>(
        &self,
        base_address: u64,
        offset: T,
        data: &[u8],
    ) -> Result<()> {
        data.chunks(4).enumerate().try_for_each(|(i, s_chunk)| {
            self.ctrl
                .write_all_at(s_chunk, base_address + offset.into() + (i * 4) as u64)
                .map_err(|e| DriverClientError::WriteError {
                    offset: format!("{:?}", offset),
                    source: e,
                })
        })?;

        crate::setter_log!(data, offset);
        Ok(())
    }

    // ==== XDMA DMA BUS ====
    /// The method for reading data from FPGA by DMA bus
    /// It returns a `Vec<u8>` containing the read data, allowing the user to further process the data.
    ///
    /// The location is determined by adding the `offset` to the `base_address`.
    /// If you don't need an offset for reading, use the default offset [`DMA_RW::OFFSET`].
    ///
    /// # Arguments
    ///
    /// * `base_address`: the base address in the DMA bus addresses space
    /// * `offset`: an enum which represent the specific offset for given `base_address`.
    /// * `read_buffer`: existing mememory for reading data
    ///
    /// returns: Vec<u8>
    ///
    /// # Examples
    ///
    /// ```rust
    /// use ingo_blaze::shell::{shell_api::DriverClient, shell_hw_code::*};
    ///
    /// let dclient = DriverClient::new("0", DriverConfig::driver_client_cfg(CardType::U250));
    /// let size_of_input = 16;
    /// let readen = dclient.dma_read(
    ///     dclient.cfg.dma_baseaddr,
    ///     DMA_RW::OFFSET,
    ///     size_of_input,
    /// );
    /// assert_eq!(readen.len(), size_of_input);
    /// ```
    pub fn dma_read<T: Debug + Into<u64> + Copy>(
        &self,
        base_address: u64,
        offset: T,
        read_buffer: &mut Vec<u8>,
    ) -> Result<()> {
        self.dma_c2h_read
            .read_exact_at(read_buffer, base_address + offset.into())
            .map_err(|e| DriverClientError::ReadError {
                offset: format!("{:?}", offset),
                source: e,
            })?;

        crate::getter_log!(read_buffer, offset);
        Ok(())
    }

    /// The method for writing data from host memory into FPGA.
    ///
    /// The location is determined by adding the `offset` to the `base_address`.
    /// If you don't need an offset for reading, use the default offset for this:
    /// [`DMA_RW::OFFSET`]
    ///
    /// # Arguments
    ///
    /// * `base_address`: the base address in the DMA bus addresses space
    /// * `offset`: an enum which represent the specific offset for given `base_address`.
    /// * `data: &[u8]`: a byte slice containing the data to be written.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use ingo_blaze::shell::{shell_api::DriverClient, shell_hw_code::*};
    ///
    /// let dclient = DriverClient::new("0", DriverConfig::driver_client_cfg(CardType::U250));
    /// let input = vec![1, 2, 3, 4, 5, 6, 7, 8];
    /// let chunk_size = 4;
    ///
    /// dclient.dma_write(
    ///     dclient.cfg.dma_baseaddr,
    ///     DMA_RW::OFFSET,
    ///     input.as_slice(),
    /// );
    /// ```
    pub fn dma_write<T: Debug + Into<u64> + Copy>(
        &self,
        base_address: u64,
        offset: T,
        data: &[u8],
    ) -> Result<()> {
        log::trace!("Trying to write data of size {}", data.len());
        self.dma_h2c_write
            .write_all_at(data, base_address + offset.into())
            .map_err(|e| DriverClientError::WriteError {
                offset: format!("{:?}", offset),
                source: e,
            })?;
        log::trace!("Write data of size {}", data.len());

        crate::setter_log!(data, offset);
        Ok(())
    }

    /// This method writes data by chunks to a specific location in the DMA.
    /// The location is determined by adding the `offset` to the `base_address`.
    /// If you don't need an offset to write to the dma,
    /// use the default offset for this: [`DMA_RW::OFFSET`]
    ///
    /// # Arguments
    ///
    /// * `base_address`: the base address in the DMA bus addresses space
    /// * `offset`: an enum which represent the specific offset for given `base_address`.
    /// * `data`: a byte slice containing the data to be written.
    /// * `chunk_size`: representing the size of the chunks that data should be divided into before writing.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use ingo_blaze::shell::{shell_api::DriverClient, shell_hw_code::*};
    ///
    /// let dclient = DriverClient::new("0", DriverConfig::driver_client_cfg(CardType::U250));
    /// let input = vec![1, 2, 3, 4, 5, 6, 7, 8];
    /// let chunk_size = 4;
    ///
    /// dclient.dma_write_by_chunks(
    ///     dclient.cfg.dma_baseaddr,
    ///     DMA_RW::OFFSET,
    ///     input.as_slice(),
    ///     chunk_size,
    /// );
    /// ```
    pub fn dma_write_by_chunks<T: Debug + Into<u64> + Copy>(
        &self,
        base_address: u64,
        offset: T,
        data: &[u8],
        chunk_size: usize,
    ) -> Result<()> {
        data.chunks(chunk_size).try_for_each(|s_chunk| {
            self.dma_h2c_write
                .write_all_at(s_chunk, base_address + offset.into())
                .map_err(|e| DriverClientError::WriteError {
                    offset: format!("{:?}", offset),
                    source: e,
                })
        })?;
        crate::setter_log!(data, offset);
        Ok(())
    }

    pub fn firewalls_status(&self) {
        let mut ret = self.ctrl_read_u32(
            self.cfg.ctrl_hbicap_baseaddr,
            HBICAP_ADDR::ADDR_HIF2CPU_HBICAP_ABORT_STATUS,
        );
        log::info!("ICAP Abort Status: {:#X?}", ret.unwrap());

        ret = self.ctrl_read_u32(self.cfg.ctrl_firewall_baseaddr, FIREWALL_ADDR::STATUS);
        log::info!("AXIL Firewall Status: {:#X?}", ret.unwrap());

        let ret_dma = self.ctrl_read_u32(self.cfg.dma_firewall_baseaddr, FIREWALL_ADDR::STATUS);

        log::info!("DMA Firewall Status: {:#X?}", ret_dma.unwrap());
    }

    // ==== utils ====>
    pub fn is_ctrl_field_expected_value<T: Debug + Into<u64> + Copy>(
        &self,
        baseaddr: u64,
        offset: T,
        value: u32,
    ) -> bool {
        let res = self.ctrl_read_u32(baseaddr, offset).unwrap();

        res == value
    }
}
