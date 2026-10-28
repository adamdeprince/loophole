//! Treats lying about the yard, and the little effects that pop up over
//! everyone's heads: emote bubbles, feathers, dust, sparkles.

use super::SheetBuilder;
use super::paint::{Cloud, Painter, rot};
use crate::gfx::vector::{Affine, Color, PathData};
use tiny_skia::Pixmap;

const INK: Color = Color::hex(0x3d3530);

pub fn feather_cursor(px: u32) -> (Pixmap, (u32, u32)) {
    let s = px as f32 / 32.0;
    let mut p = Painter::new(px, px, s, 0.0, 0.0);
    let vane = PathData::smooth_closed(&[(7.0, 24.0), (12.0, 12.0), (24.0, 3.0), (29.0, 5.0), (22.0, 17.0), (11.0, 25.0)], 1.0);
    p.stroke(&vane, Color::rgb(40, 40, 40), 3.0);
    p.fill(&vane, Color::hex(0xfffdf7));
    let mut shaft = PathData::new();
    shaft.move_to(3.0, 29.0).quad_to(14.0, 17.0, 28.0, 4.0);
    p.stroke(&shaft, Color::hex(0x9b918a), 1.6);
    (p.pixmap().expect("a raster painter"), ((3.0 * s) as u32, (29.0 * s) as u32))
}

fn bubble(p: &mut Painter) {
    let mut d = PathData::round_rect(-13.0, -15.0, 26.0, 22.0, 9.0);
    d.extend(&PathData::polygon(&[(-8.0, 5.0), (-12.0, 13.0), (-1.0, 6.0)]));
    p.stroke(&d, INK, 3.0);
    p.fill(&d, Color::rgb(255, 255, 255));
}

fn bang(p: &mut Painter, x: f32, c: Color, s: f32) {
    p.fill(&PathData::round_rect(x - 2.2 * s, -12.0 * s - 3.0, 4.4 * s, 10.0 * s, 2.0 * s), c);
    p.fill(&PathData::circle(x, 2.0 * s - 2.5, 2.3 * s), c);
}

fn heart_path(cx: f32, cy: f32, s: f32) -> PathData {
    let mut h = PathData::new();
    h.move_to(cx, cy + 7.0 * s)
        .cubic_to(cx - 11.0 * s, cy - 1.0 * s, cx - 7.0 * s, cy - 10.0 * s, cx, cy - 5.0 * s)
        .cubic_to(cx + 7.0 * s, cy - 10.0 * s, cx + 11.0 * s, cy - 1.0 * s, cx, cy + 7.0 * s)
        .close();
    h
}

fn star_path(cx: f32, cy: f32, points: usize, outer: f32, inner: f32, phase: f32) -> PathData {
    let pts: Vec<(f32, f32)> = (0..points * 2)
        .map(|i| {
            let a = phase + i as f32 * std::f32::consts::PI / points as f32;
            let r = if i % 2 == 0 { outer } else { inner };
            (cx + a.sin() * r, cy - a.cos() * r)
        })
        .collect();
    PathData::polygon(&pts)
}

fn sparkle_shape(p: &mut Painter, x: f32, y: f32, r: f32, c: Color) {
    let s = star_path(x, y, 4, r, r * 0.28, 0.0);
    p.fill(&s, c);
}

