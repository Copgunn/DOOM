// Direct-call probe: movement::path of the scratch copy on a WAD's map, one call per line of a list.
fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let bytes = std::fs::read(&a[0]).unwrap();
    let wad = doom_wad::Wad::parse(&bytes, doom_wad::Limits::default()).unwrap();
    let map = doom_map::Map::from_wad(&wad, "E1M1", doom_map::MapLimits::default()).unwrap();
    let calls: Vec<[i32; 4]> = std::fs::read_to_string(&a[1]).unwrap().lines().filter(|l| !l.trim().is_empty())
        .map(|l| { let v: Vec<i32> = l.split_whitespace().map(|x| x.parse().unwrap()).collect(); [v[0], v[1], v[2], v[3]] })
        .collect();
    for hits in doom_sim::corner_direct_paths(map, &calls) {
        let s: Vec<String> = hits.iter().map(|(f, i)| format!("{f}:{i}")).collect();
        println!("hits {}", s.join(" "));
    }
}
