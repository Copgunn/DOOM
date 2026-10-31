//! Bounded test-evidence decoder. Never used by simulation or save restoration.
#![allow(dead_code)]
const LIMIT: usize = 16 * 1024 * 1024;
pub fn unpack(data: &[u8]) -> Result<Vec<u8>, &'static str> {
    if data.len() < 12 || data.len() > LIMIT + LIMIT / 8 + 12 || data.get(..8) != Some(b"DREFLZ1\0")
    {
        return Err("framing");
    }
    let size = usize::try_from(u32::from_le_bytes(
        data[8..12].try_into().map_err(|_| "size")?,
    ))
    .map_err(|_| "size")?;
    if size > LIMIT {
        return Err("limit");
    }
    let mut out = Vec::new();
    out.try_reserve_exact(size).map_err(|_| "allocation")?;
    let mut at = 12;
    while out.len() < size {
        let flags = *data.get(at).ok_or("flags")?;
        at += 1;
        for bit in 0..8 {
            if out.len() == size {
                if flags >> bit != 0 {
                    return Err("unused flags");
                }
                break;
            }
            if flags & (1 << bit) == 0 {
                out.push(*data.get(at).ok_or("literal")?);
                at += 1;
            } else {
                let token = data.get(at..at + 3).ok_or("reference")?;
                at += 3;
                let distance = usize::from(u16::from_le_bytes([token[0], token[1]]));
                let count = usize::from(token[2]) + 3;
                if distance == 0 || distance > out.len() || count > size - out.len() {
                    return Err("reference bounds");
                }
                for _ in 0..count {
                    let byte = out[out.len() - distance];
                    out.push(byte);
                }
            }
        }
    }
    if at != data.len() {
        return Err("trailing");
    }
    Ok(out)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn literal_and_overlap_golden_and_malformed_frames() {
        let mut b = b"DREFLZ1\0".to_vec();
        b.extend(9u32.to_le_bytes());
        b.extend(b"\x08abc\x03\x00\x03");
        assert_eq!(unpack(&b).unwrap(), b"abcabcabc");
        for length in 0..b.len() {
            assert!(unpack(&b[..length]).is_err());
        }
        let mut extra = b.clone();
        extra.push(0);
        assert!(unpack(&extra).is_err());
        for token in [[0, 0, 3], [4, 0, 3], [3, 0, 4]] {
            let mut bad = b.clone();
            bad[16..19].copy_from_slice(&token);
            assert!(unpack(&bad).is_err());
        }
        let mut bad = b.clone();
        bad[12] |= 128;
        assert!(unpack(&bad).is_err());
        let mut bad = b.clone();
        bad[8..12].copy_from_slice(&(LIMIT as u32 + 1).to_le_bytes());
        assert!(unpack(&bad).is_err());
    }
}
