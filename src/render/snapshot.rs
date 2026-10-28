//! A picture of a moment without a terminal: the same chunk scenes and the
//! same sprite sheets the terminal is sent, composited in the same order.
//! For `silkie-loophole snapshot`, the README and art review.

use super::{FrameModel, Layer, SheetId, paint_sheet};
use crate::gfx::scenery::{self, Layer as ArtLayer};
use crate::sim::costume::Costume;
use glam::Vec2;
use std::collections::BTreeMap;
use tiny_skia::{Pixmap, PixmapPaint, Transform};

/// Paints `f` (its camera, decals and sprites) into a `w` by `h` picture at
/// `tile_px` pixels per tile.
pub fn snapshot(f: &FrameModel, w: u32, h: u32, tile_px: u32) -> Pixmap {
    let mut pm = Pixmap::new(w.max(1), h.max(1)).expect("picture size");
    pm.fill(tiny_skia::Color::from_rgba8(20, 24, 30, 255));
    let Some(t) = &f.town else { return pm };
    let art = &t.art;
    let tiles = super::chunk_tiles(tile_px);
    let (cw, ch, pieces, slots) = scenery::chunk_index(art, tiles);
    let view = Vec2::new(w as f32 / tile_px as f32, h as f32 / tile_px as f32);
    let tf = tiles as f32;
    let x0 = (f.cam.x / tf).floor().max(0.0) as u32;
    let y0 = (f.cam.y / tf).floor().max(0.0) as u32;
    let x1 = (((f.cam.x + view.x) / tf).floor().max(0.0) as u32).min(cw - 1);
    let y1 = (((f.cam.y + view.y) / tf).floor().max(0.0) as u32).min(ch - 1);
    let chunk = |pm: &mut Pixmap, layer: ArtLayer| {
        for cy in y0..=y1 {
            for cx in x0..=x1 {
                let i = (cy * cw + cx) as usize;
                let origin = Vec2::new((cx * tiles) as f32, (cy * tiles) as f32);
                let s = scenery::chunk_scene(art, layer, origin, tiles, tile_px, &pieces[i], &slots[i], &f.decals, f.flicker);
                let ox = ((origin.x - f.cam.x) * tile_px as f32).round() as i32;
                let oy = ((origin.y - f.cam.y) * tile_px as f32).round() as i32;
                // Each chunk is its own image in the terminal, clipped to
                // its square: do the same here.
                let side = tiles * tile_px;
                let mut c = Pixmap::new(side, side).expect("chunk size");
                s.render(&mut c, Transform::identity());
                pm.draw_pixmap(ox, oy, c.as_ref(), &PixmapPaint::default(), Transform::identity(), None);
            }
        }
    };
    chunk(&mut pm, ArtLayer::Ground);
    let look = f.look.unwrap_or_else(|| Costume::default().look(0));
    let mut sheets = BTreeMap::new();
    let mut order: Vec<(i64, usize)> = f
        .sprites
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let foot = ((s.pos.y - f.cam.y) * tile_px as f32).round() as i64;
            let z = match s.layer {
                Layer::Decal => -1_000_000,
                Layer::Body => foot * 16,
                Layer::Cast => 1_000_000 + foot * 16,
                Layer::Over => 2_000_000 + foot * 16,
            } + s.z_bias as i64;
            (z, i)
        })
        .collect();
    order.sort();
    let draw = |pm: &mut Pixmap, sheets: &mut BTreeMap<SheetId, crate::art::Painted>, i: usize| {
        let s = &f.sprites[i];
        let sheet = sheets.entry(s.sheet).or_insert_with(|| paint_sheet(s.sheet, tile_px, &look, false));
        let Some(frames) = sheet.spec.anims.get(&s.anim) else { return };
        if frames.is_empty() {
            return;
        }
        let Some(px) = &sheet.pixels else { return };
        let frame = frames[s.frame % frames.len()];
        let cols = sheet.spec.cols.max(1);
        let (sx, sy) = ((frame % cols) * sheet.spec.cell_w, (frame / cols) * sheet.spec.cell_h);
        let Some(cell) = px.clone_rect(tiny_skia::IntRect::from_xywh(sx as i32, sy as i32, sheet.spec.cell_w, sheet.spec.cell_h).expect("frame rect")) else { return };
        let x = ((s.pos.x - f.cam.x) * tile_px as f32 - sheet.spec.anchor_x).round() as i32;
        let y = ((s.pos.y - f.cam.y) * tile_px as f32 - sheet.spec.anchor_y).round() as i32;
        pm.draw_pixmap(x, y, cell.as_ref(), &PixmapPaint::default(), Transform::identity(), None);
    };
    for &(_, i) in order.iter().filter(|(_, i)| matches!(f.sprites[*i].layer, Layer::Decal | Layer::Body)) {
        draw(&mut pm, &mut sheets, i);
    }
    chunk(&mut pm, ArtLayer::Canopy);
    for &(_, i) in order.iter().filter(|(_, i)| matches!(f.sprites[*i].layer, Layer::Cast | Layer::Over)) {
        draw(&mut pm, &mut sheets, i);
    }
    let _ = slots;
    pm
}
