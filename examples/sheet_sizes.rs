//! Prints how big every sheet is at a tile size, as pixels and as vector records.
use silkie_loophole::render::{SheetId, paint_sheet};
fn main() {
    let px: u32 = std::env::args().nth(1).and_then(|s| s.parse().ok()).unwrap_or(119);
    let look = silkie_loophole::sim::costume::Costume::default().look(1);
    let mut total = 0u64;
    let mut vtotal = 0usize;
    for id in SheetId::all() {
        let p = paint_sheet(id, px, &look, false);
        let v = paint_sheet(id, px, &look, true);
        let mut scene = silkie_loophole::gfx::vector::Scene::new(v.width as f32, v.height as f32);
        for (i, e) in &v.elements { scene.set(*i, e.clone()); }
        let rec = scene.records();
        let z = silkie_loophole::term::kitty_gfx::zlib(rec.as_bytes()).len();
        let bytes = p.width as u64 * p.height as u64 * 4;
        total += bytes;
        vtotal += z;
        if !matches!(id, SheetId::Folk(n) if n > 0) {
            println!("{id:?}: {}x{} = {:.1} MB rgba; vector {} KB records, {} KB compressed", p.width, p.height, bytes as f64 / 1e6, rec.len() / 1024, z / 1024);
        }
    }
    println!("all sheets: {:.1} MB rgba, {:.0} KB vector compressed", total as f64 / 1e6, vtotal as f64 / 1024.0);
}
