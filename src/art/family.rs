//! Hand-made art for the people who matter: Loki, Skaði, their mother, and
//! Ethan Allen on his tower.
//!
//! Loki and Skaði are the children from Silkie Sprint, in Halloween
//! costumes a six-year-old and a fourteen-year-old would really wear, and
//! still themselves: Loki's ginger fringe and the tuft that never lies flat
//! poke out of the hood of his dinosaur hoodie, he still wears his green
//! wellies, and he has a plastic pumpkin pail; Skaði has her braids with red
//! ribbons, her blue dress with the white embroidered hem, her tights and
//! boots, and has added a cat-ear headband, eyeliner whiskers and a tail
//! safety-pinned on. Their mother is big, round, freckled, red-haired and
//! dressed as a jug of maple syrup, with a platter of hákarl. Ethan Allen is
//! Ethan Allen: tricorn, green coat with buff facings, sword, a sprig of
//! evergreen in his hat.

use super::SheetBuilder;
use super::paint::{self, Cloud, Painter, mirror, rot_about};
use crate::gfx::vector::{Affine, Color, PathData};

const INK: Color = Color::hex(0x3a2a2a);

fn limb(p: &mut Painter, a: (f32, f32), b: (f32, f32), w: f32, c: Color) {
    let mut d = PathData::new();
    d.move_to(a.0, a.1).line_to(b.0, b.1);
    p.stroke(&d, INK, w + 1.6);
    p.stroke(&d, c, w);
}

