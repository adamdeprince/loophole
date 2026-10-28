//! The raccoon in its bedsheet: a white drape with two eye holes (and a
//! raccoon's masked eyes looking out of them), little black paws under the
//! hem, and a ringed tail that will not stay hidden.
//!
//! The sheet is painted with its current state: the tears and holes (fur
//! showing through), the patches and what they are made of, mud, wet (the
//! sheet goes see-through), leaves stuck on, and the belly pushing the hem
//! up. The renderer repaints it when that state changes.
//!
//! The pillowcase of candy is its own small sheet, in sizes.

use super::SheetBuilder;
use super::paint::{self, Painter, mirror, rot_about};
use crate::gfx::vector::{Affine, Color, PathData};
use crate::sim::candy::Mat;
use crate::sim::costume::{COLS, Look, ROWS};
use rand::{RngExt, SeedableRng};
use rand_chacha::ChaCha8Rng;

const INK: Color = Color::hex(0x2a2630);
const SHEET: Color = Color::hex(0xf4f1ea);
const SHEET_SHADE: Color = Color::hex(0xc9c6d2);
const FUR: Color = Color::hex(0x6f6a6e);
const FUR_DARK: Color = Color::hex(0x2c2a30);
const FUR_LIGHT: Color = Color::hex(0xb8b2b0);
const MUD: Color = Color::hex(0x5a4430);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum View {
    Front,
    Back,
    Side,
}

#[derive(Clone, Copy, Debug)]
struct Pose {
    view: View,
    /// Stride phase, -1 to 1.
    step: f32,
    bob: f32,
    /// Lean forward (running), degrees.
    lean: f32,
    /// Raise a paw out of the sheet (knocking, waving, reaching).
    paw: Option<(f32, f32)>,
    /// Hem flapping out behind (running).
    flap: f32,
    sway: f32,
    eyes_shut: bool,
}

impl Pose {
    fn new(view: View) -> Pose {
        Pose {
            view,
            step: 0.0,
            bob: 0.0,
            lean: 0.0,
            paw: None,
            flap: 0.0,
            sway: 0.0,
            eyes_shut: false,
        }
    }
}

/// Height of the head's top above the ground, and the hem's half width.
const TOP: f32 = -74.0;
const HEM_W: f32 = 25.0;

/// How far the belly pushes the hem up (art units).
fn hem_lift(look: &Look) -> f32 {
    look.belly as f32 * 2.6
}

fn hem_y(look: &Look) -> f32 {
    -7.0 - hem_lift(look)
}

/// The sheet's silhouette for a view.
fn drape(look: &Look, s: &Pose) -> PathData {
    let belly = look.belly as f32;
    let hem = hem_y(look) + s.bob;
    let top = TOP + s.bob;
    let w = HEM_W + belly * 1.6;
    let swing = s.sway * 3.0;
    let flap = s.flap;
    let mut pts: Vec<(f32, f32)> = Vec::new();
    match s.view {
        View::Front | View::Back => {
            pts.push((0.0, top));
            pts.push((13.0, top + 4.0));
            pts.push((16.5, top + 20.0));
            pts.push((18.0 + belly * 2.2, top + 40.0));
            pts.push((w + swing, hem - 2.0));
            // A scalloped hem.
            let n = 5;
            for i in 0..=n {
                let t = i as f32 / n as f32;
                let x = w + swing - t * 2.0 * w;
                let y = hem + if i % 2 == 0 { 3.0 } else { -1.5 };
                pts.push((x, y));
            }
            pts.push((-18.0 - belly * 2.2, top + 40.0));
            pts.push((-16.5, top + 20.0));
            pts.push((-13.0, top + 4.0));
        }
        View::Side => {
            // Facing right; the hem streams back when running.
            pts.push((4.0, top));
            pts.push((16.0, top + 7.0));
            pts.push((18.0, top + 22.0));
            pts.push((19.0 + belly * 2.4, top + 40.0));
            pts.push((w - 4.0 + swing, hem - 1.0));
            let n = 4;
            for i in 0..=n {
                let t = i as f32 / n as f32;
                let x = w - 4.0 + swing - t * (2.0 * w - 4.0 + flap * 10.0);
                let y = hem + if i % 2 == 0 { 3.0 } else { -1.5 } - t * flap * 6.0;
                pts.push((x, y));
            }
            pts.push((-17.0 - flap * 6.0, top + 36.0));
            pts.push((-13.0, top + 14.0));
            pts.push((-6.0, top + 2.0));
        }
    }
    PathData::smooth_closed(&pts, 0.85)
}

