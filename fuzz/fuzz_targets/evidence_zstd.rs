#![no_main]
use libfuzzer_sys::fuzz_target;
fuzz_target!(|input: &[u8]| {
    if input.len() < 4 {
        return;
    }
    let expected = u32::from_le_bytes([input[0], input[1], input[2], input[3]]) as usize;
    let _ = doom_test_evidence::decode(&input[4..], expected, 1024 * 1024);
    if let Ok(text) = std::str::from_utf8(input) {
        let _ = doom_test_evidence::Manifest::parse(text);
    }
});
