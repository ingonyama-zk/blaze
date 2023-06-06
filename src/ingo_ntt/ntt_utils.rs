use serde::{de, Deserialize};
pub const NOF_ADDRS: usize = 16;

pub fn deserialize_hex_array<'de, D: serde::Deserializer<'de>>(
    d: D,
) -> std::result::Result<[u64; NOF_ADDRS], D::Error> {
    let mut res: [u64; NOF_ADDRS] = [0; NOF_ADDRS];
    let hexes: [String; NOF_ADDRS] = Deserialize::deserialize(d)?;

    for (i, hex) in hexes.into_iter().enumerate() {
        res[i] =
            u64::from_str_radix(hex.trim_start_matches("0x"), 16).map_err(de::Error::custom)?;
    }
    Ok(res)
}
