#![no_main]
use libfuzzer_sys::fuzz_target;
fuzz_target!(|input: &[u8]| {
    if input.len() < 4 {
        return;
    }
    let expected = u32::from_le_bytes([input[0], input[1], input[2], input[3]]) as usize;
    let _ = doom_test_evidence::decode(&input[4..], expected, 1024 * 1024);
    let payload = &input[4..];
    let entry = doom_test_evidence::Entry {
        original_path: "tests/reference/fuzz.csv".into(),
        compressed_path: "tests/reference/fuzz.csv.zst".into(),
        original_length: expected,
        original_sha256: "0".repeat(64),
        original_git_blob: "0".repeat(40),
        compressed_length: if input[0] & 1 == 0 {
            payload.len()
        } else {
            expected
        },
        compressed_sha256: doom_test_evidence::sha256(payload),
        decode_budget: 1024 * 1024,
        family: "fuzz".into(),
    };
    let _ = doom_test_evidence::decode_entry(&entry, payload);
    if let Ok(text) = std::str::from_utf8(input) {
        let _ = doom_test_evidence::Manifest::parse(text);
    }
});
