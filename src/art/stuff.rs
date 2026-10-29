//! Small things: every kind of candy (and the disappointing things handed
//! out instead), the scraps a sheet is patched with, eggs, what hens and
//! big dogs leave behind, and what Ethan Allen gave the children.

use super::SheetBuilder;
use super::paint::{self, Painter};
use crate::gfx::vector::{Affine, Color, PathData};
use crate::sim::candy::{Candy, Mat};

const INK: Color = Color::hex(0x2a2226);

fn wrapped(p: &mut Painter, body: Color, ends: Color, stripe: Option<Color>) {
    p.inked(&PathData::round_rect(-6.0, -6.0, 12.0, 7.0, 2.0), body, INK, 0.6);
    p.inked(&PathData::polygon(&[(-6.0, -2.5), (-10.0, -6.0), (-10.0, 1.0)]), ends, INK, 0.5);
    p.inked(&PathData::polygon(&[(6.0, -2.5), (10.0, -6.0), (10.0, 1.0)]), ends, INK, 0.5);
    if let Some(s) = stripe {
        p.fill(&PathData::rect(-6.0, -3.5, 12.0, 2.0), s);
    }
}

fn candy(p: &mut Painter, c: Candy) {
    paint::shadow(p, 7.0, 2.0);
    match c {
        Candy::CandyCorn => {
            let corn = PathData::polygon(&[(-5.0, 0.0), (5.0, 0.0), (0.0, -11.0)]);
            p.inked(&corn, Color::hex(0xf6f0e0), INK, 0.6);
            p.fill(&PathData::polygon(&[(-5.0, 0.0), (5.0, 0.0), (3.6, -3.4), (-3.6, -3.4)]), Color::hex(0xf2c230));
            p.fill(&PathData::polygon(&[(-3.6, -3.4), (3.6, -3.4), (2.0, -6.8), (-2.0, -6.8)]), Color::hex(0xf07a1c));
        }
        Candy::Lollipop => {
            let mut stick = PathData::new();
            stick.move_to(0.0, 0.0).line_to(0.0, -7.0);
            p.stroke(&stick, Color::hex(0xf6f2ea), 1.4);
            p.inked(&PathData::circle(0.0, -10.0, 4.6), Color::hex(0xe02a5a), INK, 0.6);
            let mut sw = PathData::new();
            sw.move_to(-3.0, -10.0).quad_to(0.0, -14.0, 3.0, -10.0);
            p.stroke(&sw, Color::hex(0xf8d0e0), 1.0);
        }
        Candy::Chocolate => wrapped(p, Color::hex(0x5a2e1a), Color::hex(0xc8a050), Some(Color::hex(0xe0b840))),
        Candy::FullSize => {
            p.inked(&PathData::rect(-12.0, -8.0, 24.0, 8.0), Color::hex(0x6a2a1a), INK, 0.8);
            p.fill(&PathData::rect(-9.0, -6.5, 18.0, 5.0), Color::hex(0xf2c230));
            p.fill(&PathData::rect(-6.0, -5.0, 12.0, 2.0), Color::hex(0xc82a2a));
        }
        Candy::MapleLeaf => {
            let mut pts = Vec::new();
            for k in 0..10 {
                let a = -std::f32::consts::FRAC_PI_2 + k as f32 / 10.0 * std::f32::consts::TAU;
                let r = if k % 2 == 0 { 6.5 } else { 3.0 };
                pts.push((a.cos() * r, -5.0 + a.sin() * r));
            }
            p.inked(&PathData::polygon(&pts), Color::hex(0xc8823a), INK, 0.6);
        }
        Candy::SyrupJug => {
            p.inked(&PathData::round_rect(-5.0, -11.0, 10.0, 11.0, 3.0), Color::hex(0xc8822a), INK, 0.7);
            p.fill(&PathData::rect(-2.0, -14.0, 4.0, 3.0), Color::hex(0xd8302a));
            p.fill(&PathData::rect(-4.0, -7.0, 8.0, 3.0), Color::hex(0xf2ead6));
        }
        Candy::MapleCotton => {
            let mut stick = PathData::new();
            stick.move_to(0.0, 0.0).line_to(0.0, -5.0);
            p.stroke(&stick, Color::hex(0xe8dcc0), 1.2);
            let cloud = paint::fluff(0.0, -10.0, 6.0, 5.0, 7, 0.2, 5);
            p.inked(&cloud, Color::hex(0xe8c088), INK, 0.5);
        }
        Candy::GummyWorms => {
            for (k, c) in [Color::hex(0xf05a5a), Color::hex(0x5ad05a), Color::hex(0xf0c040)].into_iter().enumerate() {
                let mut w = PathData::new();
                let y = -3.0 - k as f32 * 2.5;
                w.move_to(-6.0, y).quad_to(-2.0, y - 4.0, 1.0, y).quad_to(4.0, y + 3.0, 6.0, y - 1.0);
                p.stroke(&w, INK, 3.0);
                p.stroke(&w, c, 2.0);
            }
        }
        Candy::Caramel => wrapped(p, Color::hex(0xc88a3a), Color::hex(0xf2ead6), None),
        Candy::PeanutCup => {
            p.inked(&PathData::polygon(&[(-6.0, -6.0), (6.0, -6.0), (4.5, 0.0), (-4.5, 0.0)]), Color::hex(0xe07a28), INK, 0.6);
            p.fill(&PathData::ellipse(0.0, -6.0, 6.0, 1.6), Color::hex(0x5a2e1a));
        }
        Candy::PopcornBall => {
            let ball = paint::fluff(0.0, -6.0, 6.0, 6.0, 8, 0.15, 9);
            p.inked(&ball, Color::hex(0xf6e8b0), INK, 0.6);
        }
        Candy::Raspberry => {
            let mut d = PathData::new();
            for (x, y) in [(-1.6f32, -3.0f32), (1.6, -3.0), (0.0, -5.4), (-1.6, -7.0), (1.6, -7.0), (0.0, -1.0)] {
                d.extend(&PathData::circle(x, y, 1.8));
            }
            p.inked(&d, Color::hex(0xd8204a), INK, 0.4);
        }
        Candy::Cheddar => {
            p.inked(&PathData::polygon(&[(-5.0, 0.0), (5.0, 0.0), (5.0, -6.0), (1.0, -9.0), (-5.0, -6.0)]), Color::hex(0xf2b030), INK, 0.6);
        }
        Candy::Apple => {
            p.inked(&PathData::circle(0.0, -5.5, 5.5), Color::hex(0xc8282a), INK, 0.6);
            let mut stem = PathData::new();
            stem.move_to(0.0, -10.5).line_to(1.0, -13.0);
            p.stroke(&stem, Color::hex(0x5a3a22), 1.2);
        }
        Candy::Raisins => {
            p.inked(&PathData::rect(-4.0, -10.0, 8.0, 10.0), Color::hex(0xc8282a), INK, 0.6);
            p.fill(&PathData::circle(0.0, -5.0, 2.4), Color::hex(0xf2c230));
        }
        Candy::Carob => {
            let mut d = PathData::new();
            for x in [-3.0f32, 0.0, 3.0] {
                d.extend(&PathData::circle(x, -2.0, 1.8));
            }
            p.inked(&d, Color::hex(0x6a4a30), INK, 0.4);
        }
        Candy::Toothbrush => {
            p.inked(&PathData::round_rect(-8.0, -3.5, 16.0, 3.0, 1.5), Color::hex(0x3a8ad0), INK, 0.5);
            p.fill(&PathData::rect(4.0, -6.5, 4.0, 3.0), Color::hex(0xf6f6f6));
        }
        Candy::CiderDonut => {
            let mut ring = PathData::circle(0.0, -5.5, 5.8);
            ring.extend(&PathData::circle(0.0, -5.5, 1.8));
            p.inked(&ring, Color::hex(0xc8884a), INK, 0.6);
            let mut sugar = PathData::new();
            for (x, y) in [(-3.5f32, -8.0f32), (2.5, -9.5), (4.0, -4.5), (-2.5, -2.5), (0.5, -1.5)] {
                sugar.extend(&PathData::circle(x, y, 0.6));
            }
            p.fill(&sugar, Color::hex(0xf6ead0));
        }
        Candy::Pennies => {
            for (x, y) in [(-3.0f32, -2.0f32), (3.0, -2.5), (0.0, -4.5)] {
                p.inked(&PathData::circle(x, y, 2.6), Color::hex(0xb86a3a), INK, 0.4);
            }
        }
        Candy::Hakarl => {
            p.inked(&PathData::rect(-4.5, -8.0, 9.0, 8.0), Color::hex(0xece4c8), INK, 0.6);
            let mut stink = PathData::new();
            stink.move_to(-2.0, -9.0).quad_to(0.0, -12.0, -2.0, -15.0).move_to(2.0, -9.0).quad_to(4.0, -12.0, 2.0, -15.0);
            p.stroke(&stink, Color::rgba(150, 200, 90, 200), 1.0);
        }
    }
}