fn tail(p: &mut Painter, s: &Pose, look: &Look) {
    let hem = hem_y(look) + s.bob;
    let (x0, y0, x1, y1, x2, y2) = match s.view {
        View::Side => (-16.0, hem - 2.0, -34.0, hem - 2.0 + s.step * 2.0, -44.0, hem - 14.0),
        View::Back => (0.0, hem - 1.0, 12.0 + s.sway * 6.0, hem + 2.0, 20.0 + s.sway * 8.0, hem - 16.0),
        View::Front => (-12.0, hem - 1.0, -26.0, hem + 1.0, -32.0 - s.sway * 4.0, hem - 13.0),
    };
    let mut d = PathData::new();
    d.move_to(x0, y0).quad_to(x1, y1, x2, y2);
    p.stroke(&d, INK, 12.0);
    p.stroke(&d, FUR, 10.0);
    // Rings along it.
    for i in 1..6 {
        let t = i as f32 / 6.0;
        let (ax, ay) = quad_at((x0, y0), (x1, y1), (x2, y2), t);
        if i % 2 == 1 {
            p.fill(&PathData::circle(ax, ay, 4.7), FUR_DARK);
        }
    }
    p.fill(&PathData::circle(x2, y2, 4.6), FUR_DARK);
}

fn quad_at(a: (f32, f32), b: (f32, f32), c: (f32, f32), t: f32) -> (f32, f32) {
    let u = 1.0 - t;
    (u * u * a.0 + 2.0 * u * t * b.0 + t * t * c.0, u * u * a.1 + 2.0 * u * t * b.1 + t * t * c.1)
}

fn feet(p: &mut Painter, s: &Pose, look: &Look) {
    let hem = hem_y(look) + s.bob;
    let lift = hem_lift(look);
    // Legs show more as the belly hitches the sheet up.
    let leg = (lift + 3.0).max(3.0);
    let xs: [(f32, f32); 2] = match s.view {
        View::Side => [(-4.0 - s.step * 7.0, 0.0), (6.0 + s.step * 7.0, 0.0)],
        _ => [(-8.0, s.step.max(0.0) * 3.0), (8.0, (-s.step).max(0.0) * 3.0)],
    };
    for (x, up) in xs {
        if leg > 4.0 {
            p.fill(&PathData::rect(x - 3.5, hem - 2.0, 7.0, leg + 2.0 - up), FUR_DARK);
        }
        p.inked(&PathData::ellipse(x + if s.view == View::Side { 2.0 } else { 0.0 }, -2.5 - up, 5.0, 3.0), FUR_DARK, INK, 0.8);
    }
}

/// The two eye holes, with a raccoon's mask and eyes in them.
fn eyes(p: &mut Painter, s: &Pose) {
    let y = TOP + 15.0 + s.bob;
    let xs: &[f32] = match s.view {
        View::Front => &[-6.5, 6.5],
        View::Side => &[11.0],
        View::Back => &[],
    };
    for &x in xs {
        let hole = PathData::ellipse(x, y, 5.2, 4.6);
        p.fill(&hole, INK);
        p.fill(&PathData::ellipse(x, y + 0.5, 4.4, 3.8), FUR_DARK);
        if s.eyes_shut {
            let mut lid = PathData::new();
            lid.move_to(x - 2.6, y).quad_to(x, y + 1.8, x + 2.6, y);
            p.stroke(&lid, FUR_LIGHT, 1.2);
        } else {
            p.fill(&PathData::circle(x, y + 0.4, 2.3), Color::hex(0x141218));
            // A little eyeshine.
            p.fill(&PathData::circle(x + 0.9, y - 0.5, 0.95), Color::hex(0xfff6d0));
        }
    }
}

/// Where slot (c, r) of the sheet is drawn, in this view.
fn slot_at(c: usize, r: usize, look: &Look, s: &Pose) -> (f32, f32) {
    let hem = hem_y(look) + s.bob;
    let top = TOP + 26.0 + s.bob;
    let w = match s.view {
        View::Side => 30.0,
        _ => 30.0 + look.belly as f32 * 2.0,
    };
    let x = -w / 2.0 + (c as f32 + 0.5) * w / COLS as f32 + if s.view == View::Side { 2.0 } else { 0.0 };
    let y = top + (r as f32 + 0.5) * (hem - 4.0 - top) / ROWS as f32;
    (x, y)
}

