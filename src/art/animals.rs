//! Gravy (a very big brown dog whose coat hangs in cords), the black cat
//! from State Street, and the dogs people walk.

use super::SheetBuilder;
use super::paint::{self, Painter, mirror};
use crate::gfx::vector::{Affine, Color, PathData};
use rand::{RngExt, SeedableRng};
use rand_chacha::ChaCha8Rng;

const INK: Color = Color::hex(0x2a2224);
const CORD: Color = Color::hex(0x7a4a26);
const CORD_LIGHT: Color = Color::hex(0x9a6436);
const CORD_DARK: Color = Color::hex(0x5a3418);

/// A corded coat over an ellipse: dozens of hanging ropes of fur.
fn cords(p: &mut Painter, cx: f32, cy: f32, rx: f32, ry: f32, hang: f32, seed: u64) {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let base = PathData::ellipse(cx, cy, rx, ry);
    p.inked(&base, CORD_DARK, INK, 1.4);
    for layer in 0..3 {
        let mut d = PathData::new();
        let n = 26;
        for k in 0..n {
            let t = k as f32 / (n - 1) as f32;
            let x = cx - rx * 0.95 + t * rx * 1.9 + rng.random_range(-1.5..1.5);
            let top = cy - ry * (0.9 - layer as f32 * 0.35) * (1.0 - (2.0 * t - 1.0).powi(2)).sqrt();
            let len = hang + ry * (0.6 + layer as f32 * 0.25) + rng.random_range(-3.0..3.0);
            let sway = rng.random_range(-2.0..2.0);
            d.move_to(x, top).quad_to(x + sway, top + len * 0.5, x + sway * 1.6, top + len);
        }
        let c = [CORD_DARK, CORD, CORD_LIGHT][layer];
        p.stroke(&d, INK.with_alpha(160), 5.4);
        p.stroke(&d, c, 4.0);
    }
}

/// Gravy, side on, facing right.
fn gravy(p: &mut Painter, pose: u32, k: u32) {
    match pose {
        // Standing or walking: a hearthrug on legs.
        0 => {
            paint::shadow(p, 46.0, 8.0);
            let step = [0.0f32, 2.5, 0.0, -2.5][k as usize % 4];
            for (x, s) in [(-26.0f32, step), (-14.0, -step), (16.0, step), (28.0, -step)] {
                p.inked(&PathData::round_rect(x - 4.0 + s, -16.0, 8.0, 16.0, 3.0), CORD_DARK, INK, 1.0);
            }
            cords(p, 0.0, -34.0, 40.0, 20.0, 6.0, 11 + k as u64);
            head(p, 36.0, -40.0, false);
        }
        // Lying down across the way, breathing.
        1 => {
            paint::shadow(p, 52.0, 9.0);
            let breathe = k as f32 * 1.2;
            cords(p, 0.0, -16.0 - breathe * 0.5, 46.0, 14.0 + breathe, 2.0, 21);
            head(p, 44.0, -14.0, true);
        }
        // On his back being rubbed, legs going.
        2 => {
            paint::shadow(p, 50.0, 9.0);
            cords(p, 0.0, -18.0, 44.0, 16.0, 0.0, 31);
            p.fill(&PathData::ellipse(-4.0, -28.0, 22.0, 7.0), Color::hex(0xc89a72));
            let kick = if k == 0 { -8.0 } else { 6.0 };
            for (x, l) in [(-22.0f32, 0.0f32), (-10.0, kick), (12.0, 0.0), (24.0, -kick)] {
                let mut leg = PathData::new();
                leg.move_to(x, -30.0).line_to(x + l * 0.4, -46.0 + l.abs() * 0.3);
                p.stroke(&leg, INK, 8.0);
                p.stroke(&leg, CORD_DARK, 6.0);
            }
            head(p, 46.0, -14.0, true);
            // Tongue out, very happy.
            p.fill(&PathData::ellipse(54.0, -6.0, 3.0, 5.0), Color::hex(0xe0607a));
        }
        // Eating: head down.
        3 => {
            paint::shadow(p, 46.0, 8.0);
            for x in [-26.0f32, -14.0, 16.0, 28.0] {
                p.inked(&PathData::round_rect(x - 4.0, -16.0, 8.0, 16.0, 3.0), CORD_DARK, INK, 1.0);
            }
            cords(p, 0.0, -32.0, 40.0, 19.0, 6.0, 41);
            head(p, 40.0, -14.0 - k as f32 * 2.0, false);
        }
        // Barfing: hunched, then the splash.
        _ => {
            paint::shadow(p, 46.0, 8.0);
            p.push(Affine::scale(1.0, 1.0 + 0.06 * k as f32));
            for x in [-26.0f32, -14.0, 16.0, 28.0] {
                p.inked(&PathData::round_rect(x - 4.0, -16.0, 8.0, 16.0, 3.0), CORD_DARK, INK, 1.0);
            }
            cords(p, 0.0, -36.0, 38.0, 22.0, 5.0, 51);
            p.pop();
            head(p, 40.0, -18.0, false);
            if k == 1 {
                p.fill(&PathData::ellipse(54.0, -4.0, 8.0, 3.0), Color::hex(0xb8b058));
            }
        }
    }
}

