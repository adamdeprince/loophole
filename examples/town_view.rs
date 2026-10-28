//! Paints part of the town's scenery (ground and canopy) to a PNG, through
//! the same chunk scenes the terminal gets:
//! `cargo run --release --example town_view -- x y w h px out.png`
use silkie_loophole::gfx::scenery::{self, Layer};
use silkie_loophole::world::{plan, town};
use tiny_skia::{Pixmap, PixmapPaint, Transform};
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let f = |i: usize, d: f32| a.get(i).and_then(|s| s.parse().ok()).unwrap_or(d);
    let (x, y, w, h, px) = (f(1, 110.0), f(2, 55.0), f(3, 40.0), f(4, 25.0), f(5, 32.0) as u32);
    let out = a.get(6).cloned().unwrap_or("view.png".into());
    let t = plan::build(&town::parse(town::MONTPELIER).unwrap());
    let art = scenery::build(&t, false);
    let tiles = 20u32;
    let (cw, _ch, pieces, slots) = scenery::chunk_index(&art, tiles);
    let states = vec![1u32; art.slots.len()];
    let mut pm = Pixmap::new((w * px as f32) as u32, (h * px as f32) as u32).unwrap();
    let started = std::time::Instant::now();
    let mut n = 0;
    for layer in [Layer::Ground, Layer::Canopy] {
        for cy in (y / tiles as f32).floor() as u32..=((y + h) / tiles as f32).floor() as u32 {
            for cx in (x / tiles as f32).floor() as u32..=((x + w) / tiles as f32).floor() as u32 {
                let i = (cy * cw + cx) as usize;
                if i >= pieces.len() { continue; }
                let origin = glam::Vec2::new((cx * tiles) as f32, (cy * tiles) as f32);
                let s = scenery::chunk_scene(&art, layer, origin, tiles, px, &pieces[i], &slots[i], &states, 0);
                let mut c = Pixmap::new(tiles * px, tiles * px).unwrap();
                s.render(&mut c, Transform::identity());
                n += 1;
                let ox = ((origin.x - x) * px as f32) as i32;
                let oy = ((origin.y - y) * px as f32) as i32;
                pm.draw_pixmap(ox, oy, c.as_ref(), &PixmapPaint::default(), Transform::identity(), None);
            }
        }
    }
    eprintln!("{n} chunks in {:?}, {} pieces, {} decals", started.elapsed(), art.pieces.len(), art.slots.len());
    pm.save_png(&out).unwrap();
}