fn damage(p: &mut Painter, look: &Look, s: &Pose) {
    for (i, &st) in look.slots.iter().enumerate() {
        if st == 0 {
            continue;
        }
        let (c, r) = (i % COLS, i / COLS);
        let (x, y) = slot_at(c, r, look, s);
        let mut rng = ChaCha8Rng::seed_from_u64(i as u64 * 31 + 7);
        let rot: f32 = rng.random_range(-25.0..25.0);
        p.push(rot_about(rot, x, y));
        match st {
            1 => {
                // A snag: a little rip, a little fur.
                let mut d = PathData::new();
                d.move_to(x - 3.5, y - 1.0).line_to(x - 1.0, y + 1.5).line_to(x + 1.0, y - 1.5).line_to(x + 3.5, y + 1.0);
                p.stroke(&d, FUR, 2.6);
                p.stroke(&d, FUR_DARK, 0.9);
            }
            2 => {
                // A hole: fur shows through, ragged edges.
                let mut pts = Vec::new();
                for k in 0..9 {
                    let a = k as f32 / 9.0 * std::f32::consts::TAU;
                    let rr = rng.random_range(3.6..5.6);
                    pts.push((x + a.cos() * rr, y + a.sin() * rr * 0.8));
                }
                let hole = PathData::polygon(&pts);
                p.fill(&hole, FUR);
                p.stroke(&hole, SHEET_SHADE, 0.9);
                let mut fur = PathData::new();
                for k in 0..4 {
                    let fx = x - 3.0 + k as f32 * 2.0;
                    fur.move_to(fx, y - 2.5).line_to(fx + 0.6, y + 2.5);
                }
                p.stroke(&fur, FUR_DARK, 0.7);
            }
            n => {
                let m = Mat::ALL.iter().copied().find(|m| *m as u8 == n - 3).unwrap_or(Mat::Paper);
                patch(p, m, x, y);
            }
        }
        p.pop();
    }
}

/// A patch over a hole, looking like what it is made of.
pub fn patch(p: &mut Painter, m: Mat, x: f32, y: f32) {
    let (r, g, b) = m.colour();
    let c = Color::rgb(r, g, b);
    match m {
        Mat::Leaf => {
            let mut pts = Vec::new();
            for k in 0..10 {
                let a = -std::f32::consts::FRAC_PI_2 + k as f32 / 10.0 * std::f32::consts::TAU;
                let rr = if k % 2 == 0 { 6.5 } else { 3.0 };
                pts.push((x + a.cos() * rr, y + a.sin() * rr));
            }
            p.inked(&PathData::polygon(&pts), c, INK, 0.5);
        }
        Mat::Gum => {
            p.fill(&PathData::smooth_closed(&[(x - 4.0, y - 2.0), (x + 1.0, y - 4.0), (x + 4.5, y), (x + 1.0, y + 3.5), (x - 3.5, y + 2.5)], 0.9), c);
            p.fill(&PathData::circle(x - 1.0, y - 1.5, 1.2), c.shade(0.4));
            let mut st = PathData::new();
            st.move_to(x + 3.0, y + 1.0).quad_to(x + 6.0, y + 4.0, x + 4.0, y + 7.0);
            p.stroke(&st, c.shade(0.2), 0.7);
        }
        Mat::Tape => {
            p.fill(&PathData::rect(x - 6.0, y - 2.5, 12.0, 5.0), c);
            p.fill(&PathData::rect(x - 2.5, y - 6.0, 5.0, 12.0), c.shade(-0.1));
        }
        Mat::Wrapper => {
            p.inked(&PathData::round_rect(x - 4.5, y - 2.8, 9.0, 5.6, 1.5), c, INK, 0.4);
            p.fill(&PathData::polygon(&[(x - 4.5, y), (x - 7.5, y - 2.5), (x - 7.5, y + 2.5)]), c.shade(0.2));
            p.fill(&PathData::polygon(&[(x + 4.5, y), (x + 7.5, y - 2.5), (x + 7.5, y + 2.5)]), c.shade(0.2));
            p.fill(&PathData::rect(x - 3.0, y - 2.0, 2.0, 1.0), Color::rgba(255, 255, 255, 170));
        }
        Mat::Sticker => {
            p.inked(&PathData::circle(x, y, 4.6), c, INK, 0.4);
            p.fill(&PathData::circle(x, y, 2.0), Color::hex(0xffe060));
        }
        Mat::Flyer => {
            p.inked(&PathData::rect(x - 5.0, y - 6.0, 10.0, 12.0), c, INK, 0.4);
            let mut lines = PathData::new();
            for k in 0..3 {
                lines.move_to(x - 3.5, y - 3.0 + k as f32 * 3.0).line_to(x + 3.5, y - 3.0 + k as f32 * 3.0);
            }
            p.stroke(&lines, INK.with_alpha(160), 0.8);
        }
        _ => {
            p.inked(&PathData::rect(x - 5.0, y - 4.0, 10.0, 8.0), c, INK.with_alpha(120), 0.4);
            let mut stitches = PathData::new();
            for k in 0..4 {
                let sx = x - 4.5 + k as f32 * 3.0;
                stitches.move_to(sx, y - 4.8).line_to(sx + 1.0, y - 3.2);
            }
            p.stroke(&stitches, INK.with_alpha(150), 0.6);
        }
    }
}

