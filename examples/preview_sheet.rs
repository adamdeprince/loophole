//! Paints a sprite sheet to a PNG for review, on a dark background:
//! `cargo run --release --example preview_sheet -- NAME TILE_PX out.png [worn]`
use silkie_loophole::sim::candy::Mat;
use silkie_loophole::sim::costume::{Costume, Slot};
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let name = a.get(1).cloned().unwrap_or("raccoon".into());
    let px: u32 = a.get(2).and_then(|s| s.parse().ok()).unwrap_or(64);
    let out = a.get(3).cloned().unwrap_or("sheet.png".into());
    let worn = a.get(4).is_some();
    let mut c = Costume::default();
    if worn {
        c.slots[1] = Slot::Hole;
        c.slots[4] = Slot::Snag;
        c.slots[6] = Slot::Patched(Mat::Gum, 1.0);
        c.slots[9] = Slot::Patched(Mat::Tape, 1.0);
        c.slots[10] = Slot::Hole;
        c.slots[3] = Slot::Patched(Mat::Flyer, 1.0);
        c.dirt = 0.7;
        c.leaves = 2;
    }
    let (spec, pm) = silkie_loophole::render::paint_named(&name, px, &c.look(if worn { 3 } else { 0 })).expect("no such sheet");
    let mut bg = tiny_skia::Pixmap::new(pm.width(), pm.height()).unwrap();
    bg.fill(tiny_skia::Color::from_rgba8(44, 68, 41, 255));
    bg.draw_pixmap(0, 0, pm.as_ref(), &tiny_skia::PixmapPaint::default(), tiny_skia::Transform::identity(), None);
    bg.save_png(&out).unwrap();
    println!("{}: {} frames, {:?}", spec.name, spec.frames, spec.anims.keys().collect::<Vec<_>>());
}
