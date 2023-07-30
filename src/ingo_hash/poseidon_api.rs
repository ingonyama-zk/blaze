use packed_struct::prelude::PackedStruct;
use std::{option::Option, thread::sleep, time::{Duration, Instant}};
use strum::IntoEnumIterator;

use crate::{
    driver_client::dclient::*, driver_client::dclient_code::*, error::*,
    ingo_hash::{hash_hw_code::*, dma_buffer::Align4K}, ingo_hash::utils::*, utils::convert_to_32_byte_array,
};

use csv;
use num::{bigint::BigUint, Num};

use super::dma_buffer::DmaBuffer;

pub enum Hash {
    Poseidon,
}

pub struct PoseidonClient {
    pub dclient: DriverClient,
}

#[derive(Clone)]
pub struct PoseidonInitializeParameters {
    pub tree_height: u32,
    pub tree_mode: TreeMode,
    pub instruction_path: String,
}

pub struct PoseidonResult {
    pub hash_byte: [u8; 32],
    pub hash_id: u32,
    pub layer_id: u32,
}

pub struct PoseidonReadResult<'a> {
    pub expected_result: usize,
    pub result_store_buffer: &'a mut DmaBuffer,
}

impl PoseidonResult {
    /// The function parses poseidon hash result
    ///
    /// # Arguments
    ///
    /// * `data` - an Vec<u8> of size 64 bytes representing a hash (32 bytes) and hash_id (32 bytes).
    ///
    /// # Example
    ///
    /// The hash_id contains layer index and and layer height.
    pub fn parse_poseidon_hash_results(data: Vec<u8>) -> Vec<PoseidonResult> {
        let split_data: Vec<&[u8]> = data.chunks(64).collect();

        let mut results: Vec<PoseidonResult> = Vec::new();

        for element in split_data {
            assert_eq!(element.len(), 64);

            let hash: &[u8; 32] = element[0..32].try_into().unwrap();
            let hash_data: &[u8; 32] = element[32..].try_into().unwrap();

            let hash_first_4_bytes: [u8; 4] = hash_data[..4].try_into().unwrap();

            // optimize this its bad
            let hash_2_bytes: [u8; 2] = hash_data[3..5].try_into().unwrap();
            let mut hash_last_2_bytes: [u8; 4] = [0; 4];
            hash_last_2_bytes[..hash_2_bytes.len()].copy_from_slice(&hash_2_bytes);

            let hash_id = u32::from_le_bytes(hash_first_4_bytes) & 0x3fffffff;
            let layer_id = u32::from_le_bytes(hash_last_2_bytes) >> 6;

            results.push(PoseidonResult {
                hash_byte: hash.to_owned(),
                hash_id,
                layer_id,
            });
        }

        results
    }
}

