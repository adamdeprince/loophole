//! Renderer tests: decode the bytes it writes.

use super::*;
use crate::world::{plan, town};

fn town() -> TownRef {
    let t = Arc::new(plan::build(&town::parse(town::MONTPELIER).unwrap()));
    TownRef {
        key: "montpelier".into(),
        art: Arc::new(scenery::build(&t, false)),
        town: t,
    }
}

fn renderer(vector: bool) -> Renderer {
    let mut r = Renderer::new(RenderConfig {
        vector,
        uploader: Uploader {
            temp_file: false,
            gamut_p3: false,
        },
        sync_output: true,
        cell_alpha: vector,
        hdr: false,
    });
    r.set_layout(Layout::new(160, 48, 10, 20, 0));
    r
}

/// The graphics commands in some output: (control keys, payload).
fn commands(out: &[u8]) -> Vec<String> {
    let s = String::from_utf8_lossy(out);
    s.split("\x1b_G").skip(1).map(|c| c.split(';').next().unwrap_or("").to_string()).collect()
}

fn model(t: &TownRef, cam: Vec2, decals: Vec<u32>) -> FrameModel {
    FrameModel {
        town: Some(t.clone()),
        cam,
        decals,
        sprites: vec![Sprite {
            key: 7,
            sheet: SheetId::Stuff,
            anim: "egg".into(),
            frame: 0,
            pos: cam + Vec2::new(5.0, 5.0),
            layer: Layer::Body,
            z_bias: 0,
        }],
        top: "top".into(),
        bottom: "bottom".into(),
        ..Default::default()
    }
}

fn decals(t: &TownRef) -> Vec<u32> {
    let art = scenery::build(&t.town, false);
    vec![0; art.slots.len()]
}

#[test]
fn the_first_frame_sends_chunks_and_a_still_frame_sends_nothing() {
    for vector in [false, true] {
        let t = town();
        let mut r = renderer(vector);
        let d = decals(&t);
        let cam = Vec2::new(110.0, 120.0);
        let out = r.render(&model(&t, cam, d.clone()));
        let cmds = commands(&out);
        assert!(cmds.iter().any(|c| c.contains("a=t") && c.contains(if vector { "f=200" } else { "a=t" })));
        assert!(r.last.chunks_sent >= 2, "{} chunks", r.last.chunks_sent);
        // The ring around the view arrives over the next frames.
        for _ in 0..40 {
            r.render(&model(&t, cam, d.clone()));
        }
        let out = r.render(&model(&t, cam, d.clone()));
        assert!(out.is_empty(), "{} bytes for nothing", out.len());
    }
}

#[test]
fn scrolling_moves_placements_without_uploads() {
    let t = town();
    let mut r = renderer(true);
    let d = decals(&t);
    let cam = Vec2::new(110.0, 120.0);
    for _ in 0..40 {
        r.render(&model(&t, cam, d.clone()));
    }
    let out = r.render(&model(&t, cam + Vec2::new(0.2, 0.1), d.clone()));
    let cmds = commands(&out);
    assert!(!cmds.is_empty());
    assert!(cmds.iter().all(|c| c.contains("a=p")), "{cmds:?}");
}

#[test]
fn a_porch_light_going_out_is_a_patch_on_silkie_and_a_chunk_on_kitty() {
    let t = town();
    let art = scenery::build(&t.town, false);
    let cam = Vec2::new(110.0, 120.0);
    let view = Layout::new(160, 48, 10, 20, 0).unwrap().view();
    let (i, _) = art
        .slots
        .iter()
        .enumerate()
        .find(|(_, s)| matches!(s.kind, DecalKind::PorchLight { .. }) && s.at.x > cam.x + 2.0 && s.at.x < cam.x + view.x - 2.0 && s.at.y > cam.y + 2.0 && s.at.y < cam.y + view.y - 2.0)
        .expect("a porch in view");
    for vector in [true, false] {
        let mut r = renderer(vector);
        let mut d = vec![1u32; art.slots.len()];
        for _ in 0..40 {
            r.render(&model(&t, cam, d.clone()));
        }
        d[i] = 0;
        let out = r.render(&model(&t, cam, d.clone()));
        let cmds = commands(&out);
        if vector {
            assert!(cmds.iter().any(|c| c.starts_with("a=e")), "{cmds:?}");
            assert!(!cmds.iter().any(|c| c.starts_with("a=t")));
            assert!(out.len() < 3000, "{} bytes", out.len());
        } else {
            assert!(cmds.iter().any(|c| c.starts_with("a=t")));
        }
    }
}