fn mud(p: &mut Painter, look: &Look, s: &Pose) {
    if look.dirt == 0 {
        return;
    }
    let hem = hem_y(look) + s.bob;
    let mut rng = ChaCha8Rng::seed_from_u64(99);
    let mut d = PathData::new();
    for _ in 0..(look.dirt as usize * 4) {
        let x = rng.random_range(-HEM_W..HEM_W) * 0.85;
        let y = hem - rng.random_range(1.0..14.0 * look.dirt as f32);
        d.extend(&PathData::ellipse(x, y, rng.random_range(2.0..4.5), rng.random_range(1.2..2.6)));
    }
    p.fill(&d, MUD.with_alpha(170));
}

fn stuck_leaves(p: &mut Painter, look: &Look, s: &Pose) {
    let mut rng = ChaCha8Rng::seed_from_u64(5150);
    for k in 0..look.leaves {
        let x = rng.random_range(-14.0..14.0);
        let y = TOP + s.bob + rng.random_range(22.0..58.0);
        let m = [Mat::Leaf, Mat::Leaf, Mat::Leaf][k as usize % 3];
        p.push(rot_about(rng.random_range(-60.0..60.0), x, y));
        p.push(Affine::translate(x, y).then(Affine::scale(1.0, 1.0)));
        p.pop();
        patch(p, m, x, y);
        p.pop();
    }
}

/// The raccoon, whole, in a pose, facing right (or front/back).
fn draw(p: &mut Painter, look: &Look, s: &Pose) {
    paint::shadow(p, 20.0 + look.belly as f32, 5.0);
    p.push(rot_about(s.lean, 0.0, 0.0));
    if s.view != View::Back {
        tail(p, s, look);
    }
    feet(p, s, look);
    let body = drape(look, s);
    let wet = look.wet > 0;
    if wet {
        // A wet sheet clings and shows the raccoon through it.
        p.inked(&body, Color::hex(0xb8bcc8), INK, 1.4);
        p.fill(&PathData::ellipse(0.0, TOP + 40.0 + s.bob, 15.0 + look.belly as f32 * 2.0, 26.0), FUR.with_alpha(110));
        p.fill(&PathData::ellipse(0.0, TOP + 16.0 + s.bob, 12.0, 10.0), FUR.with_alpha(90));
    } else {
        p.inked(&body, SHEET, INK, 1.4);
    }
    // Soft folds and the roundness of the belly.
    let mut folds = PathData::new();
    let hem = hem_y(look) + s.bob;
    for x in [-9.0f32, 0.0, 9.0] {
        folds.move_to(x * 0.6, TOP + 30.0 + s.bob).quad_to(x * 1.1, hem - 12.0, x * 1.3 + s.sway * 2.0, hem - 1.0);
    }
    p.stroke(&folds, SHEET_SHADE.with_alpha(if wet { 90 } else { 160 }), 1.2);
    if look.belly >= 2 {
        p.fill(&PathData::ellipse(0.0, TOP + 46.0 + s.bob, 12.0 + look.belly as f32 * 2.0, 10.0), SHEET_SHADE.with_alpha(70));
    }
    mud(p, look, s);
    damage(p, look, s);
    stuck_leaves(p, look, s);
    eyes(p, s);
    if s.view == View::Back {
        tail(p, s, look);
    }
    if let Some((px, py)) = s.paw {
        let mut arm = PathData::new();
        arm.move_to(8.0, TOP + 34.0 + s.bob).quad_to(px - 2.0, py + 6.0, px, py);
        p.stroke(&arm, INK, 7.0);
        p.stroke(&arm, if wet { Color::hex(0xb8bcc8) } else { SHEET }, 5.4);
        // The paw peeks out of the sheet's edge.
        p.inked(&PathData::ellipse(px, py, 3.6, 3.0), FUR_DARK, INK, 0.7);
    }
    p.pop();
}

