use strum_macros::EnumString;

#[derive(Debug, EnumString, PartialEq)]
pub enum Curve {
    BLS377,
    BLS381,
    BN254,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, EnumString)]
pub enum PointMemoryType {
    HBM,
    DMA,
}

#[derive(Debug, Copy, Clone)]
pub(super) struct MSMConfig {
    // The size characteristic in points and scalars in a curve.
    /// The size in bytes of result point. The point is expected to be in projective form.
    pub result_point_size: usize,
    /// The size of one point in bytes. Point is represented in affine form.
    pub point_size: Option<usize>,
    /// The size of scalar coordinate in bytes.
    pub scalar_size: usize,

    // Ingo MSM Core additional addresses
    pub dma_scalars_addr: Option<u64>,
    pub dma_points_addr: Option<u64>,
}

impl MSMConfig {
    pub(super) fn msm_cfg(curve: Curve, mem: PointMemoryType) -> Self {
        match (curve, mem) {
            (Curve::BLS377, PointMemoryType::HBM) => msm_bls377_hbm_cfg(),
            (Curve::BLS377, PointMemoryType::DMA) => msm_bls377_dma_cfg(),
            (Curve::BLS381, PointMemoryType::HBM) => msm_bls381_hbm_cfg(),
            (Curve::BLS381, PointMemoryType::DMA) => msm_bls381_dma_cfg(),
            (Curve::BN254, PointMemoryType::HBM) => todo!(),
            (Curve::BN254, PointMemoryType::DMA) => msm_bn254_dma_cfg(),
        }
    }
}

fn msm_bls377_hbm_cfg() -> MSMConfig {
    MSMConfig {
        result_point_size: 144,
        point_size: Some(96),
        scalar_size: 32,
        dma_scalars_addr: Some(0x0000020000000000),
        dma_points_addr: None,
    }
}

fn msm_bls377_dma_cfg() -> MSMConfig {
    MSMConfig {
        result_point_size: 144,
        point_size: Some(96),
        scalar_size: 32,
        dma_scalars_addr: Some(0x0000_0200_0000_0000),
        dma_points_addr: Some(0x0000_0100_0000_0000),
        // dma_scalars_addr: Some(0x0000_0100_0000_0000),
        // dma_points_addr: Some(0x0000_0000_0000_0000),
    }
}

fn msm_bls381_hbm_cfg() -> MSMConfig {
    MSMConfig {
        result_point_size: 144,
        point_size: Some(96),
        scalar_size: 32,
        dma_scalars_addr: Some(0x0000020000000000),
        dma_points_addr: None,
    }
}

fn msm_bls381_dma_cfg() -> MSMConfig {
    MSMConfig {
        result_point_size: 144,
        point_size: Some(96),
        scalar_size: 32,
        dma_scalars_addr: Some(0x0000020000000000),
        dma_points_addr: Some(0x0000010000000000),
    }
}

fn msm_bn254_dma_cfg() -> MSMConfig {
    MSMConfig {
        result_point_size: 96,
        point_size: Some(64),
        scalar_size: 32,
        dma_scalars_addr: Some(0x0000010000000000),
        dma_points_addr: Some(0x0000000000000000),
    }
}
