//! The town map (M, or a pinch on Silkie's trackpad): Montpelier from
//! above, with its streets and their names, the river, the park and the
//! landmarks. A vector image on Silkie, drawn at whatever size it is shown;
//! rasterized for kitty.

use super::title::text_path;
use super::vector::{Affine, Color, Element, Fill, PathData, Scene, StrokeStyle};
use crate::world::plan::Town;
use crate::world::town::{AreaKind, BuildingKind, StreetKind};

/// Map units per tile.
pub const K: f32 = 4.0;

/// A label to put on the map, if there is room for it.
struct Label {
    text: String,
    /// Where it may go (x, y, angle), best first.
    spots: Vec<(f32, f32, f32)>,
    size: f32,
    colour: Color,
}

/// Places labels in order (the most important first), leaving out any that
/// would overlap one already placed.
fn place_labels(s: &mut Scene, id: &mut u32, labels: Vec<Label>) {
    let mut taken: Vec<(f32, f32, f32, f32)> = Vec::new();
    for lb in labels {
        let (d, adv) = text_path(&lb.text, lb.size);
        if adv <= 0.0 {
            continue;
        }
        let pad = lb.size * 0.15;
        let fits = lb.spots.iter().find_map(|&(x, y, angle)| {
            let (sin, cos) = angle.sin_cos();
            let t = Affine([1.0, 0.0, 0.0, 1.0, -adv / 2.0, lb.size * 0.35]).then(Affine([cos, sin, -sin, cos, x, y]));
            let placed = d.transformed(t);
            let (x0, y0, x1, y1) = placed.bounds()?;
            let b = (x0 - pad, y0 - pad, x1 + pad, y1 + pad);
            (!taken.iter().any(|o| b.0 < o.2 && o.0 < b.2 && b.1 < o.3 && o.1 < b.3)).then_some((placed, b))
        });
        let Some((placed, b)) = fits else { continue };
        taken.push(b);
        s.set(*id, Element::stroke_path(0, placed.clone(), StrokeStyle::new(Color::rgba(10, 10, 20, 200), lb.size * 0.22)));
        *id += 1;
        s.set(*id, Element::fill_path(0, placed, lb.colour));
        *id += 1;
    }
}

pub fn scene(town: &Town) -> Scene {
    let (w, h) = (town.w as f32 * K, town.h as f32 * K);
    let mut s = Scene::new(w, h);
    let mut id = 1u32;
    let mut add = |s: &mut Scene, e: Element| {
        s.set(id, e);
        id += 1;
    };
    add(&mut s, Element::fill_path(0, PathData::rect(0.0, 0.0, w, h), Color::hex(0x2a3e28)));
    for a in &town.areas {
        let c = match a.kind {
            AreaKind::Woods => Color::hex(0x1c3020),
            AreaKind::Lawn | AreaKind::Green => Color::hex(0x3a5a32),
            AreaKind::Parking | AreaKind::Plaza => Color::hex(0x3a3a42),
        };
        let pts: Vec<(f32, f32)> = a.pts.iter().map(|p| (p.x * K, p.y * K)).collect();
        add(&mut s, Element::fill_path(0, PathData::polygon(&pts), c));
    }
    for r in &town.rivers {
        let pts: Vec<(f32, f32)> = r.pts.iter().map(|p| (p.x * K, p.y * K)).collect();
        add(&mut s, Element::stroke_path(0, PathData::smooth_open(&pts, 0.5), StrokeStyle::new(Color::hex(0x2a4a7a), r.width * K)));
    }
    for st in &town.streets {
        let pts: Vec<(f32, f32)> = st.pts.iter().map(|p| (p.x * K, p.y * K)).collect();
        let (c, wd) = match st.kind {
            StreetKind::Street => (Color::hex(0x8a8890), (st.width + 1.0) * K),
            StreetKind::Lane => (Color::hex(0x7a7880), st.width * K),
            _ => (Color::hex(0x8a7458), st.width * K),
        };
        let mut style = StrokeStyle::new(c, wd);
        if matches!(st.kind, StreetKind::Path) {
            style.dash = vec![K * 1.5, K * 0.8];
        }
        add(&mut s, Element::stroke_path(0, PathData::polyline(&pts), style));
    }
    for b in &town.buildings {
        let c = match b.kind {
            BuildingKind::House => Color::hex(0xb8a888),
            BuildingKind::Home => Color::hex(0xff8ac8),
            BuildingKind::StateHouse => Color::hex(0xe8b84a),
            BuildingKind::Tower => Color::hex(0xc8c4bc),
            BuildingKind::Office => Color::hex(0x6a6a6a),
            _ => Color::hex(0xd8703a),
        };
        add(&mut s, Element::fill_path(0, PathData::rect(b.rect.x * K, b.rect.y * K, b.rect.w * K, b.rect.h * K), Fill::Solid(c)));
    }
    let mut labels = Vec::new();
    // Landmarks first, then water and woods, then streets.
    for b in &town.buildings {
        let text = match b.kind {
            BuildingKind::House | BuildingKind::Office => continue,
            BuildingKind::Home => "Loki & Skaði".to_string(),
            _ => {
                let n = b.name.strip_prefix("the ").unwrap_or(&b.name);
                n.split(" and the ").next().unwrap_or(n).to_string()
            }
        };
        let c = b.rect.center();
        labels.push(Label {
            text,
            spots: vec![(c.x * K, (b.rect.y - 0.8) * K, 0.0), (c.x * K, c.y * K, 0.0), (c.x * K, (b.rect.y1() + 1.2) * K, 0.0)],
            size: 2.4 * K,
            colour: Color::hex(0xffd08a),
        });
    }
    for r in &town.rivers {
        if let Some(p) = r.pts.get(r.pts.len() / 3) {
            labels.push(Label {
                text: r.name.clone(),
                spots: vec![(p.x * K, p.y * K, 0.0)],
                size: 2.8 * K,
                colour: Color::hex(0x9ac0f0),
            });
        }
    }
    for a in town.areas.iter().filter(|a| a.kind == AreaKind::Woods) {
        let b = crate::world::bounds_of(&a.pts, 0.0);
        labels.push(Label {
            text: a.name.clone(),
            spots: vec![(b.center().x * K, b.center().y * K, 0.0)],
            size: 3.6 * K,
            colour: Color::hex(0x8ad08a),
        });
    }
    // Street names, once each, wherever along the street there is room.
    for st in town.streets.iter().filter(|s| matches!(s.kind, StreetKind::Street | StreetKind::Lane)) {
        let len = st.length();
        let spots = [0.5f32, 0.3, 0.7, 0.15, 0.85, 0.4, 0.6]
            .iter()
            .map(|f| {
                let (p, dir) = st.at(len * f);
                let dir = if dir.x < -0.3 || (dir.x.abs() <= 0.3 && dir.y > 0.0) { -dir } else { dir };
                (p.x * K, p.y * K, dir.y.atan2(dir.x))
            })
            .collect();
        labels.push(Label {
            text: st.name.clone(),
            spots,
            size: 3.4 * K,
            colour: Color::hex(0xf2ecdc),
        });
    }
    let mut lid = id;
    place_labels(&mut s, &mut lid, labels);
    s
}
