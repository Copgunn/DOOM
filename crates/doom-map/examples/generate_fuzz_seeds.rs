//! Regenerate only original synthetic fuzz seeds from shared fixture builders.

#[path = "../../../tests/support/mod.rs"]
mod support;

fn main() -> std::io::Result<()> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fuzz/corpus");
    std::fs::create_dir_all(root.join("wad"))?;
    std::fs::create_dir_all(root.join("map"))?;
    std::fs::write(root.join("wad/empty_iwad"), support::wad(*b"IWAD", &[]))?;
    std::fs::write(
        root.join("wad/duplicates"),
        support::wad(
            *b"PWAD",
            &[
                (support::name("TEST"), vec![1]),
                (support::name("test"), vec![]),
            ],
        ),
    )?;
    std::fs::write(
        root.join("map/classic_square"),
        support::wad(*b"PWAD", &support::classic_map("MAP01")),
    )?;
    Ok(())
}