impl
    DriverPrimitive<
        Hash,
        PoseidonInitializeParameters,
        &[u8],
        Vec<PoseidonResult>,
        PoseidonReadResult<'_>,
    > for PoseidonClient
{
    fn new(_ptype: Hash, dclient: DriverClient) -> Self {
        PoseidonClient { dclient }
    }

    fn loaded_binary_parameters(&self) -> Vec<u32> {
        [
            INGO_POSEIDON_ADDR::ADDR_HIF2CPU_C_IMAGE_ID,
            INGO_POSEIDON_ADDR::ADDR_HIF2CPU_C_IMAGE_PARAMTERS,
        ]
        .map(|offset| {
            self.dclient
                .ctrl_read_u32(self.dclient.cfg.ctrl_baseaddr, offset)
                .map_err(|_| DriverClientError::InvalidPrimitiveParam)
                .unwrap()
        })
        .into_iter()
        .collect::<Vec<u32>>()
    }

    fn reset(&self) -> Result<()> {
        self.dclient.set_dfx_decoupling(1)?;
        self.dclient.set_dfx_decoupling(0)?;
        sleep(Duration::from_millis(100));
        Ok(())
    }

    fn initialize(&self, param: PoseidonInitializeParameters) -> Result<()> {
        //self.reset()?;
        
        self.dclient.initialize_cms()?;
        self.dclient.set_dma_firewall_prescale(0x4FFF)?;

        self.set_initialize_mode(true)?;

        self.load_instructions(&param.instruction_path)
            .map_err(|_| DriverClientError::LoadFailed {
                path: param.instruction_path,
            })?;
        self.set_initialize_mode(false)?;
        log::debug!("successfully loaded instruction set");

        self.set_merkle_tree_height(param.tree_height)?;
        self.set_tree_start_layer_for_tree(param.tree_mode)?;
        log::debug!("set merkle tree height: {:?}", param.tree_height);

        Ok(())
    }

    fn set_data(&self, input: &[u8]) -> Result<()> {
        self.dclient
            .dma_write(self.dclient.cfg.dma_baseaddr, DMA_RW::OFFSET, input)?;

        Ok(())
    }

    fn wait_result(&self) -> Result<()> {
        todo!()
    }

    fn result(&self, _param: Option<PoseidonReadResult>) -> Result<Option<Vec<PoseidonResult>>> {
        let params = _param.unwrap();

        let start_read = Instant::now();
        self.dclient.dma_read_into(
            self.dclient.cfg.dma_baseaddr,
            DMA_RW::OFFSET,
            params.result_store_buffer,
        );
        let duration_read = start_read.elapsed();
        log::info!("took {:?} to read dma", duration_read);


        assert_eq!(
            params.result_store_buffer.len(),
            params.expected_result * 64
        );

        let start_parse = Instant::now();
        let results =
            PoseidonResult::parse_poseidon_hash_results(params.result_store_buffer.get().to_vec());
        let duration_parse = start_parse.elapsed();
        log::info!("took {:?} to parse dma data", duration_parse);

        Ok(Some(results))
    }
}

impl PoseidonClient {
    pub fn get_last_element_sent_to_ring(&self) -> Result<u32> {
        self.dclient.ctrl_read_u32(
            self.dclient.cfg.ctrl_baseaddr,
            INGO_POSEIDON_ADDR::ADDR_HIF2CPU_C_LAST_ELEMENT_ID_SENT_TO_RING,
        )
    }

    pub fn get_num_of_pending_results(&self) -> Result<u32> {
        self.dclient.ctrl_read_u32(
            self.dclient.cfg.ctrl_baseaddr,
            INGO_POSEIDON_ADDR::ADDR_HIF2CPU_C_NOF_RESULTS_PENDING_ON_DMA_FIFO,
        )
    }

    fn set_merkle_tree_height(&self, height: u32) -> Result<()> {
        self.dclient.ctrl_write_u32(
            self.dclient.cfg.ctrl_baseaddr,
            INGO_POSEIDON_ADDR::ADDR_CPU2HIF_C_MERKLE_TREE_HEIGHT,
            height,
        )
    }

    fn set_tree_start_layer_for_tree(&self, tree: TreeMode) -> Result<()> {
        let start_height: u32 = TreeMode::value(tree);

        self.dclient.ctrl_write_u32(
            self.dclient.cfg.ctrl_baseaddr,
            INGO_POSEIDON_ADDR::ADDR_CPU2HIF_C_MERKLE_TREE_START_LAYER,
            start_height,
        )
    }

    fn set_initialize_mode(&self, enter: bool) -> Result<()> {
        let value = if enter { 1 } else { 0 };

        self.dclient.ctrl_write_u32(
            self.dclient.cfg.ctrl_baseaddr,
            INGO_POSEIDON_ADDR::ADDR_CPU2HIF_C_INITIALIZATION_MODE,
            value,
        )
    }

    pub fn get_raw_results(&self, num_of_results: u32) -> Result<Vec<u8>> {
        self.dclient.dma_read(
            self.dclient.cfg.dma_baseaddr,
            DMA_RW::OFFSET,
            (64 * num_of_results).try_into().unwrap(),
        )
    }

    pub fn get_last_hash_sent_to_host(&self) -> Result<u32> {
        self.dclient.ctrl_read_u32(
            self.dclient.cfg.ctrl_baseaddr,
            INGO_POSEIDON_ADDR::ADDR_HIF2CPU_C_LAST_HASH_ID_SENT_TO_HOST,
        )
    }

