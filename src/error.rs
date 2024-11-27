use std::io;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, DriverClientError>;

#[derive(Error, Debug)]
pub enum DriverClientError {
    #[error("failed to write data in offset {:?}", offset)]
    WriteError {
        offset: String,
        #[source]
        source: io::Error,
    },
    #[error("failed to read data from offset {:?}", offset)]
    ReadError {
        offset: String,
        #[source]
        source: io::Error,
    },
    #[error("hbicap doesn't ready to work")]
    HBICAPNotReady,
    #[error("failed to get driver primitive param")]
    InvalidPrimitiveParam,
    #[error("failed to parse csv")]
    CsvError(#[from] csv::Error),
    #[error("failed to load instruction set from: {:?}", path)]
    LoadFailed { path: String },
    #[error("failed open file")]
    FileError(#[from] io::Error),
    #[error("NOT MSM Binary")]
    NotMsmBin,
    #[error("unknown driver client error")]
    Unknown,
}
