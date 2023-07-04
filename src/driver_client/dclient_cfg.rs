pub enum CardType {
    C1100,
}

/// The [`DriverConfig`] is a struct that defines a set of 64-bit unsigned integer (`u64`)
/// representing addreses memory space for different components of a FPGA.
/// The struct is divided into logical parts: AXI Lite space of addresses and AXI space of addresses
#[derive(Copy, Clone, Debug)]
pub struct DriverConfig {
    // CTRL
    pub(crate) ctrl_baseaddr: u64,
    pub(crate) ctrl_cms_baseaddr: u64,
    // pub(crate) ctrl_qspi_baseaddr: u64,
    pub(crate) ctrl_hbicap_baseaddr: u64,
    // pub(crate) ctrl_mgmt_ram_baseaddr: u64,
    pub(crate) ctrl_firewall_baseaddr: u64,
    pub(crate) dma_firewall_baseaddr: u64,
    pub(crate) ctrl_dfx_decoupler_baseaddr: u64,

    // DMA
    pub(crate) dma_baseaddr: u64,
    // pub(crate) dma_hbicap_baseaddr: u64,
}

impl DriverConfig {
    /// Create a new driver config.
    pub fn driver_client_cfg(card_type: CardType) -> Self {
        match card_type {
            CardType::C1100 => c1100_cfg(),
        }
    }
}

fn c1100_cfg() -> DriverConfig {
    DriverConfig {
        ctrl_baseaddr: u64::from_str_radix("0x00000000".trim_start_matches("0x"), 16).unwrap(),
        ctrl_cms_baseaddr: u64::from_str_radix("0x04000000".trim_start_matches("0x"), 16).unwrap(),
        // ctrl_qspi_baseaddr: u64::from_str_radix("0x04040000".trim_start_matches("0x"), 16).unwrap(),
        ctrl_hbicap_baseaddr: u64::from_str_radix("0x04050000".trim_start_matches("0x"), 16)
            .unwrap(),
        // ctrl_mgmt_ram_baseaddr: u64::from_str_radix("0x04060000".trim_start_matches("0x"), 16).unwrap(),
        ctrl_firewall_baseaddr: u64::from_str_radix("0x04070000".trim_start_matches("0x"), 16)
            .unwrap(),
        dma_firewall_baseaddr: u64::from_str_radix("0x04080000".trim_start_matches("0x"), 16)
            .unwrap(),
        ctrl_dfx_decoupler_baseaddr: u64::from_str_radix("0x04090000".trim_start_matches("0x"), 16)
            .unwrap(),
        dma_baseaddr: u64::from_str_radix("0x0000000000000000".trim_start_matches("0x"), 16)
            .unwrap(),
        // dma_hbicap_baseaddr: u64::from_str_radix("0x1000000000000000".trim_start_matches("0x"), 16).unwrap(),
    }
}
