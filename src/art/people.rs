//! People, painted procedurally: one figure painter for everybody, with
//! height, build, skin, hair, clothes, a costume and something in hand.
//!
//! These are the townsfolk, whom nobody designed: the trick-or-treaters
//! in every costume, and grown-ups in every build there is. Loki, Skaði,
//! their mother and Ethan Allen are drawn by hand in `family`.

use super::SheetBuilder;
use super::paint::{self, Painter, mirror, rot_about};
use crate::gfx::vector::{Affine, Color, PathData};
use rand::{RngExt, SeedableRng};
use rand_chacha::ChaCha8Rng;

pub const INK: Color = Color::hex(0x2e2630);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hair {
    Short,
    Long,
    Bun,
    Bald,
    Curly,
    Braids,
    Ponytail,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outfit {
    Plain,
    Flannel,
    Tweed,
    Suit,
    Jogger,
    Parka,
    Hoodie,
    Witch,
    Pirate,
    Robot,
    Cow,
    Syrup,
    Skeleton,
    Superhero,
    Pumpkin,
    Ghost,
    Astronaut,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Prop {
    None,
    Pail,
    Pillowcase,
    Camera,
    Leash,
    Platter,
    Book,
}

#[derive(Clone, Copy, Debug)]
pub struct Figure {
    /// Art units, ground to the top of the head (a tile is 64).
    pub height: f32,
    /// 1 is average; more is broader.
    pub girth: f32,
    pub skin: Color,
    pub hair: Color,
    pub style: Hair,
    pub top: Color,
    pub bottom: Color,
    pub shoes: Color,
    pub outfit: Outfit,
    pub prop: Prop,
    /// A child's proportions (bigger head).
    pub child: bool,
    pub glasses: bool,
    pub freckles: bool,
    pub beard: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum View {
    Front,
    Back,
    Side,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Pose {
    Stand,
    Walk(f32),
    /// Arms forward round something held at the chest.
    Hold,
    /// Leaning back, arms out (facing right).
    Tug(u32),
    Cry(u32),
    Jump(u32),
    /// Lying in a leaf pile, arms and legs out.
    Sprawl,
    Sit,
    /// Arms around somebody (Skaði comforting Loki), facing left.
    Hug(u32),
    /// Holding the platter up.
    Offer(u32),
    Laugh,
}

pub const SKINS: [Color; 6] = [
    Color::hex(0xf6d4b8),
    Color::hex(0xeab58e),
    Color::hex(0xc98d62),
    Color::hex(0xa06a44),
    Color::hex(0x7a4e32),
    Color::hex(0x5a3a26),
];

pub const HAIRS: [Color; 6] = [
    Color::hex(0x2a2220),
    Color::hex(0x5a3a22),
    Color::hex(0x9a6a3a),
    Color::hex(0xe0c070),
    Color::hex(0xc8501e),
    Color::hex(0xb8b4b0),
];

struct Body {
    hr: f32,
    head_y: f32,
    shoulder_y: f32,
    hip_y: f32,
    sw: f32,
    hw: f32,
}

fn body(f: &Figure) -> Body {
    let h = f.height;
    let hr = if f.child { h * 0.15 } else { h * 0.115 };
    let head_y = -h + hr;
    let shoulder_y = head_y + hr + h * 0.04;
    let hip_y = if f.child { -h * 0.40 } else { -h * 0.46 };
    let sw = h * 0.15 * f.girth;
    let hw = h * 0.14 * f.girth.powf(1.25);
    Body {
        hr,
        head_y,
        shoulder_y,
        hip_y,
        sw,
        hw,
    }
}

fn limb(p: &mut Painter, a: (f32, f32), b: (f32, f32), w: f32, c: Color) {
    let mut d = PathData::new();
    d.move_to(a.0, a.1).line_to(b.0, b.1);
    p.stroke(&d, INK, w + 1.6);
    p.stroke(&d, c, w);
}

fn torso_path(f: &Figure, b: &Body, view: View) -> PathData {
    let belly = (f.girth - 1.0).max(0.0) * f.height * 0.09;
    let s = if view == View::Side { 0.7 } else { 1.0 };
    let sw = b.sw * s;
    let hw = b.hw * s;
    let fwd = if view == View::Side { belly * 0.9 } else { 0.0 };
    let mid = (b.shoulder_y + b.hip_y) / 2.0;
    PathData::smooth_closed(
        &[
            (-sw, b.shoulder_y + 1.0),
            (sw, b.shoulder_y + 1.0),
            (sw * 1.05 + belly * 0.5 + fwd, mid),
            (hw + belly * 0.3 + fwd * 0.6, b.hip_y + 1.5),
            (-hw - belly * 0.3, b.hip_y + 1.5),
            (-sw * 1.05 - belly * 0.5, mid),
        ],
        0.55,
    )
}

fn pick_colour(f: &Figure) -> (Color, Color) {
    match f.outfit {
        Outfit::Witch => (Color::hex(0x5a2a7a), Color::hex(0x2a1a3a)),
        Outfit::Pirate => (Color::hex(0xf0ece0), Color::hex(0x2a2a40)),
        Outfit::Robot | Outfit::Astronaut => (Color::hex(0xc8ccd4), Color::hex(0xa8acb4)),
        Outfit::Cow => (Color::hex(0xf4f2ea), Color::hex(0xf4f2ea)),
        Outfit::Syrup => (Color::hex(0xc8822a), Color::hex(0xc8822a)),
        Outfit::Skeleton => (Color::hex(0x1e1e24), Color::hex(0x1e1e24)),
        Outfit::Superhero => (Color::hex(0x2a5ad0), Color::hex(0xd02a2a)),
        Outfit::Pumpkin => (Color::hex(0xe8701c), Color::hex(0x2a5a22)),
        Outfit::Ghost => (Color::hex(0xf0eef6), Color::hex(0xf0eef6)),
        Outfit::Flannel => (Color::hex(0xb02a2a), f.bottom),
        Outfit::Tweed => (Color::hex(0x7a5a3a), Color::hex(0x4a3a30)),
        Outfit::Suit => (Color::hex(0x24304a), Color::hex(0x24304a)),
        Outfit::Jogger => (Color::hex(0x2ad08a), Color::hex(0x2a2a30)),
        Outfit::Parka => (Color::hex(0xe8a02a), Color::hex(0x3a4a6a)),
        _ => (f.top, f.bottom),
    }
}

fn head(p: &mut Painter, f: &Figure, b: &Body, view: View, mood: Mood) {
    let (x, y, r) = (if view == View::Side { 1.5 } else { 0.0 }, b.head_y, b.hr);
    // Hair behind the head.
    if matches!(f.style, Hair::Long | Hair::Braids | Hair::Curly | Hair::Ponytail) && view != View::Back {
        let back = match f.style {
            Hair::Curly => paint::fluff(x, y - r * 0.1, r * 1.35, r * 1.25, 9, 0.18, 77),
            _ => PathData::round_rect(x - r * 1.05, y - r * 0.5, r * 2.1, r * 2.2, r * 0.8),
        };
        p.inked(&back, f.hair, INK, 0.9);
    }
    let face = PathData::ellipse(x, y, r * (if f.girth > 1.3 { 1.08 } else { 1.0 }), r * 1.02);
    p.inked(&face, f.skin, INK, 1.0);
    if f.girth > 1.35 && view != View::Back {
        // A soft double chin.
        let mut chin = PathData::new();
        chin.move_to(x - r * 0.5, y + r * 0.82).quad_to(x, y + r * 1.05, x + r * 0.5, y + r * 0.82);
        p.stroke(&chin, INK.with_alpha(120), 0.8);
    }
    match view {
        View::Back => {
            let cap = PathData::ellipse(x, y - r * 0.1, r * 1.04, r * 0.98);
            p.inked(&cap, f.hair, INK, 0.9);
        }
        _ => {
            let side = view == View::Side;
            let ex: &[f32] = if side { &[r * 0.45] } else { &[-r * 0.38, r * 0.38] };
            for &e in ex {
                match mood {
                    Mood::Glad => paint::happy_eye(p, x + e, y - r * 0.05, r * 0.16, INK),
                    Mood::Sad => {
                        paint::eye(p, x + e, y, r * 0.14, 0.6, INK);
                        // Tears.
                        p.fill(&PathData::ellipse(x + e, y + r * 0.35, r * 0.08, r * 0.16), Color::hex(0x8ac8f0));
                    }
                    Mood::Plain => paint::eye(p, x + e, y - r * 0.05, r * 0.14, 1.0, INK),
                }
                if f.glasses {
                    p.stroke(&PathData::circle(x + e, y - r * 0.05, r * 0.26), INK, 0.9);
                }
            }
            let mx = if side { x + r * 0.5 } else { x };
            let mut m = PathData::new();
            match mood {
                Mood::Sad => {
                    m.move_to(mx - r * 0.25, y + r * 0.55).quad_to(mx, y + r * 0.25, mx + r * 0.25, y + r * 0.55);
                    p.stroke(&m, Color::hex(0x9a3040), 1.2);
                }
                _ => {
                    m.move_to(mx - r * 0.25, y + r * 0.35).quad_to(mx, y + r * 0.6, mx + r * 0.25, y + r * 0.35).close();
                    p.inked(&m, Color::hex(0xa8384a), INK, 0.5);
                }
            }
            p.fill(&PathData::circle(x - r * 0.55 + if side { r * 0.7 } else { 0.0 }, y + r * 0.3, r * 0.17), Color::rgba(240, 120, 110, 120));
            if !side {
                p.fill(&PathData::circle(x + r * 0.55, y + r * 0.3, r * 0.17), Color::rgba(240, 120, 110, 120));
            }
            if f.freckles {
                for (fx, fy) in [(-0.3f32, 0.2f32), (-0.15, 0.28), (0.18, 0.24), (0.32, 0.18)] {
                    p.fill(&PathData::circle(x + r * fx, y + r * fy, 0.6), Color::hex(0xb07050));
                }
            }
            if f.beard {
                let beard = PathData::smooth_closed(&[(x - r * 0.8, y + r * 0.1), (x + r * 0.8, y + r * 0.1), (x + r * 0.6, y + r * 0.9), (x, y + r * 1.25), (x - r * 0.6, y + r * 0.9)], 0.7);
                p.fill(&beard, f.hair);
                let mut mm = PathData::new();
                mm.move_to(mx - r * 0.2, y + r * 0.45).line_to(mx + r * 0.2, y + r * 0.45);
                p.stroke(&mm, INK, 1.0);
            }
            // Hair on top.
            match f.style {
                Hair::Bald => {}
                Hair::Curly => {
                    let top = paint::fluff(x, y - r * 0.65, r * 1.05, r * 0.55, 7, 0.25, 31);
                    p.inked(&top, f.hair, INK, 0.8);
                }
                _ => {
                    let fringe = PathData::smooth_closed(&[(x - r * 1.02, y - r * 0.05), (x - r * 0.9, y - r * 0.85), (x, y - r * 1.12), (x + r * 0.9, y - r * 0.85), (x + r * 1.02, y - r * 0.05), (x + r * 0.2, y - r * 0.55)], 0.6);
                    p.inked(&fringe, f.hair, INK, 0.8);
                    if f.style == Hair::Bun {
                        p.inked(&PathData::circle(x, y - r * 1.15, r * 0.4), f.hair, INK, 0.8);
                    }
                }
            }
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Mood {
    Plain,
    Glad,
    Sad,
}

/// A costume's headgear, over the hair.
fn headgear(p: &mut Painter, f: &Figure, b: &Body, view: View) {
    let (x, y, r) = (if view == View::Side { 1.5 } else { 0.0 }, b.head_y, b.hr);
    match f.outfit {
        Outfit::Witch => {
            p.inked(&PathData::ellipse(x, y - r * 0.75, r * 1.6, r * 0.3), Color::hex(0x1e1428), INK, 0.8);
            let cone = PathData::polygon(&[(x - r * 0.85, y - r * 0.8), (x + r * 0.85, y - r * 0.8), (x + r * 0.6, y - r * 2.6)]);
            p.inked(&cone, Color::hex(0x1e1428), INK, 0.8);
            p.fill(&PathData::rect(x - r * 0.8, y - r * 1.05, r * 1.6, r * 0.22), Color::hex(0x8a3ad0));
        }
        Outfit::Pirate => {
            let band = PathData::smooth_closed(&[(x - r * 1.05, y - r * 0.2), (x - r * 0.9, y - r * 0.95), (x + r * 0.9, y - r * 0.95), (x + r * 1.05, y - r * 0.2)], 0.4);
            p.inked(&band, Color::hex(0xc82a2a), INK, 0.7);
            if view != View::Back {
                p.fill(&PathData::circle(x + r * 0.38, y - r * 0.05, r * 0.24), INK);
            }
        }
        Outfit::Robot => {
            let box_ = PathData::round_rect(x - r * 1.1, y - r * 1.1, r * 2.2, r * 2.1, r * 0.2);
            p.inked(&box_, Color::hex(0xb0b4bc), INK, 0.9);
            if view != View::Back {
                p.fill(&PathData::rect(x - r * 0.7, y - r * 0.3, r * 1.4, r * 0.45), Color::hex(0x2ad0f0));
            }
            let mut ant = PathData::new();
            ant.move_to(x, y - r * 1.1).line_to(x, y - r * 1.6);
            p.stroke(&ant, INK, 1.0);
            p.fill(&PathData::circle(x, y - r * 1.65, r * 0.18), Color::hex(0xf03a3a));
        }
        Outfit::Cow => {
            let hood = PathData::ellipse(x, y - r * 0.35, r * 1.12, r * 0.85);
            p.inked(&hood, Color::hex(0xf4f2ea), INK, 0.8);
            for s in [-1.0f32, 1.0] {
                p.inked(&PathData::ellipse(x + s * r * 1.15, y - r * 0.5, r * 0.4, r * 0.18), Color::hex(0xf4f2ea), INK, 0.6);
                let horn = PathData::polygon(&[(x + s * r * 0.5, y - r * 1.0), (x + s * r * 0.75, y - r * 1.0), (x + s * r * 0.75, y - r * 1.45)]);
                p.inked(&horn, Color::hex(0xe8dcb8), INK, 0.5);
            }
            p.fill(&PathData::ellipse(x - r * 0.3, y - r * 0.8, r * 0.3, r * 0.2), INK);
        }
        Outfit::Syrup => {
            p.inked(&PathData::rect(x - r * 0.6, y - r * 1.5, r * 1.2, r * 0.6), Color::hex(0xe8402a), INK, 0.7);
        }
        Outfit::Skeleton => {
            if view != View::Back {
                for e in [-0.38f32, 0.38] {
                    p.fill(&PathData::circle(x + r * e, y - r * 0.05, r * 0.22), INK.with_alpha(160));
                }
            }
        }
        Outfit::Pumpkin => {
            p.inked(&PathData::rect(x - r * 0.15, y - r * 1.35, r * 0.3, r * 0.35), Color::hex(0x2a5a22), INK, 0.6);
        }
        Outfit::Astronaut => {
            let bubble = PathData::circle(x, y, r * 1.35);
            p.fill(&bubble, Color::rgba(200, 230, 255, 70));
            p.stroke(&bubble, Color::hex(0xe8ecf0), 2.0);
            p.fill(&PathData::ellipse(x - r * 0.5, y - r * 0.6, r * 0.3, r * 0.15), Color::rgba(255, 255, 255, 160));
        }
        Outfit::Ghost => {}
        _ => {}
    }
}

fn torso_extras(p: &mut Painter, f: &Figure, b: &Body, view: View, tp: &PathData) {
    let mid = (b.shoulder_y + b.hip_y) / 2.0;
    match f.outfit {
        Outfit::Flannel => {
            let mut plaid = PathData::new();
            let mut x = -b.sw * 1.3;
            while x < b.sw * 1.3 {
                plaid.move_to(x, b.shoulder_y).line_to(x, b.hip_y);
                x += 3.0;
            }
            let mut y = b.shoulder_y + 2.0;
            while y < b.hip_y {
                plaid.move_to(-b.sw * 1.3, y).line_to(b.sw * 1.3, y);
                y += 3.0;
            }
            p.layer(1.0, |q| q.stroke(&plaid, Color::rgba(20, 20, 20, 90), 1.0));
            let _ = tp;
        }
        Outfit::Suit if view == View::Front => {
            p.fill(&PathData::polygon(&[(-2.0, b.shoulder_y + 1.0), (2.0, b.shoulder_y + 1.0), (0.0, mid + 4.0)]), Color::hex(0xf0ece4));
            p.fill(&PathData::polygon(&[(-1.0, b.shoulder_y + 2.0), (1.0, b.shoulder_y + 2.0), (1.4, mid + 2.0), (0.0, mid + 4.0), (-1.4, mid + 2.0)]), Color::hex(0xc0282a));
        }
        Outfit::Tweed if view == View::Front => {
            let mut lapel = PathData::new();
            lapel.move_to(-b.sw * 0.6, b.shoulder_y + 1.0).line_to(0.0, mid).line_to(b.sw * 0.6, b.shoulder_y + 1.0);
            p.stroke(&lapel, INK.with_alpha(160), 1.0);
        }
        Outfit::Jogger => {
            let mut stripe = PathData::new();
            stripe.move_to(-b.sw, mid).line_to(b.sw, mid);
            p.stroke(&stripe, Color::hex(0xf0f0e8), 1.8);
        }
        Outfit::Skeleton if view != View::Side => {
            let mut bones = PathData::new();
            bones.move_to(0.0, b.shoulder_y + 2.0).line_to(0.0, b.hip_y);
            for k in 0..3 {
                let y = b.shoulder_y + 4.0 + k as f32 * 4.0;
                bones.move_to(-b.sw * 0.8, y).quad_to(0.0, y - 2.0, b.sw * 0.8, y);
            }
            p.stroke(&bones, Color::hex(0xf0ece0), 1.6);
        }
        Outfit::Superhero if view == View::Front => {
            p.inked(&PathData::polygon(&[(-4.0, mid - 4.0), (4.0, mid - 4.0), (0.0, mid + 3.0)]), Color::hex(0xf2c230), INK, 0.5);
        }
        Outfit::Cow => {
            let mut spots = PathData::new();
            spots.extend(&PathData::ellipse(-b.sw * 0.4, mid - 2.0, 3.0, 2.2));
            spots.extend(&PathData::ellipse(b.sw * 0.5, mid + 3.0, 2.5, 3.0));
            p.fill(&spots, Color::hex(0x1e1e22));
        }
        Outfit::Pirate => {
            let mut st = PathData::new();
            for k in 0..4 {
                let y = b.shoulder_y + 3.0 + k as f32 * 3.0;
                st.move_to(-b.sw * 1.1, y).line_to(b.sw * 1.1, y);
            }
            p.stroke(&st, Color::hex(0xc82a2a), 1.2);
        }
        Outfit::Astronaut if view == View::Front => {
            p.fill(&PathData::rect(-3.0, mid - 3.0, 6.0, 4.0), Color::hex(0x2a5ad0));
        }
        _ => {}
    }
}

/// The whole figure in a pose, facing right (Side), at us or away.
pub fn draw(p: &mut Painter, f: &Figure, view: View, pose: Pose) {
    let b = body(f);
    let (top_c, bottom_c) = pick_colour(f);
    let lift = match pose {
        Pose::Jump(k) => -(k as f32) * f.height * 0.12,
        _ => 0.0,
    };
    let shadow = (b.hw * 1.4).max(8.0);
    paint::shadow(p, shadow, 3.5);
    if pose == Pose::Sprawl {
        p.push(Affine::translate(0.0, -2.0).then(rot_about(-90.0, 0.0, -2.0)));
        p.push(Affine::scale(1.0, 0.85));
    }
    if pose == Pose::Sit {
        p.push(Affine::translate(0.0, f.height * 0.22));
    }
    p.push(Affine::translate(0.0, lift));
    let lean = match pose {
        Pose::Tug(k) => -14.0 - k as f32 * 6.0,
        Pose::Walk(_) if f.outfit == Outfit::Jogger => 6.0,
        Pose::Laugh => -6.0,
        _ => 0.0,
    };
    p.push(rot_about(lean, 0.0, b.hip_y));
    // Behind the body: capes, tails, the jug.
    if f.outfit == Outfit::Superhero {
        let cape = PathData::polygon(&[(-b.sw, b.shoulder_y), (b.sw, b.shoulder_y), (b.sw * 1.6, -3.0), (-b.sw * 1.6, -3.0)]);
        p.inked(&cape, Color::hex(0xd02a2a), INK, 0.8);
    }
    let tail_c = match f.outfit {
        Outfit::Cow => Some((Color::hex(0xf4f2ea), 1.8)),
        _ => None,
    };
    if let Some((c, w)) = tail_c {
        let mut t = PathData::new();
        let sx = if view == View::Side { -b.hw * 0.7 } else { b.hw * 0.4 };
        t.move_to(sx, b.hip_y + 2.0).quad_to(sx - 10.0, b.hip_y + 4.0, sx - 8.0, b.hip_y - 10.0);
        p.stroke(&t, INK, w + 1.6);
        p.stroke(&t, c, w);
    }
    // Legs.
    let step = match pose {
        Pose::Walk(s) => s,
        _ => 0.0,
    };
    let leg_w = f.height * 0.07 * f.girth.sqrt();
    let leg_c = bottom_c;
    let hip = b.hip_y + 1.0;
    let (l1, l2) = match view {
        View::Side => ((-1.0 + step * 9.0, 0.0 - step.max(0.0) * 1.5), (1.0 - step * 9.0, 0.0 - (-step).max(0.0) * 1.5)),
        _ => ((-b.hw * 0.5, -(step.max(0.0)) * 2.0), (b.hw * 0.5, -((-step).max(0.0)) * 2.0)),
    };
    let ghost = f.outfit == Outfit::Ghost;
    for (lx, ly) in [l1, l2] {
        let foot = (lx, ly - 2.0);
        limb(p, (lx * 0.5, hip), foot, leg_w, leg_c);
        let boot = f.shoes;
        p.inked(&PathData::ellipse(foot.0 + if view == View::Side { 2.0 } else { 0.0 }, foot.1, leg_w * 0.85, leg_w * 0.5), boot, INK, 0.6);
    }
    // Arms, and what the hands hold.
    let hand_y = b.hip_y + 2.0;
    let swing = step * 7.0;
    let arm_w = f.height * 0.06 * f.girth.sqrt();
    let sl = (-b.sw * 0.95, b.shoulder_y + 2.5);
    let sr = (b.sw * 0.95, b.shoulder_y + 2.5);
    let (hl, hr) = match pose {
        Pose::Hold | Pose::Hug(_) => ((-b.sw * 0.4, b.shoulder_y + 10.0), (b.sw * 0.4, b.shoulder_y + 10.0)),
        Pose::Tug(_) => ((b.sw * 2.6, b.shoulder_y + 6.0), (b.sw * 2.4, b.shoulder_y + 9.0)),
        Pose::Jump(_) | Pose::Laugh => ((-b.sw * 1.8, b.shoulder_y - 10.0), (b.sw * 1.8, b.shoulder_y - 10.0)),
        Pose::Offer(k) => ((-b.sw * 0.9, b.shoulder_y + 3.0 - k as f32 * 2.0), (b.sw * 0.9, b.shoulder_y + 3.0 - k as f32 * 2.0)),
        Pose::Cry(_) => ((-b.hr * 0.3, b.head_y + b.hr * 0.4), (b.hr * 0.3, b.head_y + b.hr * 0.4)),
        Pose::Sprawl => ((-b.sw * 2.2, b.shoulder_y), (b.sw * 2.2, b.shoulder_y)),
        _ => match view {
            View::Side => ((-1.0 - swing, hand_y), (1.0 + swing, hand_y)),
            _ => ((-b.sw * 1.25, hand_y + swing * 0.3), (b.sw * 1.25, hand_y - swing * 0.3)),
        },
    };
    let arm_c = match f.outfit {
        Outfit::Ghost => Color::hex(0xf0eef6),
        _ => top_c,
    };
    if view == View::Side && !matches!(pose, Pose::Hold | Pose::Tug(_)) {
        limb(p, sl, hl, arm_w, arm_c.shade(-0.15));
    }
    // The torso (or the costume that replaces it).
    if ghost {
        let sheet = PathData::smooth_closed(
            &[(0.0, b.head_y - b.hr * 1.1), (b.hr * 1.1, b.head_y), (b.sw * 1.5, b.hip_y), (b.sw * 1.7, -6.0), (0.0, -4.0), (-b.sw * 1.7, -6.0), (-b.sw * 1.5, b.hip_y), (-b.hr * 1.1, b.head_y)],
            0.8,
        );
        p.inked(&sheet, Color::hex(0xf0eef6), INK, 1.1);
        if view != View::Back {
            for e in [-0.4f32, 0.4] {
                if view == View::Side && e < 0.0 {
                    continue;
                }
                p.fill(&PathData::ellipse(e * b.hr + if view == View::Side { b.hr * 0.3 } else { 0.0 }, b.head_y, b.hr * 0.25, b.hr * 0.3), INK);
            }
        }
    } else if f.outfit == Outfit::Pumpkin {
        let pumpkin = PathData::ellipse(0.0, (b.shoulder_y + b.hip_y) / 2.0 + 2.0, b.sw * 1.6, (b.hip_y - b.shoulder_y) * 0.65);
        p.inked(&pumpkin, Color::hex(0xe8701c), INK, 1.0);
        if view != View::Back {
            let cy = (b.shoulder_y + b.hip_y) / 2.0;
            p.fill(&PathData::polygon(&[(-5.0, cy), (-2.0, cy), (-3.5, cy - 3.0)]), INK);
            p.fill(&PathData::polygon(&[(2.0, cy), (5.0, cy), (3.5, cy - 3.0)]), INK);
            p.fill(&PathData::polygon(&[(-5.0, cy + 3.0), (5.0, cy + 3.0), (0.0, cy + 6.0)]), INK);
        }
    } else if f.outfit == Outfit::Syrup {
        let bottle = PathData::round_rect(-b.sw * 1.15, b.shoulder_y - 2.0, b.sw * 2.3, b.hip_y - b.shoulder_y + 8.0, 4.0);
        p.inked(&bottle, Color::hex(0xc8822a), INK, 1.0);
        p.fill(&PathData::rect(-b.sw * 1.0, (b.shoulder_y + b.hip_y) / 2.0 - 3.0, b.sw * 2.0, 6.0), Color::hex(0xf2ece0));
        p.fill(&PathData::rect(-b.sw * 0.7, (b.shoulder_y + b.hip_y) / 2.0 - 1.0, b.sw * 1.4, 2.0), Color::hex(0xc8302a));
    } else {
        let tp = torso_path(f, &b, view);
        p.inked(&tp, top_c, INK, 1.0);
        torso_extras(p, f, &b, view, &tp);
    }
    if !ghost {
        head(
            p,
            f,
            &b,
            view,
            match pose {
                Pose::Cry(_) => Mood::Sad,
                Pose::Jump(_) | Pose::Laugh | Pose::Hug(_) | Pose::Offer(_) => Mood::Glad,
                _ => Mood::Plain,
            },
        );
        if f.style == Hair::Braids && view != View::Back {
            for s in [-1.0f32, 1.0] {
                if view == View::Side && s > 0.0 {
                    continue;
                }
                let mut d = PathData::new();
                d.move_to(s * b.hr * 0.8, b.head_y + b.hr * 0.3).quad_to(s * b.hr * 1.3, b.head_y + b.hr * 1.4, s * b.hr * 1.1, b.head_y + b.hr * 2.6);
                p.stroke(&d, INK, 5.4);
                p.stroke(&d, f.hair, 3.8);
                p.inked(&PathData::ellipse(s * b.hr * 1.1, b.head_y + b.hr * 2.6, 2.6, 1.9), Color::hex(0xe53935), INK, 0.5);
            }
        }
        headgear(p, f, &b, view);
    }
    if view != View::Side || matches!(pose, Pose::Hold | Pose::Tug(_)) {
        limb(p, sl, hl, arm_w, arm_c);
    }
    limb(p, sr, hr, arm_w, arm_c);
    for h in [hl, hr] {
        if !(view == View::Side && h == hl && !matches!(pose, Pose::Hold | Pose::Tug(_))) {
            p.inked(&PathData::circle(h.0, h.1, arm_w * 0.6), f.skin, INK, 0.5);
        }
    }
    prop(p, f, hr, pose);
    p.pop();
    p.pop();
    if pose == Pose::Sit {
        p.pop();
    }
    if pose == Pose::Sprawl {
        p.pop();
        p.pop();
    }
}

fn prop(p: &mut Painter, f: &Figure, hand: (f32, f32), pose: Pose) {
    let (x, y) = hand;
    match f.prop {
        Prop::Pail => {
            let pail = PathData::round_rect(x - 5.0, y + 1.0, 10.0, 8.0, 3.0);
            p.inked(&pail, Color::hex(0xf07a1c), INK, 0.6);
            let mut h = PathData::new();
            h.move_to(x - 4.0, y + 1.5).quad_to(x, y - 4.0, x + 4.0, y + 1.5);
            p.stroke(&h, INK, 0.9);
            p.fill(&PathData::rect(x - 2.5, y + 3.0, 1.6, 1.4), INK);
            p.fill(&PathData::rect(x + 0.9, y + 3.0, 1.6, 1.4), INK);
        }
        Prop::Pillowcase => {
            let sack = PathData::smooth_closed(&[(x - 4.0, y), (x + 4.0, y), (x + 7.0, y + 12.0), (x, y + 15.0), (x - 7.0, y + 12.0)], 0.8);
            p.inked(&sack, Color::hex(0xf0ecf4), INK, 0.7);
        }
        Prop::Camera => {
            p.inked(&PathData::round_rect(x - 4.0, y - 3.0, 8.0, 5.5, 1.0), Color::hex(0x2a2a30), INK, 0.4);
            p.fill(&PathData::circle(x + 1.0, y - 0.3, 1.6), Color::hex(0x6a8ab0));
        }
        Prop::Leash => {
            let mut l = PathData::new();
            l.move_to(x, y).quad_to(x + 8.0, y + 6.0, x + 14.0, -4.0);
            p.stroke(&l, Color::hex(0xc02a2a), 0.9);
        }
        Prop::Platter => {
            if matches!(pose, Pose::Offer(_) | Pose::Stand | Pose::Laugh) {
                let py = y - 2.0;
                p.inked(&PathData::ellipse(-1.0, py, 13.0, 3.6), Color::hex(0xd8dce4), INK, 0.6);
                let mut cubes = PathData::new();
                for k in 0..6 {
                    cubes.extend(&PathData::rect(-9.0 + k as f32 * 3.4, py - 3.6, 2.6, 2.6));
                }
                p.fill(&cubes, Color::hex(0xe8e0c4));
                // The smell, drawn.
                let mut wav = PathData::new();
                for k in 0..3 {
                    let wx = -6.0 + k as f32 * 6.0;
                    wav.move_to(wx, py - 5.0).quad_to(wx + 2.0, py - 8.0, wx, py - 11.0).quad_to(wx - 2.0, py - 14.0, wx, py - 17.0);
                }
                p.stroke(&wav, Color::rgba(170, 200, 110, 170), 1.0);
            }
        }
        Prop::Book => {
            p.inked(&PathData::rect(x - 3.5, y - 4.0, 7.0, 9.0), Color::hex(0x7a2a3a), INK, 0.4);
        }
        Prop::None => {}
    }
}

// ---------------------------------------------------------------- who --

/// The looks townsfolk are painted in: children in every costume, and
/// grown-ups of every build. `folk_look` picks one for a walker.
pub fn looks() -> Vec<Figure> {
    let mut out = Vec::new();
    let mut rng = ChaCha8Rng::seed_from_u64(0x70_1c);
    let costumes = [
        Outfit::Witch,
        Outfit::Pirate,
        Outfit::Robot,
        Outfit::Cow,
        Outfit::Syrup,
        Outfit::Skeleton,
        Outfit::Superhero,
        Outfit::Pumpkin,
        Outfit::Ghost,
        Outfit::Astronaut,
    ];
    // 0..20: children, two of each costume.
    for (i, c) in costumes.iter().cycle().take(20).enumerate() {
        out.push(Figure {
            height: rng.random_range(50.0..66.0),
            girth: rng.random_range(0.9..1.25),
            skin: SKINS[rng.random_range(0..SKINS.len())],
            hair: HAIRS[rng.random_range(0..5)],
            style: [Hair::Short, Hair::Long, Hair::Ponytail, Hair::Curly, Hair::Braids][i % 5],
            top: Color::hex(0x6a4a8a),
            bottom: Color::hex(0x3a3a48),
            shoes: Color::hex(0x2a2a30),
            outfit: *c,
            prop: if i % 3 == 0 { Prop::Pillowcase } else { Prop::Pail },
            child: true,
            glasses: rng.random_bool(0.15),
            freckles: rng.random_bool(0.3),
            beard: false,
        });
    }
    // 20..32: grown-ups out with families and walking about, every build.
    let builds = [0.85f32, 1.0, 1.15, 1.35, 1.55, 1.75, 0.9, 1.25, 1.6, 1.05, 1.45, 1.2];
    for (i, g) in builds.iter().enumerate() {
        let tops = [Color::hex(0x8a3a3a), Color::hex(0x3a6a8a), Color::hex(0x4a6a3a), Color::hex(0x8a6a3a), Color::hex(0x5a4a7a), Color::hex(0x2a2a30)];
        out.push(Figure {
            height: rng.random_range(88.0..104.0),
            girth: *g,
            skin: SKINS[rng.random_range(0..SKINS.len())],
            hair: HAIRS[rng.random_range(0..HAIRS.len())],
            style: [Hair::Short, Hair::Long, Hair::Bun, Hair::Bald, Hair::Curly, Hair::Ponytail][i % 6],
            top: tops[i % tops.len()],
            bottom: Color::hex(0x2e3a52),
            shoes: Color::hex(0x3a2a22),
            outfit: if i % 4 == 0 { Outfit::Flannel } else { Outfit::Plain },
            prop: Prop::None,
            child: false,
            glasses: rng.random_bool(0.3),
            freckles: false,
            beard: i % 5 == 1,
        });
    }
    // 32..: particular people.
    let special = [
        (Outfit::Hoodie, Prop::Pillowcase, 80.0, 1.0, false),   // 32 teen
        (Outfit::Hoodie, Prop::Pillowcase, 84.0, 1.2, false),   // 33 teen
        (Outfit::Jogger, Prop::None, 96.0, 0.95, false),        // 34 jogger
        (Outfit::Parka, Prop::Camera, 92.0, 1.3, true),         // 35 tourist
        (Outfit::Parka, Prop::Camera, 100.0, 1.05, false),      // 36 tourist
        (Outfit::Tweed, Prop::Book, 96.0, 1.15, true),          // 37 professor
        (Outfit::Suit, Prop::None, 98.0, 1.4, true),            // 38 legislator
        (Outfit::Flannel, Prop::None, 100.0, 1.5, false),       // 39 farmer
        (Outfit::Plain, Prop::Leash, 94.0, 1.1, false),         // 40 dog walker
    ];
    for (i, (o, pr, h, g, gl)) in special.iter().enumerate() {
        out.push(Figure {
            height: *h,
            girth: *g,
            skin: SKINS[(i * 2 + 1) % SKINS.len()],
            hair: HAIRS[(i + 1) % HAIRS.len()],
            style: [Hair::Short, Hair::Ponytail, Hair::Bald, Hair::Curly, Hair::Bun][i % 5],
            top: Color::hex(0x5a5a6a),
            bottom: Color::hex(0x2e3a52),
            shoes: Color::hex(0x3a2a22),
            outfit: *o,
            prop: *pr,
            child: false,
            glasses: *gl,
            freckles: false,
            beard: i == 7,
        });
    }
    out
}

pub const LOOK_TEEN: [usize; 2] = [32, 33];
pub const LOOK_JOGGER: usize = 34;
pub const LOOK_TOURIST: [usize; 2] = [35, 36];
pub const LOOK_PROFESSOR: usize = 37;
pub const LOOK_LEGISLATOR: usize = 38;
pub const LOOK_FARMER: usize = 39;
pub const LOOK_DOGWALKER: usize = 40;
pub const ADULTS: std::ops::Range<usize> = 20..32;

fn frame(sb: &mut SheetBuilder, anim: &str, f: Figure, view: View, pose: Pose, left: bool) {
    sb.frame(anim, move |p| {
        if left {
            p.push(mirror());
        }
        draw(p, &f, view, pose);
        if left {
            p.pop();
        }
    });
}

/// How many looks there are (one sheet each).
pub fn look_count() -> usize {
    looks().len()
}

/// One townsperson's look: `walk_{d}` with three frames (step, stand,
/// step) for each direction d. Each look is its own small sheet, so only
/// the looks in view are ever sent.
pub fn look_sheet(i: usize, tile_px: u32, vector: bool) -> super::Painted {
    let all = looks();
    let f = all[i.min(all.len() - 1)];
    let mut sb = SheetBuilder::new(&format!("folk{i}"), (80.0, 120.0), (40.0, 112.0), tile_px).vector(vector);
    for (dir, view, left) in [("down", View::Front, false), ("up", View::Back, false), ("right", View::Side, false), ("left", View::Side, true)] {
        for pose in [Pose::Walk(0.8), Pose::Stand, Pose::Walk(-0.8)] {
            frame(&mut sb, &format!("walk_{dir}"), f, view, pose, left);
        }
    }
    sb.finish_any()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grown_ups_come_in_every_build() {
        let l = looks();
        let girths: Vec<f32> = l[ADULTS].iter().map(|f| f.girth).collect();
        assert!(girths.iter().any(|&g| g >= 1.5), "some big folks");
        assert!(girths.iter().any(|&g| g <= 0.9), "some slim folks");
    }
}
