//! Paints the town map to a PNG: `cargo run --release --example map_view -- WIDTH out.png`
use silkie_loophole::world::{plan, town};
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let w: u32 = a.get(1).and_then(|s| s.parse().ok()).unwrap_or(1600);
    let t = plan::build(&town::parse(town::MONTPELIER).unwrap());
    let s = silkie_loophole::gfx::minimap::scene(&t);
    let k = w as f32 / s.width;
    let mut pm = tiny_skia::Pixmap::new(w, (s.height * k) as u32).unwrap();
    s.render(&mut pm, tiny_skia::Transform::from_scale(k, k));
    pm.save_png(a.get(2).cloned().unwrap_or("map.png".into())).unwrap();
}
