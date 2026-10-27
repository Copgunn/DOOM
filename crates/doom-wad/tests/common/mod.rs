//! Synthetic data only: an empty classic WAD, with its directory at byte 12.

pub fn empty_wad(magic: [u8; 4]) -> Vec<u8> {
    let mut bytes = Vec::from(magic);
    bytes.extend_from_slice(&0_i32.to_le_bytes());
    bytes.extend_from_slice(&12_i32.to_le_bytes());
    bytes
}