    fn load_instructions(&self, file_path: &str) -> Result<()> {
        let mut reader = csv::Reader::from_path(file_path)?;

        for res in reader.records() {
            let result = res?;

            let element_one_str = result.get(result.len() - 1).unwrap();
            let element_two_str = result.get(result.len() - 2).unwrap();

            let mut first_value_to_send = DmaBuffer::new::<Align4K>(32);
            let mut second_value_to_send = DmaBuffer::new::<Align4K>(32);

            first_value_to_send.extend_from_slice(&convert_to_32_byte_array(
                BigUint::from_str_radix(element_one_str, 10)
                    .unwrap()
                    .to_bytes_le()
                    .as_slice(),
            ));

            second_value_to_send.extend_from_slice(&convert_to_32_byte_array(
                BigUint::from_str_radix(element_two_str, 10)
                    .unwrap()
                    .to_bytes_le()
                    .as_slice(),
            ));

            // some sanity tests,
            debug_assert!(
                BigUint::from_bytes_le(first_value_to_send.as_slice()).to_string() == element_one_str
            );
            debug_assert!(
                BigUint::from_bytes_le(second_value_to_send.as_slice()).to_string() == element_two_str
            );
            debug_assert_eq!(first_value_to_send.len(), 32);
            debug_assert_eq!(second_value_to_send.len(), 32);

            self.set_data(first_value_to_send.as_slice())?;
            self.set_data(second_value_to_send.as_slice())?;
        }

        Ok(())
    }

    pub fn log_api(&self) {
        log::info!("=== api values ===");
        for api_val in INGO_POSEIDON_ADDR::iter() {
            log::info!(
                "{:?} value: {:?}",
                api_val,
                self.dclient.ctrl_read_u32(self.dclient.cfg.ctrl_baseaddr, api_val).unwrap()
            );
        }
        log::info!("=== api values ===");
    }

    pub fn log_api_value(&self, api_addrs: &[INGO_POSEIDON_ADDR]) {
        log::info!("=== api values ===");
        for api_val in api_addrs {
            log::info!(
                "{:?} value: {:?}",
                api_val,
                self.dclient.ctrl_read_u32(self.dclient.cfg.ctrl_baseaddr, *api_val).unwrap()
            );
        }
        log::info!("=== api values ===");
    }
}

#[derive(PackedStruct, Debug)]
#[packed_struct(bit_numbering = "msb0")]
pub struct PoseidonImageParametrs {
    #[packed_field(bits = "28..=31", endian = "lsb")]
    pub hif2_cpu_c_is_stub: u8,
    #[packed_field(bits = "20..=27", endian = "lsb")]
    pub hif2_cpu_c_number_of_cores: u8,
    #[packed_field(bits = "0..=19", endian = "lsb")]
    pub hif2_cpu_c_place_holder: u32,
}

impl ParametersAPI for PoseidonImageParametrs {
    fn parse_image_params(params: u32) -> PoseidonImageParametrs {
        let buf = params.to_be_bytes();
        PoseidonImageParametrs::unpack(&buf).unwrap()
    }

    fn debug_information(&self) {
        log::debug!("Is Stub: {:?}", self.hif2_cpu_c_is_stub);
        log::debug!("Number of Cores: {:?}", self.hif2_cpu_c_number_of_cores);
        log::debug!("Place Holder: {:?}", self.hif2_cpu_c_place_holder);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use log::info;
    use std::env;

    #[test]
    fn load_hash_binary_test() {
        env_logger::try_init().expect("Invalid logger initialisation");
        let id = env::var("ID").unwrap_or_else(|_| 0.to_string());
        info!("ID: {}", id);

        info!("Create Driver API instance");

        let dclient = DriverClient::new(&id, DriverConfig::driver_client_c1100_cfg());
        let driver: PoseidonClient = PoseidonClient::new(Hash::Poseidon, dclient);
        let params = driver.loaded_binary_parameters();
        info!("Driver parameters: [{:?}, {:032b}]", params[0], params[1]);
        let params_parce = PoseidonImageParametrs::parse_image_params(params[1]);
        params_parce.debug_information();
    }
}
