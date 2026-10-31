//! Offline test/tool evidence codec. Never a production simulation dependency.
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{fs::File, io::Read, path::Path, sync::OnceLock};

/// Maximum decoded bytes accepted for one trace.
pub const MAX_DECODED: usize = 32 * 1024 * 1024;
/// Maximum compressed bytes accepted for one trace.
pub const MAX_COMPRESSED: usize = 8 * 1024 * 1024;
/// Maximum aggregate declared decode budgets in one catalog.
pub const MAX_TOTAL: usize = 96 * 1024 * 1024;
/// Maximum UTF-8 manifest bytes admitted before JSON parsing.
pub const MAX_MANIFEST: usize = 128 * 1024;
/// Immutable post-M3g9 Git object used by this migration contract.
pub const BASELINE: &str = "138c66b6c6b6d57d0b18cf945d8661722741bd66";
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Deterministic checked evidence failure; never an executable-world error.
pub struct Error(#[doc = "Stable failure category."] pub &'static str);
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0)
    }
}
impl std::error::Error for Error {}
type Result<T> = std::result::Result<T, Error>;

/// Return the lowercase SHA-256 identity of exact bytes.
pub fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn hex(value: &str, length: usize) -> bool {
    value.len() == length
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
fn filename(path: &str, suffix: &str) -> bool {
    path.strip_prefix("tests/reference/").is_some_and(|name| {
        !name.is_empty()
            && name.ends_with(suffix)
            && !name.contains("..")
            && name
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"_-.".contains(&b))
    })
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
/// One original trace and its lossless compressed replacement.
pub struct Entry {
    /// Original repository-relative CSV path.
    pub original_path: String,
    /// The original path with a .zst suffix.
    pub compressed_path: String,
    /// Exact original byte length.
    pub original_length: usize,
    /// Original exact-byte SHA-256 captured before transformation.
    pub original_sha256: String,
    /// Original Git blob identity, independently audited against Git bytes.
    pub original_git_blob: String,
    /// Exact single-frame byte length.
    pub compressed_length: usize,
    /// Exact committed compressed-byte SHA-256.
    pub compressed_sha256: String,
    /// Checked output allocation limit, at least original_length.
    pub decode_budget: usize,
    /// Historical source-evidence family, not a gameplay capability.
    pub family: String,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
/// Strict version-one migration catalog, separate from DSIM.
pub struct Manifest {
    /// Evidence catalog identifier; not a simulation schema.
    pub schema: String,
    /// Immutable Git baseline for independent migration proof.
    pub baseline: String,
    /// Pinned encoder identity.
    pub encoder: String,
    /// Pinned deterministic settings.
    pub parameters: String,
    /// Strictly ordered, unique bounded trace entries.
    pub entries: Vec<Entry>,
}
/// Pinned encoder and bundled native format implementation.
pub const ENCODER: &str = "zstd crate 0.13.3; zstd-safe 7.3.0; libzstd 1.5.7";
/// Exact deterministic encoding settings used for committed frames.
pub const PARAMETERS: &str =
    "level=9,window_log=23,checksum=true,content_size=true,workers=0,dictionary=none";
impl Manifest {
    /// Validate schema, paths, identity syntax, order and resource budgets.
    pub fn parse(text: &str) -> Result<Self> {
        if text.len() > MAX_MANIFEST {
            return Err(Error("manifest budget"));
        }
        let m: Self = serde_json::from_str(text).map_err(|_| Error("invalid manifest JSON"))?;
        if m.schema != "DOOM-ZSTD-EVIDENCE-v1"
            || m.baseline != BASELINE
            || m.encoder != ENCODER
            || m.parameters != PARAMETERS
        {
            return Err(Error("incompatible manifest contract"));
        }
        if m.entries.is_empty() || m.entries.len() > 64 {
            return Err(Error("entry count"));
        }
        let (mut decoded, mut compressed) = (0usize, 0usize);
        let mut previous = "";
        for e in &m.entries {
            if !filename(&e.original_path, ".csv")
                || e.compressed_path != format!("{}.zst", e.original_path)
                || e.original_path.as_str() <= previous
                || e.family.is_empty()
                || e.family.len() > 32
                || !e
                    .family
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'_')
                || !hex(&e.original_sha256, 64)
                || !hex(&e.compressed_sha256, 64)
                || !hex(&e.original_git_blob, 40)
            {
                return Err(Error("invalid manifest entry"));
            }
            if e.original_length == 0
                || e.original_length > e.decode_budget
                || e.decode_budget > MAX_DECODED
                || e.compressed_length == 0
                || e.compressed_length > MAX_COMPRESSED
            {
                return Err(Error("entry budget"));
            }
            decoded = decoded
                .checked_add(e.decode_budget)
                .ok_or(Error("total overflow"))?;
            compressed = compressed
                .checked_add(e.compressed_length)
                .ok_or(Error("total overflow"))?;
            if decoded > MAX_TOTAL || compressed > 32 * 1024 * 1024 {
                return Err(Error("total budget"));
            }
            previous = &e.original_path;
        }
        Ok(m)
    }
}

/// Exactly one standard, checksummed, size-bearing frame, no dictionary.
/// Header/window budgets are checked before creating a native decode context.
pub fn decode(data: &[u8], expected: usize, budget: usize) -> Result<Vec<u8>> {
    if data.len() > MAX_COMPRESSED || expected == 0 || expected > budget || budget > MAX_DECODED {
        return Err(Error("decode budget"));
    }
    if data.len() < 6 || data[..4] != [0x28, 0xb5, 0x2f, 0xfd] {
        return Err(Error("frame magic/truncation"));
    }
    let descriptor = data[4];
    if descriptor & 0x1b != 0 || descriptor & 4 == 0 {
        return Err(Error("frame flags/dictionary/checksum"));
    }
    if descriptor & 0x20 == 0 {
        let wd = data[5];
        let base = 1u64
            .checked_shl(10 + u32::from(wd >> 3))
            .ok_or(Error("window overflow"))?;
        let window = base
            .checked_add((base / 8) * u64::from(wd & 7))
            .ok_or(Error("window overflow"))?;
        if window > 8 * 1024 * 1024 {
            return Err(Error("window budget"));
        }
    } else if expected > 8 * 1024 * 1024 {
        return Err(Error("single segment window budget"));
    }
    let size =
        zstd::zstd_safe::get_frame_content_size(data).map_err(|_| Error("invalid frame header"))?;
    if size != Some(expected as u64) {
        return Err(Error("declared size mismatch"));
    }
    let consumed =
        zstd::zstd_safe::find_frame_compressed_size(data).map_err(|_| Error("invalid frame"))?;
    if consumed != data.len() {
        return Err(Error("trailing bytes/extra frames"));
    }
    let mut out = Vec::new();
    out.try_reserve_exact(expected)
        .map_err(|_| Error("output allocation"))?;
    out.resize(expected, 0);
    let mut decoder = zstd::bulk::Decompressor::new().map_err(|_| Error("decoder context"))?;
    decoder
        .set_parameter(zstd::zstd_safe::DParameter::WindowLogMax(23))
        .map_err(|_| Error("decoder window"))?;
    let written = decoder
        .decompress_to_buffer(data, &mut out[..])
        .map_err(|_| Error("corrupt/truncated frame"))?;
    if written != expected {
        return Err(Error("decoded size mismatch"));
    }
    Ok(out)
}
/// Verify compressed identity, decode exactly, then verify original identity.
pub fn decode_entry(entry: &Entry, data: &[u8]) -> Result<Vec<u8>> {
    if data.len() != entry.compressed_length || sha256(data) != entry.compressed_sha256 {
        return Err(Error("compressed identity mismatch"));
    }
    let out = decode(data, entry.original_length, entry.decode_budget)?;
    if sha256(&out) != entry.original_sha256 {
        return Err(Error("decoded identity mismatch"));
    }
    Ok(out)
}
/// Reproducible maintenance encoder, also independently round-trip checked.
pub fn encode(raw: &[u8]) -> Result<Vec<u8>> {
    if raw.is_empty() || raw.len() > MAX_DECODED {
        return Err(Error("encoder budget"));
    }
    if zstd::zstd_safe::version_string() != "1.5.7" {
        return Err(Error("encoder version drift"));
    }
    let mut c = zstd::bulk::Compressor::new(9).map_err(|_| Error("encoder context"))?;
    for parameter in [
        zstd::zstd_safe::CParameter::WindowLog(23),
        zstd::zstd_safe::CParameter::ChecksumFlag(true),
        zstd::zstd_safe::CParameter::ContentSizeFlag(true),
        zstd::zstd_safe::CParameter::NbWorkers(0),
    ] {
        c.set_parameter(parameter)
            .map_err(|_| Error("encoder parameter"))?;
    }
    let data = c.compress(raw).map_err(|_| Error("encode"))?;
    let out = decode(&data, raw.len(), raw.len())?;
    if out != raw {
        return Err(Error("roundtrip byte mismatch"));
    }
    Ok(data)
}
/// Bounded read, including a one-byte sentinel for growth during reading.
pub fn read_bounded(path: &Path, limit: usize) -> Result<Vec<u8>> {
    let bound = limit.checked_add(1).ok_or(Error("read budget overflow"))?;
    let file = File::open(path).map_err(|_| Error("missing/unreadable evidence"))?;
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(bound)
        .map_err(|_| Error("input allocation"))?;
    file.take(bound as u64)
        .read_to_end(&mut bytes)
        .map_err(|_| Error("evidence read"))?;
    if bytes.len() > limit {
        return Err(Error("file budget"));
    }
    Ok(bytes)
}
/// Read one listed frame within its root and identity/resource limits.
pub fn load(root: &Path, entry: &Entry) -> Result<Vec<u8>> {
    if !filename(&entry.compressed_path, ".csv.zst") {
        return Err(Error("unsafe path"));
    }
    let root = root.canonicalize().map_err(|_| Error("evidence root"))?;
    let path = root
        .join(&entry.compressed_path)
        .canonicalize()
        .map_err(|_| Error("missing evidence"))?;
    if !path.starts_with(&root) {
        return Err(Error("path escapes root"));
    }
    decode_entry(
        entry,
        &read_bounded(&path, entry.compressed_length.min(MAX_COMPRESSED))?,
    )
}

struct CachedText {
    text: String,
    compressed: &'static [u8],
}
struct Cache {
    manifest_text: &'static str,
    manifest: Manifest,
    entries: Vec<OnceLock<Result<CachedText>>>,
}
static CACHE: OnceLock<Cache> = OnceLock::new();
/// Checked replacement for embedded historical include_str data. Immutable,
/// bounded cache belongs only to the test executable, not the game runtime.
pub fn text(
    name: &str,
    compressed: &'static [u8],
    manifest_text: &'static str,
) -> Result<&'static str> {
    if CACHE.get().is_none() {
        let manifest = Manifest::parse(manifest_text)?;
        let entries = (0..manifest.entries.len())
            .map(|_| OnceLock::new())
            .collect();
        let _ = CACHE.set(Cache {
            manifest_text,
            manifest,
            entries,
        });
    }
    let cache = CACHE.get().ok_or(Error("cache initialization"))?;
    if cache.manifest_text != manifest_text {
        return Err(Error("cache manifest mismatch"));
    }
    let index = cache
        .manifest
        .entries
        .iter()
        .position(|e| e.original_path == name)
        .ok_or(Error("unlisted evidence"))?;
    let entry = &cache.manifest.entries[index];
    if let Some(Ok(cached)) = cache.entries[index].get() {
        if std::ptr::eq(cached.compressed, compressed) {
            return Ok(&cached.text);
        }
    }
    if compressed.len() != entry.compressed_length || sha256(compressed) != entry.compressed_sha256
    {
        return Err(Error("compressed identity mismatch"));
    }
    let result = cache.entries[index].get_or_init(|| {
        let bytes = decode_entry(entry, compressed)?;
        Ok(CachedText {
            text: String::from_utf8(bytes).map_err(|_| Error("invalid UTF-8 evidence"))?,
            compressed,
        })
    });
    match result {
        Ok(cached) if cached.compressed == compressed => Ok(&cached.text),
        Ok(_) => Err(Error("cache input mismatch")),
        Err(error) => Err(*error),
    }
}

#[cfg(test)]
mod tests;