fn mat(p: &mut Painter, m: Mat) {
    paint::shadow(p, 6.0, 1.6);
    p.push(Affine::translate(0.0, -4.0).then(Affine::scale(1.0, 0.7)).then(Affine::translate(0.0, 0.0)));
    super::raccoon::patch(p, m, 0.0, -2.0);
    p.pop();
}

/// Everything small, one frame each: `candy_{i}`, `mat_{i}` (by the
/// enums' order), `egg`, `egg_broken`, `poop`, `barf`, `tricorn`, `key`.
pub fn stuff_sheet(tile_px: u32, vector: bool) -> super::Painted {
    let mut sb = SheetBuilder::new("stuff", (32.0, 32.0), (16.0, 26.0), tile_px).vector(vector);
    for c in Candy::ALL {
        sb.frame(&format!("candy_{}", c as usize), move |p| candy(p, c));
    }
    for m in Mat::ALL {
        sb.frame(&format!("mat_{}", m as usize), move |p| mat(p, m));
    }
    sb.frame("egg", |p| {
        paint::shadow(p, 5.0, 1.6);
        p.inked(&PathData::ellipse(0.0, -5.0, 4.0, 5.2), Color::hex(0xf6ecd8), INK, 0.6);
        p.fill(&PathData::ellipse(-1.2, -6.5, 1.1, 1.6), Color::rgba(255, 255, 255, 180));
    });
    sb.frame("egg_broken", |p| {
        p.fill(&PathData::smooth_closed(&[(-8.0, -1.0), (-3.0, -4.0), (4.0, -3.5), (8.0, 0.0), (2.0, 2.5), (-5.0, 2.0)], 0.9), Color::rgba(250, 245, 230, 220));
        p.fill(&PathData::circle(0.0, -0.5, 2.6), Color::hex(0xf2b020));
        p.inked(&PathData::polygon(&[(-7.0, -2.0), (-4.0, -6.0), (-3.0, -2.0)]), Color::hex(0xf6ecd8), INK, 0.4);
    });
    sb.frame("poop", |p| {
        let mut d = PathData::new();
        d.extend(&PathData::ellipse(0.0, -1.5, 3.6, 1.8));
        d.extend(&PathData::ellipse(0.0, -3.5, 2.4, 1.4));
        d.extend(&PathData::circle(0.4, -5.0, 1.0));
        p.inked(&d, Color::hex(0x6a5a3a), INK, 0.4);
        p.fill(&PathData::circle(-1.0, -3.6, 0.6), Color::hex(0xf0f0e0));
    });
    sb.frame("barf", |p| {
        p.fill(&PathData::smooth_closed(&[(-11.0, 0.0), (-6.0, -4.0), (2.0, -4.5), (11.0, -1.0), (6.0, 3.0), (-4.0, 3.5)], 0.9), Color::hex(0xb8b058));
        let mut bits = PathData::new();
        for (x, y) in [(-5.0f32, -1.0f32), (1.0, -2.0), (5.0, 0.5), (-2.0, 1.5)] {
            bits.extend(&PathData::circle(x, y, 1.0));
        }
        p.fill(&bits, Color::hex(0xe07a28));
    });
    sb.frame("tricorn", |p| {
        let hat = PathData::polygon(&[(-9.0, -3.0), (0.0, -6.0), (9.0, -3.0), (6.0, -9.0), (0.0, -11.0), (-6.0, -9.0)]);
        p.inked(&hat, Color::hex(0xece6d4), INK, 0.6);
        let mut print = PathData::new();
        for k in 0..3 {
            print.move_to(-4.0, -8.0 + k as f32 * 1.6).line_to(4.0, -8.0 + k as f32 * 1.6);
        }
        p.stroke(&print, Color::rgba(40, 40, 40, 120), 0.5);
        let mut sprig = PathData::new();
        sprig.move_to(4.0, -6.0).line_to(7.0, -13.0);
        p.stroke(&sprig, Color::hex(0x2e8a4a), 1.2);
    });
    sb.frame("key", |p| {
        let mut string = PathData::new();
        string.move_to(-5.0, -14.0).quad_to(0.0, -6.0, 5.0, -14.0);
        p.stroke(&string, Color::hex(0x8a6a4a), 0.6);
        p.inked(&PathData::circle(0.0, -7.0, 2.6), Color::hex(0x6a6a72), INK, 0.4);
        p.inked(&PathData::rect(-0.8, -5.0, 1.6, 7.0), Color::hex(0x6a6a72), INK, 0.4);
        p.inked(&PathData::rect(0.8, 0.0, 2.4, 1.6), Color::hex(0x6a6a72), INK, 0.3);
    });
    sb.finish_any()
}

#[cfg(test)]
mod tests {
    #[test]
    fn small_things_paint() {
        for v in [false, true] {
            let s = super::stuff_sheet(48, v);
            assert!(s.spec.anims.contains_key("candy_18"));
            assert!(s.spec.anims.contains_key("mat_11"));
            let g = super::super::animals::gravy_sheet(48, v);
            assert!(g.spec.anims.contains_key("rubbed_left"));
            let c = super::super::animals::pets_sheet(48, v);
            assert!(c.spec.anims.contains_key("cat_hiss_right"));
        }
    }
}
