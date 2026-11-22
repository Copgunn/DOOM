use doom_sim::ArithmeticCertificate;
fn main() {
    for path in std::env::args().skip(1) {
        let bytes = std::fs::read(&path).unwrap();
        let wad = doom_wad::Wad::parse(&bytes, doom_wad::Limits::default()).unwrap();
        let map = doom_map::Map::from_wad(&wad, "E1M1", doom_map::MapLimits::default()).unwrap();
        match ArithmeticCertificate::evaluate(&map) {
            Ok((c, clauses)) => println!("{path}: {:?} {:?}", clauses, c),
            Err(e) => println!("{path}: error {e:?}"),
        }
    }
}