/// The raccoon flat on the ground, sheet everywhere, seeing stars.
fn fallen(p: &mut Painter, look: &Look, k: u32) {
    paint::shadow(p, 26.0, 6.0);
    let mut s = Pose::new(View::Front);
    s.eyes_shut = true;
    p.push(Affine::translate(0.0, 2.0).then(Affine::scale(1.25, 0.55)).then(Affine::translate(0.0, -4.0)));
    p.push(rot_about(-80.0 + k as f32 * 6.0, 0.0, -20.0));
    draw(p, look, &s);
    p.pop();
    p.pop();
}

/// Swimming: just the sheet floating, eyes above the water.
fn swimming(p: &mut Painter, look: &Look, k: u32) {
    let mut ring = PathData::new();
    ring.move_to(-26.0, -2.0).quad_to(0.0, 4.0 + k as f32 * 2.0, 26.0, -2.0);
    p.stroke(&ring, Color::rgba(200, 220, 255, 140), 2.0);
    let mut s = Pose::new(View::Front);
    s.bob = 30.0 + k as f32 * 1.5;
    let mut wet = *look;
    wet.wet = 2;
    p.push(Affine::translate(0.0, 0.0));
    draw(p, &wet, &s);
    p.pop();
    p.fill(&PathData::ellipse(0.0, 2.0, 30.0, 6.0), Color::rgba(40, 70, 120, 200));
}

/// Every pose of the raccoon for one state of the costume.
///
/// `walk_{d}`, `run_{d}` (four each), `idle_{d}` (two) for d in down, up,
/// left, right; `knock`, `eat`, `patch`, `dance`, `take` (two each);
/// `fallen` (two), `swim` (two).
pub fn sheet(tile_px: u32, look: &Look, vector: bool) -> super::Painted {
    let mut sb = SheetBuilder::new("raccoon", (120.0, 104.0), (60.0, 96.0), tile_px).vector(vector);
    let scale = 0.92;
    let views = [("down", View::Front, false), ("up", View::Back, false), ("right", View::Side, false), ("left", View::Side, true)];
    let frame = |sb: &mut SheetBuilder, name: &str, s: Pose, left: bool| {
        let look = *look;
        sb.frame(name, move |p| {
            p.push(paint::scale_about(scale, scale, 0.0, 0.0));
            if left {
                p.push(mirror());
            }
            draw(p, &look, &s);
            if left {
                p.pop();
            }
            p.pop();
        });
    };
    for (name, view, left) in views {
        for k in 0..4 {
            let ph = k as f32 * std::f32::consts::FRAC_PI_2;
            let mut s = Pose::new(view);
            s.step = ph.sin();
            s.bob = -ph.sin().abs() * 1.6;
            s.sway = ph.cos() * 0.5;
            frame(&mut sb, &format!("walk_{name}"), s, left);
        }
        for k in 0..4 {
            let ph = k as f32 * std::f32::consts::FRAC_PI_2;
            let mut s = Pose::new(view);
            s.step = ph.sin() * 1.4;
            s.bob = -ph.sin().abs() * 3.0;
            s.sway = ph.cos();
            if view == View::Side {
                s.lean = 8.0;
                s.flap = 0.6 + ph.cos() * 0.3;
            }
            frame(&mut sb, &format!("run_{name}"), s, left);
        }
        for k in 0..2 {
            let mut s = Pose::new(view);
            s.sway = if k == 0 { -0.4 } else { 0.4 };
            s.bob = k as f32 * -0.6;
            frame(&mut sb, &format!("idle_{name}"), s, left);
        }
    }
    for k in 0..2 {
        let mut s = Pose::new(View::Back);
        s.paw = Some((10.0, TOP + 16.0 - k as f32 * 6.0));
        frame(&mut sb, "knock", s, false);
    }
    for k in 0..2 {
        let mut s = Pose::new(View::Front);
        s.paw = Some((3.0, TOP + 22.0 + k as f32 * 3.0));
        s.eyes_shut = k == 1;
        frame(&mut sb, "eat", s, false);
    }
    for k in 0..2 {
        let mut s = Pose::new(View::Front);
        s.paw = Some((10.0 - k as f32 * 8.0, TOP + 44.0));
        frame(&mut sb, "patch", s, false);
    }
    for k in 0..2 {
        let mut s = Pose::new(View::Front);
        s.paw = Some((18.0, TOP + 4.0 - k as f32 * 4.0));
        s.bob = -4.0 * k as f32;
        s.sway = if k == 0 { -1.0 } else { 1.0 };
        s.eyes_shut = true;
        frame(&mut sb, "dance", s, k == 1);
    }
    for k in 0..2 {
        let mut s = Pose::new(View::Front);
        s.paw = Some((14.0, TOP - 2.0 - k as f32 * 4.0));
        frame(&mut sb, "take", s, false);
    }
    for k in 0..2 {
        let look = *look;
        sb.frame("fallen", move |p| {
            p.push(paint::scale_about(scale, scale, 0.0, 0.0));
            fallen(p, &look, k);
            p.pop();
        });
    }
    for k in 0..2 {
        let look = *look;
        sb.frame("swim", move |p| {
            p.push(paint::scale_about(scale, scale, 0.0, 0.0));
            swimming(p, &look, k);
            p.pop();
        });
    }
    sb.finish_any()
}

