use anyhow;
use std::{
    fs::{File, OpenOptions},
    io::{Error, Read},
    os::unix::prelude::OpenOptionsExt,
    thread, time,
};

#[macro_export]
macro_rules! getter_log {
    ($data:expr, $from: expr) => {
        if $data.len() < 256 {
            log::debug!("Getting data [ {:#X?} ] from {:#X?}", $data, $from);
        } else {
            log::debug!(
                "Getting data of size [ {:?} ] from {:#X?}",
                $data.len(),
                $from
            );
        }
    };
}

#[macro_export]
macro_rules! setter_log {
    ($data:expr, $from: expr) => {
        if $data.len() < 256 {
            log::trace!("Successfully set data [ {:?} ] to {:#X?}", $data, $from);
        } else {
            log::trace!(
                "Successfully set data of size [ {:?} ] to {:#X?}",
                $data.len(),
                $from
            );
        }
    };
}

#[repr(u8)]
#[derive(PartialEq, Eq)]
pub enum AccessFlags {
    RdMode = 0,   // rdonly channel
    WrMode = 1,   // wronly channel
    RdwrMode = 2, //rdwr channel
}

impl AccessFlags {
    #[allow(dead_code)]
    fn value(&self) -> u8 {
        match *self {
            AccessFlags::RdMode => 0,
            AccessFlags::WrMode => 1,
            AccessFlags::RdwrMode => 2,
        }
    }
}

// ==== read/write ====
/* 
pub fn open_channel(path: &str, mode: AccessFlags) -> std::fs::File {
    let mut options = OpenOptions::new();
    if mode == AccessFlags::RdwrMode || mode == AccessFlags::RdMode {
        options.read(true);
    }

    if mode == AccessFlags::RdwrMode || mode == AccessFlags::WrMode {
        options.write(true);
    }

    if cfg!(unix) {
        options.custom_flags(libc::O_SYNC);
        // options.custom_flags(libc::O_RDWR);
    }
    options.open(path).unwrap()
} */

pub fn open_channel(path: &str, mode: AccessFlags) -> std::fs::File {
    let mut options = OpenOptions::new();
    if mode == AccessFlags::RdwrMode || mode == AccessFlags::RdMode {
        options.read(true);
    }

    if mode == AccessFlags::RdwrMode || mode == AccessFlags::WrMode {
        options.write(true);
    }

    if cfg!(unix) {
        options.custom_flags(libc::O_SYNC);
        // options.custom_flags(libc::O_RDWR);
    }
    options.open(path).unwrap()
}


pub fn read_binary_file(path: &str) -> Result<Vec<u8>, Error> {
    let mut buffer = Vec::new();
    log::debug!("Trying to open file: {:?}", path);
    let mut file = File::open(path)?;
    file.read_to_end(&mut buffer)?;

    Ok(buffer)
}

// ==== conversions ====

// pub fn u8_vec_to_u32_sum(arr: &[u8]) -> u32 {
//     arr.iter()
//         .enumerate()
//         .map(|(i, v)| (*v as u32) << (8 * i))
//         .sum::<u32>()
// }

// pub fn u32_arr_to_u8_arr(input: &[u32]) -> Vec<u8> {
//     input
//         .iter()
//         .flat_map(|v| v.to_le_bytes())
//         .collect::<Vec<u8>>()
// }

// pub fn u8_vec_to_u32_vec(arr: &[u8]) -> Vec<u32> {
//     arr.chunks(4)
//         .into_iter()
//         .map(|chunk| u32::from_le_bytes(chunk.try_into().unwrap()))
//         .collect::<Vec<u32>>()
// }

// pub fn u8_to_bool(v: u8) -> bool {
//     match v {
//         0 => false,
//         1 => true,
//         _ => panic!("Invalid bool in u8 {}", v),
//     }
// }

pub fn u32_to_u8_vec_resize(data: u32, size: usize, fill: u8) -> Vec<u8> {
    let mut buf = data.to_le_bytes().to_vec();
    buf.resize(size, fill);
    buf
}

pub fn convert_to_32_byte_array(init: &[u8]) -> [u8; 32] {
    assert!(32 >= init.len());

    let mut arr: [u8; 32] = [0; 32];
    arr[..init.len()].copy_from_slice(init);

    arr
}

// ==== general ====
pub fn retry<T: Copy, R>(
    args: T,
    times: usize,
    callback: fn(_: T) -> anyhow::Result<R>,
) -> anyhow::Result<R> {
    std::iter::from_fn(|| Some(callback(args)))
        .inspect(|r| {
            if r.is_err() {
                thread::sleep(time::Duration::from_secs(1));
            }
        })
        .take(times)
        .find(Result::is_ok)
        .unwrap_or_else(|| callback(args))
}