fn hand(p: &mut Painter, at: (f32, f32), r: f32, skin: Color) {
    p.inked(&PathData::circle(at.0, at.1, r), skin, INK, 0.6);
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum View {
    Front,
    Back,
    Side,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Pose {
    Stand,
    Walk(f32),
    Hold,
    Tug(u32),
    Cry(u32),
    Jump(u32),
    Sprawl,
    Sit,
    Hug(u32),
    Offer(u32),
    Laugh,
    Shout(u32),
    /// A skip: a hop and a swing, four frames.
    Skip(u32),
}

/// A pose's stride (-1 to 1) and how high a skip lifts.
fn stride_of(pose: Pose) -> (f32, f32) {
    match pose {
        Pose::Walk(s) => (s, 0.0),
        Pose::Skip(k) => ([0.9f32, 0.4, -0.9, -0.4][k as usize % 4], [1.0f32, 6.0, 1.0, 6.0][k as usize % 4]),
        _ => (0.0, 0.0),
    }
}

// ---------------------------------------------------------------- Loki --

const L_SKIN: Color = Color::hex(0xf6cfae);
const L_CHEEK: Color = Color::hex(0xf59a8a);
const L_HAIR: Color = Color::hex(0xe0702a);
const L_HAIR_LIGHT: Color = Color::hex(0xf39a4e);
const DINO: Color = Color::hex(0x5aa83a);
const DINO_DARK: Color = Color::hex(0x3f7f28);
const DINO_BELLY: Color = Color::hex(0xb6e08a);
const DINO_SPIKE: Color = Color::hex(0xf2c230);
const WELLY: Color = Color::hex(0x2e9c5a);
const PAIL: Color = Color::hex(0xf07a1c);

fn loki_face(p: &mut Painter, x: f32, y: f32, side: bool, mood: u8) {
    let fx = if side { x + 3.0 } else { x };
    p.inked(&PathData::ellipse(fx, y + 2.0, 11.0, 10.5), L_SKIN, INK, 1.1);
    let (ex, ex2) = if side { (x + 7.0, x + 7.0) } else { (x - 4.5, x + 4.5) };
    for (i, e) in [ex, ex2].into_iter().enumerate() {
        if side && i == 1 {
            break;
        }
        match mood {
            1 => paint::happy_eye(p, e, y, 2.2, INK),
            2 => {
                paint::eye(p, e, y + 0.5, 2.0, 0.5, INK);
                p.fill(&PathData::ellipse(e, y + 5.0, 1.3, 2.8), Color::hex(0x8ac8f0));
            }
            _ => paint::eye(p, e, y, 2.1, 1.0, INK),
        }
    }
    p.fill(&PathData::circle(ex - 2.5, y + 5.5, 2.6), L_CHEEK.with_alpha(170));
    if !side {
        p.fill(&PathData::circle(ex2 + 2.5, y + 5.5, 2.6), L_CHEEK.with_alpha(170));
    }
    for (a, b) in [(-1.5f32, 4.0f32), (1.0, 4.5), (3.5, 3.8)] {
        p.fill(&PathData::circle(fx + a, y + b, 0.6), Color::hex(0xc98a6a));
    }
    let mx = if side { x + 8.0 } else { x };
    let mut mouth = PathData::new();
    match mood {
        2 => {
            // Wailing.
            p.inked(&PathData::ellipse(mx, y + 8.0, 3.2, 2.6), Color::hex(0x8a2a38), INK, 0.6);
        }
        _ => {
            mouth.move_to(mx - 3.5, y + 6.5).quad_to(mx, y + 9.5 + mood as f32 * 2.0, mx + 3.5, y + 6.5).close();
            p.inked(&mouth, Color::hex(0xb03a48), INK, 0.6);
        }
    }
    // His fringe and the tuft, out of the hood.
    let fringe = Cloud::new(fx, y - 7.0, 10.0, 4.5, 7, 0.45, 43);
    p.cloud_lit(&fringe, INK, 1.0, fx - 3.0, y - 10.0, &[(0.0, L_HAIR_LIGHT), (1.0, L_HAIR)]);
    let mut tuft = PathData::new();
    tuft.move_to(fx - 1.0, y - 10.0).quad_to(fx + 3.0, y - 19.0, fx + 8.0, y - 17.0);
    p.stroke(&tuft, INK, 4.0);
    p.stroke(&tuft, L_HAIR, 2.4);
}

/// The dinosaur hood: green, spikes down the crown, felt teeth over the
/// brow (from the front).
fn loki_hood(p: &mut Painter, x: f32, y: f32, view: View) {
    let hood = PathData::ellipse(x + if view == View::Side { 1.0 } else { 0.0 }, y - 1.0, 15.0, 14.5);
    p.inked(&hood, DINO, INK, 1.2);
    let mut spikes = PathData::new();
    let n = 5;
    for k in 0..n {
        let a = match view {
            View::Side => -2.9 + k as f32 * 0.42,
            _ => -2.55 + k as f32 * 0.42,
        };
        let (sx, sy) = (x + a.cos() * 14.0, y - 1.0 + a.sin() * 13.5);
        spikes.extend(&PathData::polygon(&[(sx - 2.6 * a.sin(), sy + 2.6 * a.cos()), (sx + 2.6 * a.sin(), sy - 2.6 * a.cos()), (sx + a.cos() * 6.0, sy + a.sin() * 6.0)]));
    }
    p.inked(&spikes, DINO_SPIKE, INK, 0.6);
    if view != View::Back {
        let mut teeth = PathData::new();
        let ox = if view == View::Side { x + 4.0 } else { x };
        for k in 0..5 {
            let tx = ox - 8.0 + k as f32 * 4.0;
            teeth.extend(&PathData::polygon(&[(tx - 1.6, y - 9.0), (tx + 1.6, y - 9.0), (tx, y - 5.5)]));
        }
        p.inked(&teeth, Color::hex(0xfaf6ea), INK, 0.4);
        // Eyes of the dinosaur on top of the hood.
        for e in if view == View::Side { vec![ox + 3.0] } else { vec![ox - 6.0, ox + 6.0] } {
            p.inked(&PathData::circle(e, y - 12.0, 2.4), Color::hex(0xfaf6ea), INK, 0.5);
            p.fill(&PathData::circle(e + 0.5, y - 12.0, 1.1), INK);
        }
    }
}

fn loki_draw(p: &mut Painter, view: View, pose: Pose) {
    paint::shadow(p, 12.0, 3.5);
    let lift = if let Pose::Jump(k) = pose { -(k as f32) * 8.0 } else { -stride_of(pose).1 };
    let sit = pose == Pose::Sit;
    if pose == Pose::Sprawl {
        p.push(rot_about(-80.0, 0.0, -14.0));
    }
    p.push(Affine::translate(0.0, lift + if sit { 10.0 } else { 0.0 }));
    let lean = match pose {
        Pose::Tug(k) => -16.0 - 6.0 * k as f32,
        _ => 0.0,
    };
    p.push(rot_about(lean, 0.0, -18.0));
    let (step, _) = stride_of(pose);
    let head_y = -42.0;
    // The tail, behind.
    if view != View::Back {
        let mut t = PathData::new();
        let sx = if view == View::Side { -8.0 } else { 7.0 };
        t.move_to(sx, -18.0).quad_to(sx - 12.0 + step * 2.0, -12.0, sx - 17.0, -3.0);
        p.stroke(&t, INK, 10.0);
        p.stroke(&t, DINO, 8.0);
        let mut ts = PathData::new();
        ts.move_to(sx - 6.0, -19.0).line_to(sx - 9.0, -22.0).line_to(sx - 11.0, -17.0).line_to(sx - 14.0, -18.0).line_to(sx - 15.0, -12.0);
        p.stroke(&ts, DINO_SPIKE, 1.6);
    }
    // Legs in the onesie, and his wellies.
    let legs: [(f32, f32); 2] = match view {
        View::Side => [(-2.0 + step * 6.0, -(step.max(0.0)) * 1.5), (2.0 - step * 6.0, -((-step).max(0.0)) * 1.5)],
        _ => [(-5.0, -(step.max(0.0)) * 2.0), (5.0, -((-step).max(0.0)) * 2.0)],
    };
    for (lx, up) in legs {
        limb(p, (lx * 0.6, -17.0), (lx, -6.0 - up), 6.5, DINO);
        p.inked(&PathData::round_rect(lx - 4.0 + if view == View::Side { 1.0 } else { 0.0 }, -9.0 - up, 9.0, 9.0, 3.0), WELLY, INK, 0.9);
    }
    // Body.
    let body = PathData::round_rect(-10.0, -30.0, 20.0, 16.0, 7.0);
    p.inked(&body, DINO, INK, 1.2);
    if view != View::Back {
        p.fill(&PathData::ellipse(if view == View::Side { 3.0 } else { 0.0 }, -22.0, 6.0, 6.0), DINO_BELLY);
    } else {
        let mut sp = PathData::new();
        for k in 0..3 {
            let y = -29.0 + k as f32 * 5.0;
            sp.extend(&PathData::polygon(&[(-2.5, y + 3.0), (2.5, y + 3.0), (0.0, y - 1.0)]));
        }
        p.inked(&sp, DINO_SPIKE, INK, 0.5);
    }
    // Arms, the pail.
    let (h1, h2) = match pose {
        Pose::Hold | Pose::Hug(_) => ((-4.0, -24.0), (4.0, -24.0)),
        Pose::Tug(_) => ((18.0, -26.0), (17.0, -23.0)),
        Pose::Jump(_) => ((-15.0, -40.0), (15.0, -40.0)),
        Pose::Cry(_) => ((-3.0, head_y + 4.0), (3.0, head_y + 4.0)),
        Pose::Sprawl => ((-16.0, -30.0), (16.0, -30.0)),
        _ => match view {
            View::Side => ((-2.0 - step * 5.0, -15.0), (2.0 + step * 5.0, -15.0)),
            _ => ((-12.0, -15.0 + step), (12.0, -15.0 - step)),
        },
    };
    limb(p, (-8.0, -27.0), h1, 5.0, DINO_DARK);
    loki_hood(p, 0.0, head_y, view);
    if view != View::Back {
        let mood = match pose {
            Pose::Cry(_) => 2,
            Pose::Jump(_) | Pose::Hug(_) | Pose::Sprawl => 1,
            _ => 0,
        };
        loki_face(p, 0.0, head_y + 2.0, view == View::Side, mood);
    }
    limb(p, (8.0, -27.0), h2, 5.0, DINO);
    hand(p, h1, 2.8, L_SKIN);
    hand(p, h2, 2.8, L_SKIN);
    if matches!(pose, Pose::Stand | Pose::Walk(_) | Pose::Sit) {
        let (x, y) = h2;
        let pail = PathData::round_rect(x - 4.5, y + 1.0, 9.0, 8.0, 3.0);
        p.inked(&pail, PAIL, INK, 0.6);
        p.fill(&PathData::polygon(&[(x - 2.6, y + 3.6), (x - 0.8, y + 3.6), (x - 1.7, y + 2.4)]), INK);
        p.fill(&PathData::polygon(&[(x + 0.8, y + 3.6), (x + 2.6, y + 3.6), (x + 1.7, y + 2.4)]), INK);
        let mut hdl = PathData::new();
        hdl.move_to(x - 4.0, y + 1.5).quad_to(x, y - 3.5, x + 4.0, y + 1.5);
        p.stroke(&hdl, INK, 0.9);
    }
    p.pop();
    p.pop();
    if pose == Pose::Sprawl {
        p.pop();
    }
}

// --------------------------------------------------------------- Skaði --

const S_SKIN: Color = Color::hex(0xf3cfb0);
const S_CHEEK: Color = Color::hex(0xf0a090);
const S_HAIR: Color = Color::hex(0xe8782e);
const S_HAIR_DARK: Color = Color::hex(0xb8561c);
const DRESS: Color = Color::hex(0x2f6fb8);
const DRESS_DARK: Color = Color::hex(0x24578f);
const HEM: Color = Color::hex(0xf6f1e4);
const RIBBON: Color = Color::hex(0xe53935);
const BOOT: Color = Color::hex(0x6d4c41);
const TIGHTS: Color = Color::hex(0x8e5a8a);
const CAT: Color = Color::hex(0x1e1a22);

fn braid(p: &mut Painter, x: f32, y: f32, end: (f32, f32)) {
    let mut d = PathData::new();
    d.move_to(x, y).quad_to((x + end.0) / 2.0 - 2.0, (y + end.1) / 2.0, end.0, end.1);
    p.stroke(&d, INK, 6.4);
    p.stroke(&d, S_HAIR, 4.8);
    let mut plait = PathData::new();
    for i in 1..5 {
        let t = i as f32 / 5.0;
        let px = x + (end.0 - x) * t;
        let py = y + (end.1 - y) * t;
        plait.move_to(px - 2.0, py - 1.3).line_to(px, py + 0.9).line_to(px + 2.0, py - 1.3);
    }
    p.stroke(&plait, S_HAIR_DARK, 0.9);
    p.inked(&PathData::ellipse(end.0, end.1, 3.0, 2.2), RIBBON, INK, 0.6);
}

fn skadi_head(p: &mut Painter, x: f32, y: f32, view: View, mood: u8) {
    let side = view == View::Side;
    if view == View::Back {
        p.inked(&PathData::ellipse(x, y, 10.0, 10.5), S_HAIR, INK, 1.1);
        let mut part = PathData::new();
        part.move_to(x, y - 10.0).line_to(x, y + 6.0);
        p.stroke(&part, S_HAIR_DARK, 1.0);
        braid(p, x - 5.0, y + 5.0, (x - 7.0, y + 26.0));
        braid(p, x + 5.0, y + 5.0, (x + 7.0, y + 26.0));
    } else {
        let fx = if side { x + 2.0 } else { x };
        p.inked(&PathData::ellipse(fx, y, 10.0, 10.5), S_SKIN, INK, 1.1);
        let eyes: Vec<f32> = if side { vec![x + 6.0] } else { vec![x - 4.0, x + 4.0] };
        for &e in &eyes {
            match mood {
                1 => paint::happy_eye(p, e, y - 1.0, 2.0, INK),
                _ => paint::eye(p, e, y - 1.0, 1.9, 1.0, INK),
            }
        }
        for &e in &eyes {
            p.fill(&PathData::circle(e + if e < x { -1.5 } else { 1.5 }, y + 4.0, 2.2), S_CHEEK.with_alpha(150));
        }
        for (a, b) in [(-3.0f32, 2.5f32), (2.5, 3.0), (4.5, 2.0)] {
            p.fill(&PathData::circle(fx + a, y + b, 0.6), Color::hex(0xc98a6a));
        }
        // Cat nose and whiskers, in eyeliner.
        let nx = if side { x + 10.0 } else { x };
        p.fill(&PathData::ellipse(nx, y + 3.0, 1.4, 1.0), INK);
        let mut w = PathData::new();
        for k in [-1.0f32, 0.0, 1.0] {
            w.move_to(nx - 2.0, y + 4.2 + k).line_to(nx - 8.0, y + 3.6 + k * 2.0);
            if !side {
                w.move_to(nx + 2.0, y + 4.2 + k).line_to(nx + 8.0, y + 3.6 + k * 2.0);
            }
        }
        p.stroke(&w, INK, 0.6);
        let mx = if side { x + 7.0 } else { x };
        let mut mouth = PathData::new();
        mouth.move_to(mx - 2.5, y + 6.5).quad_to(mx, y + 8.5 + mood as f32, mx + 2.5, y + 6.5);
        p.stroke(&mouth, Color::hex(0xb03a48), 1.1);
        // Her hair, parted, braids over her shoulders.
        let hair = if side {
            PathData::smooth_closed(&[(x - 9.0, y + 4.0), (x - 8.0, y - 9.0), (x + 4.0, y - 12.0), (x + 11.0, y - 7.0), (x + 4.0, y - 6.0), (x - 3.0, y - 1.0)], 0.6)
        } else {
            PathData::smooth_closed(&[(x - 11.0, y - 1.0), (x - 9.0, y - 10.0), (x, y - 12.5), (x + 9.0, y - 10.0), (x + 11.0, y - 1.0), (x, y - 7.0)], 0.6)
        };
        p.inked(&hair, S_HAIR, INK, 1.0);
        if side {
            braid(p, x - 6.0, y + 2.0, (x - 11.0, y + 24.0));
        } else {
            braid(p, x - 8.0, y + 2.0, (x - 11.0, y + 24.0));
            braid(p, x + 8.0, y + 2.0, (x + 11.0, y + 24.0));
        }
    }
    // The cat-ear headband.
    let ears: Vec<f32> = if side { vec![x - 1.0, x + 5.0] } else { vec![x - 6.5, x + 6.5] };
    for (k, &ex) in ears.iter().enumerate() {
        let s = if k == 0 { -1.0 } else { 1.0 };
        p.inked(&PathData::polygon(&[(ex - 3.4, y - 8.5), (ex + 3.4, y - 8.5), (ex + s * 0.6, y - 16.5)]), CAT, INK, 0.6);
        p.fill(&PathData::polygon(&[(ex - 1.7, y - 9.0), (ex + 1.7, y - 9.0), (ex + s * 0.4, y - 13.8)]), Color::hex(0xf0a0b8));
    }
    let mut band = PathData::new();
    band.move_to(x - 10.0, y - 6.0).quad_to(x, y - 12.0, x + 10.0, y - 6.0);
    p.stroke(&band, CAT, 1.4);
}

fn skadi_draw(p: &mut Painter, view: View, pose: Pose) {
    paint::shadow(p, 12.0, 3.5);
    let lift = if let Pose::Jump(k) = pose { -(k as f32) * 10.0 } else { -stride_of(pose).1 };
    if pose == Pose::Sprawl {
        p.push(rot_about(-80.0, 0.0, -20.0));
    }
    if pose == Pose::Sit {
        p.push(Affine::translate(0.0, 14.0));
    }
    p.push(Affine::translate(0.0, lift));
    let lean = match pose {
        Pose::Tug(k) => -16.0 - 6.0 * k as f32,
        _ => 0.0,
    };
    p.push(rot_about(lean, 0.0, -26.0));
    let (step, _) = stride_of(pose);
    // The tail, safety-pinned to the back of the dress.
    let mut tail = PathData::new();
    let tx = if view == View::Side { -7.0 } else { 6.0 };
    tail.move_to(tx, -30.0).quad_to(tx - 9.0 + step * 2.0, -24.0, tx - 7.0, -10.0).quad_to(tx - 6.0, -6.0, tx - 2.0, -8.0);
    if view != View::Back {
        p.stroke(&tail, CAT, 2.6);
    }
    let legs: [(f32, f32); 2] = match view {
        View::Side => [(-2.0 + step * 7.0, -(step.max(0.0)) * 1.5), (2.0 - step * 7.0, -((-step).max(0.0)) * 1.5)],
        _ => [(-4.0, -(step.max(0.0)) * 2.0), (4.0, -((-step).max(0.0)) * 2.0)],
    };
    for (lx, up) in legs {
        limb(p, (lx * 0.6, -26.0), (lx, -4.0 - up), 4.4, TIGHTS);
        p.inked(&PathData::round_rect(lx - 3.5 + if view == View::Side { 1.0 } else { 0.0 }, -6.0 - up, 8.0, 6.0, 2.5), BOOT, INK, 0.8);
    }
    // The dress, with its hem.
    let flare = 3.0 + step.abs() * 1.5;
    let d = PathData::smooth_closed(&[(-7.0, -56.0), (7.0, -56.0), (10.0 + flare, -24.0), (-10.0 - flare, -24.0)], 0.15);
    p.inked(&d, DRESS, INK, 1.1);
    p.fill(&PathData::rect(-10.0 - flare, -28.0, 20.0 + flare * 2.0, 4.0), HEM);
    let mut stitch = PathData::new();
    let mut sx = -9.0 - flare;
    while sx < 9.0 + flare {
        stitch.move_to(sx, -27.4).line_to(sx + 1.6, -25.0).line_to(sx + 3.2, -27.4);
        sx += 3.2;
    }
    p.stroke(&stitch, RIBBON, 0.7);
    p.fill(&PathData::rect(-7.0, -46.0, 14.0, 2.4), DRESS_DARK);
    if view == View::Back {
        p.stroke(&tail, CAT, 2.6);
    }
    let (h1, h2) = match pose {
        Pose::Hold => ((-4.0, -42.0), (4.0, -42.0)),
        Pose::Hug(_) => ((-10.0, -48.0), (-14.0, -44.0)),
        Pose::Tug(_) => ((22.0, -48.0), (20.0, -44.0)),
        Pose::Jump(_) => ((-16.0, -66.0), (16.0, -66.0)),
        Pose::Sprawl => ((-18.0, -52.0), (18.0, -52.0)),
        _ => match view {
            View::Side => ((-1.0 - step * 6.0, -34.0), (1.0 + step * 6.0, -34.0)),
            _ => ((-11.0, -34.0 + step), (11.0, -34.0 - step)),
        },
    };
    limb(p, (-6.0, -53.0), h1, 4.0, DRESS_DARK);
    skadi_head(
        p,
        0.0,
        -66.0,
        view,
        match pose {
            Pose::Jump(_) | Pose::Hug(_) | Pose::Hold => 1,
            _ => 0,
        },
    );
    limb(p, (6.0, -53.0), h2, 4.0, DRESS);
    hand(p, h1, 2.6, S_SKIN);
    hand(p, h2, 2.6, S_SKIN);
    if matches!(pose, Pose::Stand | Pose::Walk(_) | Pose::Sit) {
        let (x, y) = h2;
        let sack = PathData::smooth_closed(&[(x - 3.5, y), (x + 3.5, y), (x + 6.0, y + 11.0), (x, y + 13.0), (x - 6.0, y + 11.0)], 0.8);
        p.inked(&sack, Color::hex(0xf0ecf4), INK, 0.6);
    }
    p.pop();
    p.pop();
    if pose == Pose::Sit || pose == Pose::Sprawl {
        p.pop();
    }
}

// ------------------------------------------------------------------ Mom --

const M_SKIN: Color = Color::hex(0xf2c8a8);
const M_HAIR: Color = Color::hex(0xc8401e);
const M_HAIR_LIGHT: Color = Color::hex(0xe8683a);
const JUG: Color = Color::hex(0xd08a2a);

/// Their mother: a sturdy, round woman (broad shoulders, round arms, a soft
/// double chin) in a big jug of maple syrup, her red hair in a very long
/// ponytail brought forward over one shoulder.
fn mom_draw(p: &mut Painter, pose: Pose) {
    paint::shadow(p, 30.0, 6.0);
    let lean = if pose == Pose::Laugh { -5.0 } else { 0.0 };
    p.push(rot_about(lean, 0.0, -30.0));
    // Sturdy legs and sensible shoes under the jug.
    for x in [-11.0f32, 11.0] {
        limb(p, (x * 0.92, -18.0), (x * 0.95, -4.0), 9.0, Color::hex(0x3a3048));
        p.inked(&PathData::round_rect(x - 7.0, -7.0, 14.0, 7.0, 3.0), Color::hex(0x5a3a2a), INK, 0.8);
    }
    // The jug: amber, round-shouldered, with a handle and a label.
    let top = -76.0;
    let w = 29.0;
    let jug = PathData::smooth_closed(&[(-w * 0.62, top), (w * 0.62, top), (w * 0.98, top + 12.0), (w * 1.05, -22.0), (w * 0.9, -12.0), (-w * 0.9, -12.0), (-w * 1.05, -22.0), (-w * 0.98, top + 12.0)], 0.7);
    p.inked(&jug, JUG, INK, 1.4);
    p.radial(&jug, -12.0, top + 14.0, 50.0, &[(0.0, Color::rgba(255, 224, 150, 160)), (1.0, Color::rgba(150, 80, 16, 0))]);
    let mut handle = PathData::new();
    handle.move_to(w * 0.35, top + 1.0).quad_to(w * 1.25, top - 9.0, w * 0.95, top + 15.0);
    p.stroke(&handle, INK, 7.0);
    p.stroke(&handle, JUG, 5.0);
    let ly = -44.0;
    p.inked(&PathData::round_rect(-w * 0.78, ly - 12.0, w * 1.56, 24.0, 4.0), Color::hex(0xf2ead6), INK, 0.7);
    p.fill(&PathData::rect(-w * 0.78, ly - 12.0, w * 1.56, 6.0), Color::hex(0xc8302a));
    p.fill(&PathData::rect(-w * 0.78, ly + 8.0, w * 1.56, 4.0), Color::hex(0xc8302a));
    // A red farmhouse, a maple, a hill.
    p.fill(&PathData::ellipse(0.0, ly + 7.0, w * 0.7, 4.0), Color::hex(0x6aa04a));
    p.fill(&PathData::polygon(&[(-14.0, ly + 6.0), (-14.0, ly - 1.0), (-9.0, ly - 5.0), (-4.0, ly - 1.0), (-4.0, ly + 6.0)]), Color::hex(0xb02a2a));
    p.fill(&PathData::circle(10.0, ly - 1.0, 5.0), Color::hex(0xe0632a));
    p.fill(&PathData::rect(9.4, ly + 2.0, 1.4, 4.5), Color::hex(0x5a3a22));
    // Broad shoulders and round arms out of the jug's sides.
    let (h1, h2) = match pose {
        Pose::Offer(k) => ((-21.0, -60.0 - k as f32 * 3.0), (21.0, -60.0 - k as f32 * 3.0)),
        Pose::Laugh => ((-26.0, -58.0), (22.0, -88.0)),
        _ => ((-21.0, -57.0), (21.0, -57.0)),
    };
    limb(p, (-w * 0.72, top + 6.0), h1, 8.5, M_SKIN);
    limb(p, (w * 0.72, top + 6.0), h2, 8.5, M_SKIN);
    // Her head, a cloud of red curls, the jug's cap for a hat.
    let hy = top - 13.0;
    // Hair pulled back, smooth on top.
    let cap = PathData::smooth_closed(&[(-14.0, hy + 2.0), (-13.0, hy - 10.0), (0.0, hy - 16.0), (13.0, hy - 10.0), (14.0, hy + 2.0), (0.0, hy - 8.0)], 0.6);
    p.inked(&cap, M_HAIR, INK, 1.1);
    p.inked(&PathData::ellipse(0.0, hy + 1.0, 13.0, 12.5), M_SKIN, INK, 1.1);
    let mut chin = PathData::new();
    chin.move_to(-7.0, hy + 11.0).quad_to(0.0, hy + 15.5, 7.0, hy + 11.0);
    p.stroke(&chin, INK.with_alpha(130), 1.0);
    let glad = matches!(pose, Pose::Laugh | Pose::Offer(_));
    for e in [-5.0f32, 5.0] {
        if glad {
            paint::happy_eye(p, e, hy - 1.0, 2.4, INK);
        } else {
            paint::eye(p, e, hy - 1.0, 2.2, 1.0, INK);
        }
        p.fill(&PathData::circle(e * 1.6, hy + 4.0, 3.0), Color::rgba(240, 120, 110, 140));
    }
    for (a, b) in [(-4.0f32, 2.0f32), (-2.0, 3.0), (3.0, 2.5), (5.0, 3.5), (0.5, 4.0)] {
        p.fill(&PathData::circle(a, hy + b, 0.7), Color::hex(0xb07050));
    }
    let mut mouth = PathData::new();
    if pose == Pose::Laugh {
        p.inked(&PathData::ellipse(0.0, hy + 7.0, 4.0, 3.0), Color::hex(0x8a2a38), INK, 0.6);
    } else {
        mouth.move_to(-4.0, hy + 6.0).quad_to(0.0, hy + 9.5, 4.0, hy + 6.0).close();
        p.inked(&mouth, Color::hex(0xb03a48), INK, 0.6);
    }
    p.inked(&PathData::round_rect(-8.0, hy - 21.0, 16.0, 7.0, 2.0), Color::hex(0xd8302a), INK, 0.8);
    // The ponytail: gathered behind her ear, brought forward over her
    // shoulder and down the front of the jug, nearly to her waist, tapering
    // to a soft tip.
    let curve = |t: f32| -> (f32, f32) {
        // A gentle S: one cubic over the shoulder, one down the front.
        let cubic = |p0: (f32, f32), p1: (f32, f32), p2: (f32, f32), p3: (f32, f32), u: f32| {
            let v = 1.0 - u;
            (
                v * v * v * p0.0 + 3.0 * v * v * u * p1.0 + 3.0 * v * u * u * p2.0 + u * u * u * p3.0,
                v * v * v * p0.1 + 3.0 * v * v * u * p1.1 + 3.0 * v * u * u * p2.1 + u * u * u * p3.1,
            )
        };
        let mid = (w * 0.66, top + 16.0);
        if t < 0.4 {
            cubic((11.0, hy + 3.0), (20.0, hy + 8.0), (w * 0.82, top + 2.0), mid, t / 0.4)
        } else {
            cubic(mid, (w * 0.54, top + 30.0), (w * 0.6, top + 44.0), (w * 0.5, top + 58.0), (t - 0.4) / 0.6)
        }
    };
    let n = 24;
    let mut left = Vec::new();
    let mut right = Vec::new();
    for k in 0..=n {
        let t = k as f32 / n as f32;
        let (x, y) = curve(t);
        let (x2, y2) = curve((t + 0.02).min(1.0));
        let (dx, dy) = if k == n { let (x0, y0) = curve(t - 0.02); (x - x0, y - y0) } else { (x2 - x, y2 - y) };
        let len = (dx * dx + dy * dy).sqrt().max(0.001);
        let (nx, ny) = (-dy / len, dx / len);
        let half = 3.9 * (1.0 - t * 0.6) + (t * 14.0).sin() * 0.3;
        left.push((x + nx * half, y + ny * half));
        right.push((x - nx * half, y - ny * half));
    }
    right.reverse();
    left.extend(right);
    let hank = PathData::smooth_closed(&left, 0.5);
    p.inked(&hank, M_HAIR, INK, 0.9);
    let mut strands = PathData::new();
    for off in [-1.4f32, 0.2, 1.5] {
        let mut first = true;
        for k in 1..n {
            let t = k as f32 / n as f32;
            let (x, y) = curve(t);
            let (x, y) = (x + off * (1.0 - t * 0.6), y);
            if first {
                strands.move_to(x, y);
                first = false;
            } else {
                strands.line_to(x, y);
            }
        }
    }
    p.stroke(&strands, M_HAIR_LIGHT.with_alpha(170), 0.8);
    // The tie.
    p.inked(&PathData::ellipse(12.5, hy + 4.0, 3.0, 2.2), Color::hex(0x2e8a4a), INK, 0.6);
    hand(p, h1, 4.0, M_SKIN);
    hand(p, h2, 4.0, M_SKIN);
    // The platter of hákarl, and the smell.
    if pose != Pose::Laugh {
        let py = h1.1 - 3.0;
        p.inked(&PathData::ellipse(0.0, py, 20.0, 5.0), Color::hex(0xd8dce4), INK, 0.7);
        let mut cubes = PathData::new();
        for k in 0..7 {
            cubes.extend(&PathData::rect(-14.0 + k as f32 * 4.2, py - 4.5, 3.2, 3.2));
        }
        p.inked(&cubes, Color::hex(0xece4c8), INK.with_alpha(120), 0.3);
        let mut stink = PathData::new();
        for k in 0..3 {
            let wx = -8.0 + k as f32 * 8.0;
            stink.move_to(wx, py - 7.0).quad_to(wx + 3.0, py - 11.0, wx, py - 15.0).quad_to(wx - 3.0, py - 19.0, wx, py - 23.0);
        }
        p.stroke(&stink, Color::rgba(160, 200, 100, 180), 1.4);
    }
    p.pop();
}

// ---------------------------------------------------------- Ethan Allen --

const COAT: Color = Color::hex(0x2e5a34);
const BUFF: Color = Color::hex(0xd8c08a);
const E_SKIN: Color = Color::hex(0xe8b896);

/// Ethan Allen, leader of the Green Mountain Boys, on top of his tower.
fn ethan_draw(p: &mut Painter, pose: Pose) {
    paint::shadow(p, 16.0, 4.0);
    // Boots and breeches.
    for x in [-6.0f32, 6.0] {
        limb(p, (x * 0.7, -40.0), (x, -18.0), 7.0, BUFF);
        p.inked(&PathData::round_rect(x - 4.5, -20.0, 9.0, 20.0, 2.5), Color::hex(0x1e1a1a), INK, 0.8);
    }
    // The coat, long, green, buff facings, brass buttons.
    let coat = PathData::smooth_closed(&[(-13.0, -82.0), (13.0, -82.0), (17.0, -44.0), (19.0, -30.0), (6.0, -32.0), (0.0, -40.0), (-6.0, -32.0), (-19.0, -30.0), (-17.0, -44.0)], 0.3);
    p.inked(&coat, COAT, INK, 1.2);
    p.fill(&PathData::polygon(&[(-6.0, -82.0), (6.0, -82.0), (4.0, -44.0), (-4.0, -44.0)]), BUFF);
    let mut buttons = PathData::new();
    for k in 0..5 {
        buttons.extend(&PathData::circle(-7.5, -76.0 + k as f32 * 7.0, 1.1));
        buttons.extend(&PathData::circle(7.5, -76.0 + k as f32 * 7.0, 1.1));
    }
    p.fill(&buttons, Color::hex(0xe0b840));
    // Epaulettes.
    for s in [-1.0f32, 1.0] {
        p.inked(&PathData::ellipse(s * 13.0, -81.0, 5.0, 2.4), Color::hex(0xe0b840), INK, 0.5);
    }
    // Sword at the hip.
    let mut sword = PathData::new();
    sword.move_to(-12.0, -46.0).line_to(-20.0, -16.0);
    p.stroke(&sword, INK, 2.6);
    p.stroke(&sword, Color::hex(0xc8ccd4), 1.4);
    p.fill(&PathData::rect(-14.0, -50.0, 4.0, 4.0), Color::hex(0xe0b840));
    // Arms: one on his hip, one raised (shouting from the parapet).
    let raised = match pose {
        Pose::Shout(k) => (17.0 + k as f32 * 2.0, -104.0 - k as f32 * 4.0),
        _ => (17.0, -54.0),
    };
    limb(p, (-12.0, -78.0), (-15.0, -56.0), 6.0, COAT);
    limb(p, (12.0, -78.0), raised, 6.0, COAT);
    hand(p, (-15.0, -56.0), 3.0, E_SKIN);
    hand(p, raised, 3.0, E_SKIN);
    // The cravat, the head: strong jaw, grey queue, stern brow.
    p.inked(&PathData::ellipse(0.0, -82.0, 5.0, 3.6), Color::hex(0xf6f2ea), INK, 0.5);
    let hy = -94.0;
    let mut queue = PathData::new();
    queue.move_to(-6.0, hy + 2.0).quad_to(-11.0, hy + 8.0, -9.0, hy + 14.0);
    p.stroke(&queue, INK, 4.0);
    p.stroke(&queue, Color::hex(0xc8c4bc), 2.6);
    p.inked(&PathData::ellipse(0.0, hy, 10.5, 11.0), E_SKIN, INK, 1.1);
    p.fill(&PathData::ellipse(-9.0, hy + 1.0, 3.0, 5.0), Color::hex(0xd8d4cc));
    p.fill(&PathData::ellipse(9.0, hy + 1.0, 3.0, 5.0), Color::hex(0xd8d4cc));
    let mut brow = PathData::new();
    brow.move_to(-6.5, hy - 4.5).line_to(-2.0, hy - 3.0).move_to(6.5, hy - 4.5).line_to(2.0, hy - 3.0);
    p.stroke(&brow, INK, 1.4);
    paint::eye(p, -4.0, hy - 1.0, 1.8, 1.0, INK);
    paint::eye(p, 4.0, hy - 1.0, 1.8, 1.0, INK);
    let mut nose = PathData::new();
    nose.move_to(0.0, hy - 1.0).line_to(1.5, hy + 4.0).line_to(-0.5, hy + 4.5);
    p.stroke(&nose, INK.with_alpha(150), 0.8);
    if matches!(pose, Pose::Shout(_)) {
        p.inked(&PathData::ellipse(0.0, hy + 7.0, 3.2, 2.6), Color::hex(0x6a2028), INK, 0.6);
    } else {
        let mut m = PathData::new();
        m.move_to(-3.0, hy + 7.0).line_to(3.0, hy + 7.0);
        p.stroke(&m, INK, 1.0);
    }
    // The tricorn: three corners, gold trim, a sprig of evergreen.
    let hat = PathData::polygon(&[(-17.0, hy - 6.0), (0.0, hy - 13.0), (17.0, hy - 6.0), (11.0, hy - 17.0), (0.0, hy - 22.0), (-11.0, hy - 17.0)]);
    p.inked(&hat, Color::hex(0x1a1818), INK, 1.0);
    let mut trim = PathData::new();
    trim.move_to(-17.0, hy - 6.0).line_to(0.0, hy - 13.0).line_to(17.0, hy - 6.0);
    p.stroke(&trim, Color::hex(0xe0b840), 1.2);
    let mut sprig = PathData::new();
    sprig.move_to(8.0, hy - 13.0).line_to(13.0, hy - 24.0);
    for k in 0..4 {
        let t = k as f32 / 4.0;
        let (x, y) = (8.0 + 5.0 * t, hy - 13.0 - 11.0 * t);
        sprig.move_to(x, y).line_to(x - 3.0, y - 2.0).move_to(x, y).line_to(x + 3.0, y - 1.0);
    }
    p.stroke(&sprig, Color::hex(0x2e8a4a), 1.3);
}

// --------------------------------------------------------- Robert Frost --

const TWEED: Color = Color::hex(0x7d6a52);
const TWEED_FLECK: Color = Color::hex(0xa8957a);
const F_SKIN: Color = Color::hex(0xeec4a6);
const F_HAIR: Color = Color::hex(0xf2f0ea);

/// Robert Frost (1874 to 1963), poet, Vermont's poet laureate: a shock of
/// white hair, heavy white brows, a rumpled tweed jacket and a book, in the
/// library door. `Pose::Shout` here is reciting, a hand keeping time.
fn frost_draw(p: &mut Painter, pose: Pose) {
    paint::shadow(p, 15.0, 4.0);
    // Trousers and sensible shoes.
    for x in [-5.5f32, 5.5] {
        limb(p, (x * 0.8, -44.0), (x, -6.0), 8.0, Color::hex(0x4a4a50));
        p.inked(&PathData::ellipse(x + 1.5, -3.0, 6.0, 3.0), Color::hex(0x4a2e1e), INK, 0.8);
    }
    // White shirt, a loosened tie, and the jacket over it, a little big.
    p.inked(&PathData::polygon(&[(-7.0, -86.0), (7.0, -86.0), (5.0, -50.0), (-5.0, -50.0)]), Color::hex(0xf4f2ec), INK, 0.8);
    let mut tie = PathData::new();
    tie.move_to(0.0, -84.0).line_to(2.6, -78.0).line_to(1.0, -60.0).line_to(-1.5, -62.0).line_to(-1.6, -78.0).close();
    p.inked(&tie, Color::hex(0x7a2a2a), INK, 0.6);
    let coat_l = PathData::smooth_closed(&[(-7.0, -87.0), (-15.0, -84.0), (-18.0, -60.0), (-17.0, -40.0), (-3.0, -38.0), (-4.0, -60.0)], 0.3);
    let coat_r = PathData::smooth_closed(&[(7.0, -87.0), (15.0, -84.0), (18.0, -60.0), (17.0, -40.0), (3.0, -38.0), (4.0, -60.0)], 0.3);
    p.inked(&coat_l, TWEED, INK, 1.1);
    p.inked(&coat_r, TWEED, INK, 1.1);
    // Herringbone, more or less.
    let mut fleck = PathData::new();
    for k in 0..6 {
        let y = -80.0 + k as f32 * 6.5;
        for x in [-13.0f32, -9.0, 9.0, 13.0] {
            fleck.move_to(x - 1.5, y).line_to(x, y - 1.5).line_to(x + 1.5, y);
        }
    }
    p.stroke(&fleck, TWEED_FLECK, 0.7);
    // Elbow patches are implied. One hand holds the book open; the other
    // keeps time with the verse.
    let book_at = (-9.0, -58.0);
    let free = match pose {
        Pose::Shout(k) => (16.0, -70.0 - k as f32 * 6.0),
        _ => (15.0, -52.0),
    };
    limb(p, (-13.0, -82.0), (book_at.0 - 2.0, book_at.1 + 2.0), 6.5, TWEED);
    limb(p, (13.0, -82.0), free, 6.5, TWEED);
    let book = PathData::polygon(&[(book_at.0 - 10.0, book_at.1 - 4.0), (book_at.0, book_at.1 - 1.0), (book_at.0 + 10.0, book_at.1 - 4.0), (book_at.0 + 10.0, book_at.1 + 5.0), (book_at.0, book_at.1 + 8.0), (book_at.0 - 10.0, book_at.1 + 5.0)]);
    p.inked(&book, Color::hex(0xfaf6e8), INK, 0.8);
    let mut spine = PathData::new();
    spine.move_to(book_at.0, book_at.1 - 1.0).line_to(book_at.0, book_at.1 + 8.0);
    for k in 0..3 {
        let y = book_at.1 + k as f32 * 2.2;
        spine.move_to(book_at.0 - 8.0, y - 1.5).line_to(book_at.0 - 2.0, y);
        spine.move_to(book_at.0 + 2.0, y).line_to(book_at.0 + 8.0, y - 1.5);
    }
    p.stroke(&spine, INK.with_alpha(140), 0.6);
    hand(p, (book_at.0 - 1.0, book_at.1 + 5.0), 3.0, F_SKIN);
    hand(p, free, 3.0, F_SKIN);
    // The head: broad, lined, kindly, with that hair.
    let hy = -98.0;
    p.inked(&PathData::round_rect(-3.5, hy + 8.0, 7.0, 6.0, 2.0), F_SKIN, INK, 0.8);
    p.inked(&PathData::ellipse(0.0, hy, 11.0, 12.0), F_SKIN, INK, 1.1);
    let hair = PathData::smooth_closed(
        &[(-12.0, hy - 1.0), (-14.0, hy - 9.0), (-9.0, hy - 15.0), (-3.0, hy - 19.0), (4.0, hy - 17.0), (11.0, hy - 15.0), (15.0, hy - 8.0), (12.0, hy - 1.0), (8.0, hy - 9.0), (0.0, hy - 11.0), (-7.0, hy - 9.0)],
        0.35,
    );
    p.inked(&hair, F_HAIR, INK, 1.0);
    // Tousled at the temples, and a lock the wind got at.
    for (x, d) in [(-12.5f32, -1.0f32), (12.5, 1.0)] {
        let tuft = PathData::polygon(&[(x, hy - 7.0), (x + d * 5.0, hy - 6.0), (x + d * 2.5, hy - 3.0), (x + d * 5.5, hy - 1.0), (x + d * 1.0, hy + 1.0)]);
        p.inked(&tuft, F_HAIR, INK, 0.8);
    }
    let lock = PathData::polygon(&[(-2.0, hy - 17.0), (3.0, hy - 22.0), (2.0, hy - 18.0), (7.0, hy - 20.0), (4.0, hy - 15.0)]);
    p.inked(&lock, F_HAIR, INK, 0.8);
    // Heavy white brows, eyes with a twinkle, lines of a long life.
    let mut brow = PathData::new();
    brow.move_to(-8.0, hy - 4.0).quad_to(-5.0, hy - 6.5, -1.5, hy - 4.0);
    brow.move_to(8.0, hy - 4.0).quad_to(5.0, hy - 6.5, 1.5, hy - 4.0);
    p.stroke(&brow, INK, 3.0);
    p.stroke(&brow, F_HAIR, 2.0);
    paint::eye(p, -4.5, hy - 1.0, 1.6, 1.0, INK);
    paint::eye(p, 4.5, hy - 1.0, 1.6, 1.0, INK);
    let mut lines = PathData::new();
    lines.move_to(-9.0, hy - 1.5).line_to(-7.5, hy - 0.5).move_to(9.0, hy - 1.5).line_to(7.5, hy - 0.5);
    lines.move_to(-5.0, hy + 4.0).quad_to(-6.0, hy + 7.0, -4.0, hy + 9.0);
    lines.move_to(5.0, hy + 4.0).quad_to(6.0, hy + 7.0, 4.0, hy + 9.0);
    p.stroke(&lines, INK.with_alpha(110), 0.7);
    let mut nose = PathData::new();
    nose.move_to(0.0, hy - 1.0).quad_to(2.5, hy + 4.0, -0.5, hy + 5.0);
    p.stroke(&nose, INK.with_alpha(160), 0.9);
    if let Pose::Shout(k) = pose {
        p.inked(&PathData::ellipse(0.0, hy + 8.0, 2.6, 1.2 + k as f32 * 0.8), Color::hex(0x6a2028), INK, 0.6);
    } else {
        let mut m = PathData::new();
        m.move_to(-3.0, hy + 8.0).quad_to(0.0, hy + 9.5, 3.0, hy + 8.0);
        p.stroke(&m, INK, 1.0);
    }
}

// ------------------------------------------------------ Calvin Coolidge --

const SUIT: Color = Color::hex(0x2c2e36);
const SUIT_LIGHT: Color = Color::hex(0x3c3f4a);
const C_SKIN: Color = Color::hex(0xf0cdb2);
const C_HAIR: Color = Color::hex(0xb8764a);

/// Calvin Coolidge (1872 to 1933), of Plymouth Notch, thirtieth President:
/// slight, upright, a dark three-piece suit with a watch chain, a wing
/// collar, sandy hair parted on the side, a long face and a mouth that has
/// said all it means to. `Pose::Offer` holds out a plate: maple candy and a
/// wedge of cheddar.
fn coolidge_draw(p: &mut Painter, pose: Pose) {
    paint::shadow(p, 13.0, 3.5);
    for x in [-4.5f32, 4.5] {
        limb(p, (x * 0.8, -44.0), (x, -6.0), 7.0, SUIT);
        p.inked(&PathData::ellipse(x + 1.5, -3.0, 5.5, 2.6), Color::hex(0x121214), INK, 0.8);
    }
    // Jacket, narrow and buttoned high, with the waistcoat and its chain.
    let coat = PathData::smooth_closed(&[(-12.0, -86.0), (12.0, -86.0), (14.0, -60.0), (14.0, -40.0), (0.0, -38.0), (-14.0, -40.0), (-14.0, -60.0)], 0.25);
    p.inked(&coat, SUIT, INK, 1.1);
    p.fill(&PathData::polygon(&[(-5.0, -84.0), (5.0, -84.0), (4.0, -50.0), (-4.0, -50.0)]), SUIT_LIGHT);
    let mut chain = PathData::new();
    chain.move_to(-3.5, -62.0).quad_to(0.0, -58.0, 3.5, -62.0);
    p.stroke(&chain, Color::hex(0xe0b840), 0.9);
    let mut lapel = PathData::new();
    lapel.move_to(-6.0, -86.0).line_to(-3.0, -66.0).move_to(6.0, -86.0).line_to(3.0, -66.0);
    p.stroke(&lapel, INK, 0.9);
    // The wing collar and a dark tie.
    p.inked(&PathData::polygon(&[(-5.0, -88.0), (5.0, -88.0), (4.0, -84.0), (-4.0, -84.0)]), Color::hex(0xfbfaf6), INK, 0.6);
    p.inked(&PathData::polygon(&[(-1.5, -84.0), (1.5, -84.0), (1.0, -74.0), (-1.0, -74.0)]), Color::hex(0x1a1a20), INK, 0.4);
    // Arms: at his sides, or holding out the plate.
    let offering = matches!(pose, Pose::Offer(_));
    let (l, r) = if offering { ((-6.0, -60.0), (6.0, -60.0)) } else { ((-14.0, -50.0), (14.0, -50.0)) };
    limb(p, (-11.0, -82.0), l, 6.0, SUIT);
    limb(p, (11.0, -82.0), r, 6.0, SUIT);
    if offering {
        p.inked(&PathData::ellipse(0.0, -63.0, 13.0, 3.6), Color::hex(0xf4f2ec), INK, 0.8);
        p.inked(&PathData::polygon(&[(-9.0, -64.0), (-1.0, -64.0), (-1.0, -69.0)]), Color::hex(0xf2b33a), INK, 0.6);
        for x in [3.0f32, 7.5] {
            p.inked(&PathData::ellipse(x, -65.5, 2.6, 1.8), Color::hex(0xc8782a), INK, 0.5);
        }
    }
    hand(p, l, 2.8, C_SKIN);
    hand(p, r, 2.8, C_SKIN);
    // The head: long and narrow.
    let hy = -100.0;
    p.inked(&PathData::round_rect(-3.0, hy + 9.0, 6.0, 5.0, 2.0), C_SKIN, INK, 0.8);
    p.inked(&PathData::ellipse(0.0, hy, 8.5, 12.0), C_SKIN, INK, 1.1);
    // Thin sandy hair, parted on his left and combed flat.
    let hair = PathData::smooth_closed(&[(-8.5, hy - 2.0), (-8.0, hy - 9.0), (-3.0, hy - 12.5), (4.0, hy - 12.0), (8.5, hy - 6.0), (8.6, hy - 2.0), (6.0, hy - 7.5), (-2.0, hy - 9.0), (-5.0, hy - 6.0)], 0.3);
    p.inked(&hair, C_HAIR, INK, 0.9);
    let mut part = PathData::new();
    part.move_to(-3.5, hy - 12.0).line_to(-5.5, hy - 7.0);
    p.stroke(&part, INK.with_alpha(150), 0.7);
    // Level brows, steady eyes, a long nose, and the mouth: a straight line,
    // turned down a little at the ends. It stays shut.
    let mut brow = PathData::new();
    brow.move_to(-6.0, hy - 3.5).line_to(-1.8, hy - 3.2).move_to(6.0, hy - 3.5).line_to(1.8, hy - 3.2);
    p.stroke(&brow, Color::hex(0x8a5a3a), 1.3);
    paint::eye(p, -3.6, hy - 0.5, 1.4, 1.0, INK);
    paint::eye(p, 3.6, hy - 0.5, 1.4, 1.0, INK);
    let mut nose = PathData::new();
    nose.move_to(0.0, hy - 1.0).line_to(1.4, hy + 5.0).line_to(-0.6, hy + 5.5);
    p.stroke(&nose, INK.with_alpha(160), 0.8);
    let mut m = PathData::new();
    m.move_to(-3.4, hy + 8.6).line_to(-2.4, hy + 8.0).line_to(2.4, hy + 8.0).line_to(3.4, hy + 8.6);
    p.stroke(&m, INK, 1.1);
    let mut lines = PathData::new();
    lines.move_to(-4.5, hy + 5.0).line_to(-4.0, hy + 9.0).move_to(4.5, hy + 5.0).line_to(4.0, hy + 9.0);
    p.stroke(&lines, INK.with_alpha(90), 0.6);
}

// --------------------------------------------------------------- sheets --

fn child_sheet(name: &str, tile_px: u32, vector: bool, draw: fn(&mut Painter, View, Pose)) -> super::Painted {
    let mut sb = SheetBuilder::new(name, (96.0, 120.0), (48.0, 110.0), tile_px).vector(vector);
    let views = [("down", View::Front, false), ("up", View::Back, false), ("right", View::Side, false), ("left", View::Side, true)];
    let f = |sb: &mut SheetBuilder, anim: &str, view: View, pose: Pose, left: bool| {
        sb.frame(anim, move |p| {
            if left {
                p.push(mirror());
            }
            draw(p, view, pose);
            if left {
                p.pop();
            }
        });
    };
    for (dir, view, left) in views {
        for k in 0..4 {
            let s = (k as f32 * std::f32::consts::FRAC_PI_2).sin();
            f(&mut sb, &format!("walk_{dir}"), view, Pose::Walk(s), left);
        }
        f(&mut sb, &format!("stand_{dir}"), view, Pose::Stand, left);
        for k in 0..4 {
            f(&mut sb, &format!("skip_{dir}"), view, Pose::Skip(k), left);
        }
    }
    f(&mut sb, "hold", View::Front, Pose::Hold, false);
    for (dir, left) in [("right", false), ("left", true)] {
        for k in 0..2 {
            f(&mut sb, &format!("tug_{dir}"), View::Side, Pose::Tug(k), left);
        }
    }
    for k in 0..2 {
        f(&mut sb, "cry", View::Front, Pose::Cry(k), false);
        f(&mut sb, "jump", View::Front, Pose::Jump(k), false);
        f(&mut sb, "hug", View::Side, Pose::Hug(k), true);
    }
    f(&mut sb, "sprawl", View::Front, Pose::Sprawl, false);
    f(&mut sb, "sit", View::Front, Pose::Sit, false);
    sb.finish_any()
}

/// Loki: `walk_{d}` (four), `stand_{d}`, `hold`, `tug_{left|right}`, `cry`,
/// `jump`, `hug`, `sprawl`, `sit`.
pub fn loki_sheet(tile_px: u32, vector: bool) -> super::Painted {
    child_sheet("loki", tile_px, vector, loki_draw)
}

/// Skaði: the same poses as Loki.
pub fn skadi_sheet(tile_px: u32, vector: bool) -> super::Painted {
    child_sheet("skadi", tile_px, vector, skadi_draw)
}

/// Their mother: `stand`, `offer` (two), `laugh`.
pub fn mom_sheet(tile_px: u32, vector: bool) -> super::Painted {
    let mut sb = SheetBuilder::new("mom", (112.0, 150.0), (56.0, 140.0), tile_px).vector(vector);
    sb.frame("stand", |p| mom_draw(p, Pose::Stand));
    for k in 0..2 {
        sb.frame("offer", move |p| mom_draw(p, Pose::Offer(k)));
    }
    sb.frame("laugh", |p| mom_draw(p, Pose::Laugh));
    sb.finish_any()
}

/// Ethan Allen: `stand`, `shout` (two).
pub fn ethan_sheet(tile_px: u32, vector: bool) -> super::Painted {
    let mut sb = SheetBuilder::new("ethan", (96.0, 150.0), (48.0, 140.0), tile_px).vector(vector);
    sb.frame("stand", |p| ethan_draw(p, Pose::Stand));
    for k in 0..2 {
        sb.frame("shout", move |p| ethan_draw(p, Pose::Shout(k)));
    }
    sb.finish_any()
}

/// Calvin Coolidge: `stand`, `offer`.
pub fn coolidge_sheet(tile_px: u32, vector: bool) -> super::Painted {
    let mut sb = SheetBuilder::new("coolidge", (96.0, 150.0), (48.0, 140.0), tile_px).vector(vector);
    sb.frame("stand", |p| coolidge_draw(p, Pose::Stand));
    sb.frame("offer", |p| coolidge_draw(p, Pose::Offer(0)));
    sb.finish_any()
}

/// Robert Frost: `stand`, `recite` (two).
pub fn frost_sheet(tile_px: u32, vector: bool) -> super::Painted {
    let mut sb = SheetBuilder::new("frost", (96.0, 150.0), (48.0, 140.0), tile_px).vector(vector);
    sb.frame("stand", |p| frost_draw(p, Pose::Stand));
    for k in 0..2 {
        sb.frame("recite", move |p| frost_draw(p, Pose::Shout(k)));
    }
    sb.finish_any()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_family_paints_both_ways() {
        for v in [false, true] {
            for s in [loki_sheet(48, v), skadi_sheet(48, v), mom_sheet(48, v), ethan_sheet(48, v), frost_sheet(48, v), coolidge_sheet(48, v)] {
                assert!(s.width > 0 && s.height > 0);
                assert_eq!(s.pixels.is_some(), !v);
            }
        }
    }
}
