//! Run the public APIs on the original synthetic square fixture, without IWADs.

use doom_map::{Map, MapLimits};
use doom_wad::{Limits, Wad, WadSet};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let bytes = include_bytes!("../../../fuzz/corpus/map/classic_square");
    let archives = [Wad::parse(bytes, Limits::default())?];
    let resources = WadSet::new(&archives);
    let map = Map::load(&resources, "MAP01", MapLimits::default())?;
    println!(
        "Synthetic map: {} vertices, {} linedefs, {} sectors",
        map.vertices().len(),
        map.linedefs().len(),
        map.sectors().len()
    );
    Ok(())
}
