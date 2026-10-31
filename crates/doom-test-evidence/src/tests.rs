use super::*;
fn entry(raw: &[u8], data: &[u8]) -> Entry {
    Entry {
        original_path: "tests/reference/test.csv".into(),
        compressed_path: "tests/reference/test.csv.zst".into(),
        original_length: raw.len(),
        original_sha256: sha256(raw),
        original_git_blob: "a".repeat(40),
        compressed_length: data.len(),
        compressed_sha256: sha256(data),
        decode_budget: raw.len(),
        family: "test".into(),
    }
}
fn manifest(e: Entry) -> Manifest {
    Manifest {
        schema: "DOOM-ZSTD-EVIDENCE-v1".into(),
        baseline: BASELINE.into(),
        encoder: ENCODER.into(),
        parameters: PARAMETERS.into(),
        entries: vec![e],
    }
}
#[test]
fn roundtrip_is_deterministic() {
    let raw = b"original source evidence\n".repeat(200);
    let z = encode(&raw).unwrap();
    assert_eq!(z, encode(&raw).unwrap());
    assert_eq!(decode_entry(&entry(&raw, &z), &z).unwrap(), raw);
}
#[test]
fn every_truncation_and_corruption_rejects() {
    let raw = b"original source evidence\n".repeat(30);
    let z = encode(&raw).unwrap();
    for n in 0..z.len() {
        assert!(decode(&z[..n], raw.len(), raw.len()).is_err(), "prefix {n}");
    }
    for n in 0..z.len() {
        let mut bad = z.clone();
        bad[n] ^= 0x80;
        assert!(decode_entry(&entry(&raw, &z), &bad).is_err());
    }
    let mut bad = z.clone();
    let n = bad.len() - 1;
    bad[n] ^= 1;
    assert!(decode(&bad, raw.len(), raw.len()).is_err());
}
#[test]
fn exact_frame_consumption() {
    let raw = b"evidence";
    let z = encode(raw).unwrap();
    for tail in [&b"junk"[..], &z[..]] {
        let mut bad = z.clone();
        bad.extend(tail);
        assert!(decode(&bad, raw.len(), raw.len()).is_err());
    }
    let mut bad = z.clone();
    bad[0] = 0x50;
    assert!(decode(&bad, raw.len(), raw.len()).is_err());
}
#[test]
fn budgets_sizes_and_frame_contract() {
    let raw = b"data";
    let z = encode(raw).unwrap();
    for (expected, budget) in [
        (0, 1),
        (usize::MAX, usize::MAX),
        (MAX_DECODED + 1, MAX_DECODED + 1),
        (4, 3),
        (5, 5),
    ] {
        assert!(decode(&z, expected, budget).is_err());
    }
    for flags in [1, 2, 8, 16] {
        let mut bad = z.clone();
        bad[4] |= flags;
        assert!(decode(&bad, 4, 4).is_err());
    }
    let mut bad = z.clone();
    bad[4] &= !4;
    assert!(decode(&bad, 4, 4).is_err());
    let mut window = vec![0x28, 0xb5, 0x2f, 0xfd, 4, 0xff];
    window.extend(&z[6..]);
    assert!(decode(&window, 4, 4).is_err());
    assert!(read_bounded(Path::new("missing-evidence-never-created"), usize::MAX).is_err());
}
#[test]
fn hashes_are_separate_guards() {
    let raw = b"data";
    let z = encode(raw).unwrap();
    let mut e = entry(raw, &z);
    e.compressed_sha256 = "0".repeat(64);
    assert_eq!(
        decode_entry(&e, &z),
        Err(Error("compressed identity mismatch"))
    );
    e.compressed_sha256 = sha256(&z);
    e.original_sha256 = "0".repeat(64);
    assert_eq!(
        decode_entry(&e, &z),
        Err(Error("decoded identity mismatch"))
    );
}
#[test]
fn manifest_schema_paths_and_totals() {
    let z = encode(b"data").unwrap();
    let good = manifest(entry(b"data", &z));
    let json = serde_json::to_string(&good).unwrap();
    assert!(Manifest::parse(&json).is_ok());
    assert!(Manifest::parse(&(json.clone() + "junk")).is_err());
    assert!(Manifest::parse(&" ".repeat(MAX_MANIFEST + 1)).is_err());
    for path in [
        "../test.csv",
        "/test.csv",
        "tests/reference/../test.csv",
        "tests/reference/a\\b.csv",
        "tests/reference/C:test.csv",
    ] {
        let mut m = good.clone();
        m.entries[0].original_path = path.into();
        m.entries[0].compressed_path = format!("{path}.zst");
        assert!(Manifest::parse(&serde_json::to_string(&m).unwrap()).is_err());
    }
    let mut duplicate = good.clone();
    duplicate.entries.push(duplicate.entries[0].clone());
    assert!(Manifest::parse(&serde_json::to_string(&duplicate).unwrap()).is_err());
    for length in [0, MAX_DECODED + 1, usize::MAX] {
        let mut m = good.clone();
        m.entries[0].decode_budget = length;
        assert!(Manifest::parse(&serde_json::to_string(&m).unwrap()).is_err());
    }
    let mut m = good.clone();
    m.entries.clear();
    assert!(Manifest::parse(&serde_json::to_string(&m).unwrap()).is_err());
    for i in 0..4 {
        let mut e = good.entries[0].clone();
        e.original_path = format!("tests/reference/t{i}.csv");
        e.compressed_path = format!("{}.zst", e.original_path);
        e.decode_budget = MAX_DECODED;
        m.entries.push(e);
    }
    assert!(Manifest::parse(&serde_json::to_string(&m).unwrap()).is_err());
    assert!(load(Path::new("."), &good.entries[0]).is_err());
}
#[test]
fn unknown_fields_are_rejected() {
    let z = encode(b"data").unwrap();
    let m = manifest(entry(b"data", &z));
    let mut value = serde_json::to_value(m).unwrap();
    value["unexpected"] = true.into();
    assert!(Manifest::parse(&value.to_string()).is_err());
    value.as_object_mut().unwrap().remove("unexpected");
    value["entries"][0]["unexpected"] = true.into();
    assert!(Manifest::parse(&value.to_string()).is_err());
}