fn head(p: &mut Painter, x: f32, y: f32, lying: bool) {
    // All you can see of his face: a nose, and the shine of an eye.
    cords(p, x, y, 13.0, 11.0, if lying { 2.0 } else { 7.0 }, 61);
    p.inked(&PathData::ellipse(x + 11.0, y + 3.0, 4.2, 3.4), Color::hex(0x1a1414), INK, 0.6);
    p.fill(&PathData::circle(x + 12.0, y + 2.0, 1.2), Color::hex(0x7a7070));
    p.fill(&PathData::circle(x + 4.0, y - 2.0, 1.4), Color::hex(0x221a18));
    p.fill(&PathData::circle(x + 4.4, y - 2.4, 0.5), Color::hex(0xfff0d0));
}

/// Gravy: `walk_{right|left}` (four), `lie_{d}` (two), `rubbed_{d}` (two),
/// `eat_{d}` (two), `barf_{d}` (two).
pub fn gravy_sheet(tile_px: u32, vector: bool) -> super::Painted {
    let mut sb = SheetBuilder::new("gravy", (144.0, 96.0), (72.0, 84.0), tile_px).vector(vector);
    for (dir, left) in [("right", false), ("left", true)] {
        let frames: [(&str, u32, u32); 5] = [("walk", 0, 4), ("lie", 1, 2), ("rubbed", 2, 2), ("eat", 3, 2), ("barf", 4, 2)];
        for (name, pose, n) in frames {
            for k in 0..n {
                sb.frame(&format!("{name}_{dir}"), move |p| {
                    if left {
                        p.push(mirror());
                    }
                    gravy(p, pose, k);
                    if left {
                        p.pop();
                    }
                });
            }
        }
    }
    sb.finish_any()
}

const FUR: Color = Color::hex(0x16141a);