/// The pillowcase, in sizes 0 (a few pieces) to 7 (bigger than the
/// raccoon). Frames `bag` 0..8, drawn with the opening at the top.
pub fn bag_sheet(tile_px: u32, vector: bool) -> super::Painted {
    let mut sb = SheetBuilder::new("bag", (96.0, 112.0), (48.0, 100.0), tile_px).vector(vector);
    for k in 0..8 {
        sb.frame("bag", move |p| {
            let s = 0.45 + k as f32 * 0.17;
            let w = 15.0 * s;
            let h = 26.0 * s;
            paint::shadow(p, w * 1.1, 3.0 + s);
            let body = PathData::smooth_closed(
                &[(-w * 0.6, -h * 1.05), (w * 0.6, -h * 1.05), (w * 1.05, -h * 0.5), (w * 0.95, -2.0), (0.0, 0.0), (-w * 0.95, -2.0), (-w * 1.05, -h * 0.5)],
                0.8,
            );
            p.inked(&body, Color::hex(0xece6f2), INK, 1.2);
            // A printed pumpkin.
            p.fill(&PathData::ellipse(0.0, -h * 0.45, w * 0.45, h * 0.28), Color::hex(0xe07a28));
            p.fill(&PathData::polygon(&[(-w * 0.2, -h * 0.5), (-w * 0.05, -h * 0.5), (-w * 0.12, -h * 0.6)]), INK);
            p.fill(&PathData::polygon(&[(w * 0.05, -h * 0.5), (w * 0.2, -h * 0.5), (w * 0.12, -h * 0.6)]), INK);
            // Candy poking out of the top.
            let tops = [Color::hex(0xe8402a), Color::hex(0xf2c230), Color::hex(0x7a3ad0), Color::hex(0x2ab060)];
            for i in 0..(k + 1).min(6) {
                let x = -w * 0.5 + i as f32 * w * 0.2;
                p.inked(&PathData::circle(x, -h * 1.05, 2.5 + s), tops[i as usize % 4], INK, 0.5);
            }
            let mut tie = PathData::new();
            tie.move_to(-w * 0.6, -h * 1.02).quad_to(0.0, -h * 0.92, w * 0.6, -h * 1.02);
            p.stroke(&tie, Color::hex(0xb0a8c0), 1.2);
        });
    }
    sb.finish_any()
}

/// The bag frame for a number of pieces: it grows quickly at first, then
/// slowly (it cannot get bigger than the art allows, the count can).
pub fn bag_size(n: u64) -> u32 {
    match n {
        0 => 0,
        1..=4 => 0,
        5..=11 => 1,
        12..=24 => 2,
        25..=44 => 3,
        45..=74 => 4,
        75..=119 => 5,
        120..=199 => 6,
        _ => 7,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::costume::Costume;

    #[test]
    fn every_look_paints() {
        let mut c = Costume::default();
        c.slots[0] = crate::sim::costume::Slot::Hole;
        c.slots[5] = crate::sim::costume::Slot::Snag;
        c.slots[8] = crate::sim::costume::Slot::Patched(Mat::Gum, 1.0);
        c.dirt = 1.0;
        c.wet = 1.0;
        c.leaves = 4;
        for belly in 0..5 {
            let s = sheet(48, &c.look(belly), belly % 2 == 0);
            assert!(s.spec.anims.contains_key("walk_left"));
            assert!(s.width > 0);
        }
        let s = bag_sheet(48, false);
        assert_eq!(s.spec.anims["bag"].len(), 8);
    }
}
