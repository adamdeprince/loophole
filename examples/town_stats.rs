//! Prints what the town generator made of the town file.
use silkie_loophole::world::{grid::Terrain, plan, town};
fn main() {
    let t = plan::build(&town::parse(town::MONTPELIER).unwrap());
    let mut kinds = std::collections::BTreeMap::new();
    for b in &t.buildings { *kinds.entry(format!("{:?}", b.kind)).or_insert(0) += 1; }
    println!("buildings {kinds:?}");
    let mut fk = std::collections::BTreeMap::new();
    for f in &t.features { *fk.entry(format!("{:?}", f.kind)).or_insert(0) += 1; }
    println!("features {fk:?}");
    for tr in [Terrain::Grass, Terrain::Yard, Terrain::Woods, Terrain::Water, Terrain::Bridge, Terrain::Road, Terrain::Sidewalk, Terrain::Path, Terrain::Building] {
        println!("{tr:?}: {}", t.grid.count(tr));
    }
    let mut per_street = std::collections::BTreeMap::new();
    for b in &t.buildings { if let Some(s) = &b.street { *per_street.entry(s.clone()).or_insert(0) += 1; } }
    println!("{per_street:#?}");
    let spec = town::parse(town::MONTPELIER).unwrap();
    for f in &spec.features {
        let tt = t.grid.at(f.pos);
        if tt == Terrain::Building || (tt == Terrain::Water) { println!("dropped {:?} at {:?} on {tt:?}", f.kind, f.pos); }
    }
}
#[allow(dead_code)]
fn dropped() {}