pub fn fx_sheet(tile_px: u32, vector: bool) -> super::Painted {
    let mut sb = SheetBuilder::new("fx", (40.0, 40.0), (20.0, 20.0), tile_px).vector(vector);
    sb.frame("emote_alert", |p| {
        bubble(p);
        bang(p, 0.0, Color::hex(0xe53935), 1.0);
    });
    sb.frame("emote_puzzled", |p| {
        bubble(p);
        let mut q = PathData::new();
        q.move_to(-4.5, -8.0).quad_to(-4.5, -12.0, 0.0, -12.0).quad_to(4.5, -12.0, 4.5, -8.0).quad_to(4.5, -5.0, 0.0, -3.5).line_to(0.0, -1.5);
        p.stroke(&q, Color::hex(0x3b6fd6), 3.2);
        p.fill(&PathData::circle(0.0, 2.5, 2.0), Color::hex(0x3b6fd6));
    });
    sb.frame("emote_heart", |p| {
        bubble(p);
        p.fill(&heart_path(0.0, -4.0, 0.9), Color::hex(0xff4f86));
    });
    sb.frame("emote_fright", |p| {
        bubble(p);
        bang(p, -4.0, Color::hex(0xff8f1f), 0.85);
        bang(p, 4.0, Color::hex(0xff8f1f), 0.85);
    });
    sb.frame("emote_note", |p| {
        bubble(p);
        let c = Color::hex(0x9b59d0);
        p.push(Affine::translate(-1.5, -4.0));
        p.fill(&PathData::ellipse(-1.5, 4.0, 3.6, 2.7), c);
        let mut stem = PathData::new();
        stem.move_to(1.8, 3.5).line_to(1.8, -8.0).quad_to(5.5, -6.0, 6.5, -2.5);
        p.stroke(&stem, c, 1.8);
        p.pop();
    });
    sb.frame("emote_sweat", |p| {
        let mut d = PathData::new();
        d.move_to(0.0, -10.0).quad_to(7.0, -1.0, 0.0, 5.0).quad_to(-7.0, -1.0, 0.0, -10.0).close();
        p.inked(&d, Color::hex(0x8fd0ff), Color::hex(0x2a6d9c), 1.0);
        p.fill(&PathData::ellipse(-2.0, -1.0, 1.2, 2.2), Color::rgba(255, 255, 255, 220));
    });
    sb.frame("emote_angry", |p| {
        let c = Color::hex(0xe53935);
        let mut v = PathData::new();
        for (sx, sy) in [(-1.0f32, -1.0f32), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)] {
            v.move_to(sx * 2.0, sy * 9.0).quad_to(sx * 2.5, sy * 2.5, sx * 9.0, sy * 2.0);
        }
        p.stroke(&v, Color::rgb(255, 255, 255), 5.5);
        p.stroke(&v, c, 3.0);
    });
    for (i, a) in [-30.0f32, 15.0, 70.0].into_iter().enumerate() {
        sb.frame("feather", |p| {
            p.push(rot(a));
            let vane = PathData::smooth_closed(&[(-10.0, 2.0), (-2.0, -4.5), (10.0, -3.0), (4.0, 3.5), (-6.0, 4.5)], 1.0);
            p.inked(&vane, Color::hex(0xfffdf7), Color::hex(0x9b918a), 0.8);
            let mut shaft = PathData::new();
            shaft.move_to(-12.0, 4.0).quad_to(0.0, 0.0, 10.0, -3.0);
            p.stroke(&shaft, Color::hex(0xbcb2a6), 1.0);
            p.pop();
            let _ = i;
        });
    }
    for r in [5.0f32, 8.0, 11.0] {
        sb.frame("dust", |p| {
            let c = Cloud::new(0.0, 0.0, r, r * 0.75, 6, 0.42, (r * 10.0) as u64);
            p.fill(&c.path(0.0), Color::rgba(232, 214, 176, 210));
        });
    }
    for r in [10.0f32, 6.0] {
        sb.frame("sparkle", |p| {
            sparkle_shape(p, 0.0, 0.0, r, Color::hex(0xfff7c2));
            sparkle_shape(p, 0.0, 0.0, r * 0.55, Color::rgb(255, 255, 255));
        });
    }
    sb.frame("star", |p| {
        let s = star_path(0.0, 0.0, 5, 9.0, 4.0, 0.0);
        p.inked(&s, Color::hex(0xffd23f), Color::hex(0x9c6d00), 1.0);
        // A white-hot glint at the heart of it (the part that glows in HDR).
        sparkle_shape(p, 0.0, 0.5, 3.4, Color::rgb(255, 255, 255));
    });
    // A splash: a crown of water, spreading.
    for k in 0..3 {
        sb.frame("splash", |p| {
            let r = 6.0 + k as f32 * 4.5;
            let a = 255 - k as u8 * 60;
            p.stroke(&PathData::ellipse(0.0, 6.0, r * 1.4, r * 0.45), Color::rgba(235, 248, 255, a), 2.2);
            for i in 0..5 {
                let x = -12.0 + i as f32 * 6.0;
                let h = [8.0f32, 13.0, 16.0, 13.0, 8.0][i] * (1.0 - k as f32 * 0.25);
                let mut d = PathData::new();
                d.move_to(x * (1.0 + k as f32 * 0.3), 6.0).quad_to(x * 1.2, 6.0 - h, x * (1.3 + k as f32 * 0.3), 4.0 - h);
                p.stroke(&d, Color::hex(0x2a6d9c).with_alpha(a), 3.2);
                p.stroke(&d, Color::hex(0x8fd0ff).with_alpha(a), 2.0);
            }
            for (x, y) in [(-16.0f32, -8.0f32), (15.0, -10.0), (2.0, -14.0)] {
                p.fill(&PathData::circle(x * (1.0 + k as f32 * 0.4), y + k as f32 * 3.0, 2.0), Color::hex(0x8fd0ff).with_alpha(a));
            }
        });
    }
    // A bug that met a tyre: a cartoon splat, with a little halo.
    for k in 0..2 {
        sb.frame("splat", |p| {
            let mut d = PathData::new();
            let n = 9;
            for i in 0..n * 2 {
                let a = i as f32 * std::f32::consts::PI / n as f32;
                let r = if i % 2 == 0 { 9.0 } else { 5.0 } * (1.0 + k as f32 * 0.15);
                let (x, y) = (a.cos() * r, 4.0 + a.sin() * r * 0.45);
                if i == 0 {
                    d.move_to(x, y);
                } else {
                    d.line_to(x, y);
                }
            }
            d.close();
            p.inked(&d, Color::hex(0xc0ca33), Color::hex(0x827717), 0.8);
            p.stroke(&PathData::ellipse(0.0, -6.0 - k as f32 * 3.0, 6.0, 2.0), Color::hex(0xffe082), 1.4);
        });
    }
    // A puff of exhaust or dust, for a hen who has had a near miss.
    for k in 0..2 {
        sb.frame("puff", |p| {
            let c = Cloud::new(0.0, 0.0, 9.0 + k as f32 * 3.0, 6.0 + k as f32 * 2.0, 6, 0.42, 77 + k as u64);
            p.fill(&c.path(0.0), Color::rgba(250, 250, 250, 200 - k as u8 * 70));
        });
    }
    sb.frame("reject", |p| {
        let mut x = PathData::new();
        x.move_to(-7.0, -7.0).line_to(7.0, 7.0).move_to(7.0, -7.0).line_to(-7.0, 7.0);
        p.stroke(&x, Color::rgb(255, 255, 255), 7.0);
        p.stroke(&x, Color::hex(0xe53935), 4.0);
    });
    sb.frame("check", |p| {
        let mut c = PathData::new();
        c.move_to(-7.0, 0.0).line_to(-2.0, 6.0).line_to(8.0, -7.0);
        p.stroke(&c, Color::rgb(255, 255, 255), 7.0);
        p.stroke(&c, Color::hex(0x43a047), 4.0);
    });
    sb.frame("heart", |p| {
        let h = heart_path(0.0, 0.0, 1.0);
        p.inked(&h, Color::hex(0xff4f86), Color::rgb(255, 255, 255), 1.4);
        // A glint on the left lobe.
        p.fill(&PathData::ellipse(-3.6, -4.2, 1.5, 1.1), Color::rgb(255, 255, 255));
    });
    sb.finish_any()
}