#[test]
fn chunks_far_behind_are_let_go() {
    let t = town();
    let mut r = renderer(true);
    let d = decals(&t);
    for x in 0..60 {
        r.render(&model(&t, Vec2::new(10.0 + x as f32 * 3.0, 120.0), d.clone()));
    }
    let loaded = r.chunks_loaded();
    let l = Layout::new(160, 48, 10, 20, 0).unwrap();
    let tiles = chunk_tiles(l.tile_px) as f32;
    let span = ((l.view().x / tiles).ceil() as usize + 5) * ((l.view().y / tiles).ceil() as usize + 5) * 2;
    assert!(loaded <= span, "{loaded} chunks held, at most {span} expected");
}

#[test]
fn every_sheet_paints_both_ways() {
    let look = crate::sim::costume::Costume::default().look(1);
    for id in SheetId::all() {
        for v in [false, true] {
            let p = paint_sheet(id, 40, &look, v);
            assert!(p.width > 0 && p.height > 0, "{id:?}");
            assert!(p.width <= crate::art::MAX_IMAGE_SIDE && p.height <= crate::art::MAX_IMAGE_SIDE, "{id:?}");
        }
    }
}

#[test]
fn pixels_held_stay_within_kittys_quota() {
    let t = town();
    let mut r = Renderer::new(RenderConfig {
        vector: false,
        uploader: Uploader { temp_file: false, gamut_p3: false },
        sync_output: true,
        cell_alpha: false,
        hdr: false,
    });
    // A big high-density window: 3000 by 1750 pixels, 119 pixels a tile.
    r.set_layout(Layout::new(214, 72, 14, 25, 0));
    let d = decals(&t);
    for x in 0..40 {
        let mut m = model(&t, Vec2::new(60.0 + x as f32 * 2.0, 110.0), d.clone());
        m.sprites[0].sheet = SheetId::Folk((x % 30) as u8);
        m.sprites[0].anim = "walk_down".into();
        r.render(&m);
        assert!(r.raster_bytes() <= RASTER_BUDGET + 40 * 1024 * 1024, "{} MB held", r.raster_bytes() / (1024 * 1024));
    }
    // Standing still, nothing is sent over and over.
    let cam = Vec2::new(140.0, 110.0);
    // The ring around the view fills in, a chunk a frame.
    for _ in 0..120 {
        r.render(&model(&t, cam, d.clone()));
    }
    let before = r.totals.chunks_sent;
    for _ in 0..30 {
        r.render(&model(&t, cam, d.clone()));
    }
    assert_eq!(r.totals.chunks_sent, before, "chunks churned while standing still");
}

#[test]
fn flickering_costs_little() {
    let t = town();
    let mut r = renderer(true);
    let d = decals(&t);
    // Downtown, pumpkins and flags in view.
    let cam = Vec2::new(105.0, 120.0);
    let mut m = model(&t, cam, d.clone());
    for k in 0..60 {
        m.flicker = k / 6;
        r.render(&m);
    }
    let before = r.totals.vector_bytes;
    let frames = 120;
    for k in 0..frames {
        m.flicker = 10 + k / 12;
        r.render(&m);
    }
    let per_second = (r.totals.vector_bytes - before) as f64 / (frames as f64 / 60.0);
    assert!(per_second < 30_000.0, "{per_second:.0} bytes a second of flicker");
}

#[test]
fn a_new_size_deletes_the_old_sheets_first() {
    let t = town();
    let mut r = renderer(false);
    let d = decals(&t);
    let cam = Vec2::new(110.0, 120.0);
    r.render(&model(&t, cam, d.clone()));
    r.set_layout(Layout::new(160, 48, 10, 20, 2));
    let out = r.render(&model(&t, cam, d.clone()));
    let s = String::from_utf8_lossy(&out);
    let id = SheetId::Stuff.image_id();
    let del = s.find(&format!("a=d,d=I,i={id}")).expect("the old sheet is deleted");
    let send = s.find(&format!("i={id},")).filter(|&i| i > del);
    assert!(send.is_some(), "and sent again after");
}