/// The black cat, side on, facing right: 0 walking (k), 1 sitting (k: the
/// tail flicks), 2 arched and hissing.
fn cat(p: &mut Painter, pose: u32, k: u32) {
    paint::shadow(p, 10.0, 2.5);
    let eye = Color::hex(0xf2d84a);
    match pose {
        0 => {
            let step = [0.0f32, 2.0, 0.0, -2.0][k as usize % 4];
            for (x, s) in [(-7.0f32, step), (-4.0, -step), (5.0, step), (8.0, -step)] {
                let mut l = PathData::new();
                l.move_to(x, -8.0).line_to(x + s, 0.0);
                p.stroke(&l, FUR, 2.6);
            }
            p.fill(&PathData::ellipse(0.0, -10.0, 10.0, 4.5), FUR);
            let mut t = PathData::new();
            t.move_to(-9.0, -11.0).quad_to(-16.0, -14.0, -14.0, -22.0 + step);
            p.stroke(&t, FUR, 2.4);
            p.fill(&PathData::circle(10.0, -14.0, 4.2), FUR);
            p.fill(&PathData::polygon(&[(7.5, -16.5), (9.5, -21.0), (10.5, -16.5)]), FUR);
            p.fill(&PathData::polygon(&[(11.0, -16.5), (13.0, -21.0), (13.5, -16.0)]), FUR);
            p.fill(&PathData::ellipse(12.0, -14.5, 1.1, 0.9), eye);
        }
        1 => {
            p.fill(&PathData::ellipse(0.0, -8.0, 6.0, 8.0), FUR);
            let mut t = PathData::new();
            let flick = if k == 0 { 0.0 } else { 3.0 };
            t.move_to(-4.0, -1.0).quad_to(-12.0, 0.0, -11.0 + flick, -8.0);
            p.stroke(&t, FUR, 2.4);
            p.fill(&PathData::circle(2.0, -18.0, 4.4), FUR);
            p.fill(&PathData::polygon(&[(-1.0, -20.0), (0.0, -25.0), (2.0, -21.0)]), FUR);
            p.fill(&PathData::polygon(&[(3.0, -21.0), (5.0, -25.0), (5.8, -20.0)]), FUR);
            p.fill(&PathData::ellipse(0.5, -18.5, 1.1, 1.0), eye);
            p.fill(&PathData::ellipse(4.0, -18.5, 1.1, 1.0), eye);
        }
        _ => {
            let mut arch = PathData::new();
            arch.move_to(-9.0, 0.0).quad_to(-6.0, -20.0, 2.0, -18.0).quad_to(9.0, -16.0, 8.0, 0.0);
            p.stroke(&arch, FUR, 6.0);
            let mut t = PathData::new();
            t.move_to(-8.0, -10.0).line_to(-12.0, -24.0);
            p.stroke(&t, FUR, 3.6);
            p.fill(&PathData::circle(10.0, -12.0, 4.4), FUR);
            p.fill(&PathData::polygon(&[(8.0, -15.0), (9.0, -19.0), (11.0, -15.5)]), FUR);
            p.fill(&PathData::ellipse(12.0, -12.5, 1.2, 1.0), eye);
            p.fill(&PathData::ellipse(13.5, -9.5, 1.6, 1.0), Color::hex(0xf0a0b0));
        }
    }
}

/// A small dog on a lead: `0` walking (k), `1` barking.
fn dog(p: &mut Painter, pose: u32, k: u32) {
    paint::shadow(p, 12.0, 3.0);
    let coat = Color::hex(0xd8b078);
    let step = [0.0f32, 2.0][k as usize % 2];
    for (x, s) in [(-7.0f32, step), (-4.0, -step), (5.0, step), (8.0, -step)] {
        let mut l = PathData::new();
        l.move_to(x, -8.0).line_to(x + s, 0.0);
        p.stroke(&l, INK, 3.4);
        p.stroke(&l, coat, 2.2);
    }
    p.inked(&PathData::ellipse(0.0, -11.0, 11.0, 5.5), coat, INK, 0.8);
    p.inked(&PathData::circle(11.0, -16.0, 5.0), coat, INK, 0.8);
    p.fill(&PathData::ellipse(7.5, -17.0, 2.0, 3.6), Color::hex(0x8a5a3a));
    p.fill(&PathData::circle(15.5, -15.0, 1.2), INK);
    p.fill(&PathData::circle(12.0, -17.5, 0.9), INK);
    let mut tail = PathData::new();
    tail.move_to(-10.0, -13.0).quad_to(-15.0, -18.0 - step, -13.0, -21.0);
    p.stroke(&tail, coat, 2.0);
    if pose == 1 {
        p.fill(&PathData::ellipse(14.5, -12.0, 2.2, 1.4), Color::hex(0x8a2a38));
    }
}

/// The cat and the dogs: `cat_walk_{d}` (four), `cat_sit_{d}` (two),
/// `cat_hiss_{d}`, `dog_walk_{d}` (two), `dog_bark_{d}`.
pub fn pets_sheet(tile_px: u32, vector: bool) -> super::Painted {
    let mut sb = SheetBuilder::new("pets", (48.0, 40.0), (24.0, 34.0), tile_px).vector(vector);
    for (dir, left) in [("right", false), ("left", true)] {
        let frames: [(&str, u32, u32, bool); 5] = [("cat_walk", 0, 4, true), ("cat_sit", 1, 2, true), ("cat_hiss", 2, 1, true), ("dog_walk", 0, 2, false), ("dog_bark", 1, 1, false)];
        for (name, pose, n, is_cat) in frames {
            for k in 0..n {
                sb.frame(&format!("{name}_{dir}"), move |p| {
                    if left {
                        p.push(mirror());
                    }
                    if is_cat {
                        cat(p, pose, k);
                    } else {
                        dog(p, pose, k);
                    }
                    if left {
                        p.pop();
                    }
                });
            }
        }
    }
    sb.finish_any()
}
