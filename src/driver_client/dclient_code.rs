//! This moduel provide offsets and addreses space interface for basic shell and user logic cores.
#![allow(non_camel_case_types)]

/// High Bandwidth Internal Configuration Access Port (HBICAP) for runtime reconfiguration
/// (i.e. loading the second-stage/user configuration).
#[derive(Debug, Copy, Clone)]
pub enum HBICAP_ADDR {
    ADDR_HIF2CPU_HBICAP_STATUS = 0x110,
    ADDR_HIF2CPU_HBICAP_RESET = 0x10C,
    ADDR_CPU2HIF_HBICAP_TRANSFER_SIZE = 0x108,
    ADDR_HIF2CPU_HBICAP_ABORT_STATUS = 0x118,
}

impl From<HBICAP_ADDR> for u64 {
    fn from(addr: HBICAP_ADDR) -> Self {
        addr as u64
    }
}

/// Offsets for AXI Firewalls on both the AXI-Lite bus, and on the AXI bus.
#[derive(Debug, Copy, Clone)]
pub enum FIREWALL_ADDR {
    //HIF2CPU
    STATUS = 0x0,
    SI_STATUS = 0x100,
    // CPU2HIF
    BLOCK = 0x4,
    UNBLOCK = 0x8,
    DISABLE_BLOCK = 0x204,
}

impl From<FIREWALL_ADDR> for u64 {
    fn from(addr: FIREWALL_ADDR) -> Self {
        addr as u64
    }
}

/// Offsets for Card Management Subsystem (CMS), for system metrics such as temperature,
/// voltage and current, and also controls automatic thermal shutdown.
#[derive(Debug, Copy, Clone)]
pub enum CMS_ADDR {
    ADDR_CPU2HIF_CMS_INITIALIZE = 0x020000,
    ADDR_HIF2CPU_CMS_CONTROL_REG = 0x0018,
}
impl From<CMS_ADDR> for u64 {
    fn from(addr: CMS_ADDR) -> Self {
        addr as u64
    }
}

/// DFX Decouplers, to isolate the user logic during reconfiguration, protecting the shell from spurious signals.
///
/// The DFX Decoupler core register space include only control/status offset.
#[derive(Debug, Copy, Clone)]
pub enum DFX_DECOUPLER {
    DECOUPLE = 0x0,
}
impl From<DFX_DECOUPLER> for u64 {
    fn from(addr: DFX_DECOUPLER) -> Self {
        addr as u64
    }
}

/// Stub offset for DMA bus.
#[derive(Debug, Copy, Clone)]
pub enum DMA_RW {
    OFFSET = 0x0000_0000_0000_0000,
}

impl From<DMA_RW> for u64 {
    fn from(addr: DMA_RW) -> Self {
        addr as u64
    }
}
