//! The town drawn as vector scenery: one description of every street,
//! river, yard, building, tree and decoration, in world units, kept as a
//! list of pieces each with its bounds. A chunk of the town (a square of
//! tiles) is the pieces that touch it: on Silkie it is sent as a vector
//! image and the terminal draws it; elsewhere the game rasterizes the same
//! pieces itself.
//!
//! Two layers: the ground, under everybody, and the canopy (tree crowns,
//! lamp heads, toilet paper in the trees), over everybody.
//!
//! What changes during the night (porch lights, berries, leaf piles,
//! pumpkins, bowls, flyers, the egg basket) are decals: each owns a fixed
//! run of element ids and is always drawn with all of them, so a change is
//! a few path records replaced in place.

use super::title::text_path;
use super::vector::{Affine, Color, Element, Fill, PathData, Scene, StrokeStyle};
use crate::world::grid::Terrain;
use crate::world::plan::{Building, Feature, Town};
use crate::world::town::{AreaKind, BuildingKind, FeatureKind, StreetKind, TreeKind};
use crate::world::{Dir, Rect};
use glam::Vec2;
use rand::{RngExt, SeedableRng};
use rand_chacha::ChaCha8Rng;

/// World units per tile.
pub const TILE: f32 = 16.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Layer {
    Ground,
    Canopy,
}

#[derive(Clone, Debug)]
pub struct Piece {
    pub id: u32,
    pub layer: Layer,
    pub elem: Element,
    /// In tiles.
    pub bounds: Rect,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DecalKind {
    PorchLight { b: usize },
    Pumpkin { f: usize },
    Berries { f: usize },
    LeafPile { f: usize },
    Tp { f: usize },
    Bowl { f: usize },
    Flyers { f: usize },
    Basket { f: usize },
    /// A flag on a pole (waving, on Silkie).
    Flag { f: usize, gmb: bool },
}

#[derive(Clone, Debug)]
pub struct DecalSlot {
    pub kind: DecalKind,
    pub at: Vec2,
    pub layer: Layer,
    /// The first of the slot's element ids.
    pub first: u32,
    pub count: u32,
    pub bounds: Rect,
    pub seed: u32,
}

/// Every decal has this many ids at most.
const DECAL_IDS: u32 = 16;

/// Ids below this are shared definitions (symbols, gradients).
const FIRST_PIECE: u32 = 100;

// Shared definitions.
pub const SYM_LEAF: u32 = 1;
pub const SYM_CIRCLE: u32 = 2;
pub const GRAD_GLOW: u32 = 10;
pub const GRAD_LAMP: u32 = 11;
pub const GRAD_DOME: u32 = 12;
pub const GRAD_WATER: u32 = 13;

#[derive(Clone, Debug)]
pub struct WorldArt {
    pub pieces: Vec<Piece>,
    /// Symbols and gradients, sent with every chunk.
    pub shared: Vec<(u32, Element)>,
    pub slots: Vec<DecalSlot>,
    /// HDR brightness in paints (Silkie with HDR images only).
    pub hdr: bool,
    pub width: f32,
    pub height: f32,
}

// ------------------------------------------------------------ palette --

const GRASS: Color = Color::hex(0x2c4429);
const YARD: Color = Color::hex(0x34502e);
const LAWN: Color = Color::hex(0x3a5a31);
const GREEN: Color = Color::hex(0x36552f);
const WOODS: Color = Color::hex(0x1f3220);
const PARKING: Color = Color::hex(0x34353d);
const PLAZA: Color = Color::hex(0x6b6560);
const ROAD: Color = Color::hex(0x2b2c34);
const SIDEWALK: Color = Color::hex(0x67656d);
const CURB: Color = Color::hex(0x55535b);
const PATH: Color = Color::hex(0x6b5b48);
const LINE: Color = Color::hex(0xa8913e);
const BANK: Color = Color::hex(0x3b3426);
const WATER: Color = Color::hex(0x1b3150);
const RIPPLE: Color = Color::hex(0x31507a);
const DECK: Color = Color::hex(0x5a4c40);
const RAIL: Color = Color::hex(0x2e2620);
const SHADOW: Color = Color::rgba(8, 10, 20, 110);
const INK: Color = Color::hex(0x15131c);
const WINDOW_LIT: Color = Color::hex(0xffcf72);
const WINDOW_DARK: Color = Color::hex(0x1c2232);
const BRICK: Color = Color::hex(0x7b3a2d);
const BRICK_DARK: Color = Color::hex(0x5c2a22);
const GRANITE: Color = Color::hex(0x85847f);
const GRANITE_LIGHT: Color = Color::hex(0xb9b7ad);

const BODIES: [Color; 8] = [
    Color::hex(0x7a2e2b),
    Color::hex(0xa8862f),
    Color::hex(0x5e7a5a),
    Color::hex(0x2f4466),
    Color::hex(0xc4c0b4),
    Color::hex(0x55667a),
    Color::hex(0x5f3d5a),
    Color::hex(0x2f6b6b),
];
const ROOFS: [Color; 5] = [
    Color::hex(0x3b3f48),
    Color::hex(0x2f4a3a),
    Color::hex(0x6a2a2a),
    Color::hex(0x4a3a30),
    Color::hex(0x26262e),
];
const TRIMS: [Color; 4] = [Color::hex(0xe8e4d8), Color::hex(0xefe1b8), Color::hex(0x1e1e22), Color::hex(0x2c4a34)];

/// Autumn: a maple in October is any of these.
const FOLIAGE: [Color; 6] = [
    Color::hex(0xc8381e),
    Color::hex(0xe0632a),
    Color::hex(0xe8a12c),
    Color::hex(0xb02a3a),
    Color::hex(0xd4b23a),
    Color::hex(0x9c3a1c),
];
const BARK: Color = Color::hex(0x4b3e39);
const PINE: Color = Color::hex(0x1e3b2a);
const PINE_LIGHT: Color = Color::hex(0x2f5a3c);

fn u(v: f32) -> f32 {
    v * TILE
}

fn p(v: Vec2) -> (f32, f32) {
    (v.x * TILE, v.y * TILE)
}

struct Builder {
    pieces: Vec<Piece>,
    slots: Vec<DecalSlot>,
    next: u32,
    hdr: bool,
}

impl Builder {
    fn add(&mut self, layer: Layer, elem: Element) {
        let bounds = elem_bounds(&elem).unwrap_or(Rect::new(0.0, 0.0, 0.0, 0.0));
        let id = self.next;
        self.next += 1;
        self.pieces.push(Piece { id, layer, elem, bounds });
    }

    fn fill(&mut self, layer: Layer, d: PathData, c: Color) {
        self.add(layer, Element::fill_path(0, d, c));
    }

    fn stroke(&mut self, layer: Layer, d: PathData, c: Color, w: f32) {
        self.add(layer, Element::stroke_path(0, d, StrokeStyle::new(c, w)));
    }

    fn decal(&mut self, kind: DecalKind, at: Vec2, layer: Layer, r: f32, seed: u32) {
        let first = self.next;
        self.next += DECAL_IDS;
        self.slots.push(DecalSlot {
            kind,
            at,
            layer,
            first,
            count: DECAL_IDS,
            bounds: Rect::new(at.x - r, at.y - r, 2.0 * r, 2.0 * r),
            seed,
        });
    }

    /// A colour, brighter than SDR white where the terminal shows HDR.
    fn glow(&self, c: Color, pct: u16) -> Color {
        if self.hdr { c.with_bright(pct) } else { c }
    }
}

/// Bounds of an element in tiles (strokes padded by their width).
fn elem_bounds(e: &Element) -> Option<Rect> {
    let (d, pad, t) = match e {
        Element::Path { d, stroke, t, .. } => (d.clone(), stroke.as_ref().map_or(0.0, |s| s.width / 2.0), *t),
        Element::Use { t, .. } => {
            // Uses here are of unit symbols: the transform's translation
            // and scale give the extent.
            let t = t.unwrap_or(Affine([1.0, 0.0, 0.0, 1.0, 0.0, 0.0]));
            let s = (t.0[0].abs() + t.0[2].abs()).max(t.0[1].abs() + t.0[3].abs());
            return Some(Rect::new((t.0[4] - s) / TILE, (t.0[5] - s) / TILE, 2.0 * s / TILE, 2.0 * s / TILE));
        }
        _ => return None,
    };
    let d = match t {
        Some(t) => d.transformed(t),
        None => d,
    };
    let (x0, y0, x1, y1) = d.bounds()?;
    Some(Rect::new((x0 - pad) / TILE, (y0 - pad) / TILE, (x1 - x0 + 2.0 * pad) / TILE, (y1 - y0 + 2.0 * pad) / TILE))
}

fn seeded(seed: u64) -> ChaCha8Rng {
    ChaCha8Rng::seed_from_u64(seed)
}

/// A maple leaf, about a unit across, centred on the origin.
fn leaf_symbol() -> PathData {
    let mut pts = Vec::new();
    let n = 10;
    for i in 0..n {
        let a = -std::f32::consts::FRAC_PI_2 + i as f32 / n as f32 * std::f32::consts::TAU;
        let r = if i % 2 == 0 { 0.55 } else { 0.24 };
        pts.push((a.cos() * r, a.sin() * r));
    }
    PathData::polygon(&pts)
}

fn shared(hdr: bool) -> Vec<(u32, Element)> {
    let bright = |c: Color, pct: u16| if hdr { c.with_bright(pct) } else { c };
    vec![
        (SYM_LEAF, Element::Symbol { d: leaf_symbol() }),
        (SYM_CIRCLE, Element::Symbol { d: PathData::circle(0.0, 0.0, 1.0) }),
        (
            GRAD_GLOW,
            Element::Radial {
                cx: 0.0,
                cy: 0.0,
                r: 1.0,
                fx: 0.0,
                fy: 0.0,
                stops: vec![
                    (0.0, Color::rgba(255, 200, 110, 120)),
                    (0.45, Color::rgba(255, 180, 90, 50)),
                    (1.0, Color::rgba(255, 170, 80, 0)),
                ],
            },
        ),
        (
            GRAD_LAMP,
            Element::Radial {
                cx: 0.0,
                cy: 0.0,
                r: 1.0,
                fx: 0.0,
                fy: 0.0,
                stops: vec![(0.0, bright(Color::rgb(255, 250, 225), 260)), (0.5, Color::rgb(255, 214, 140)), (1.0, Color::rgba(255, 190, 100, 0))],
            },
        ),
        (
            GRAD_DOME,
            Element::Radial {
                cx: -0.35,
                cy: -0.35,
                r: 1.25,
                fx: -0.35,
                fy: -0.35,
                stops: vec![(0.0, bright(Color::hex(0xfff2b8), 220)), (0.35, Color::hex(0xe8b84a)), (1.0, Color::hex(0x8a5f18))],
            },
        ),
        (
            GRAD_WATER,
            Element::Radial {
                cx: 0.0,
                cy: 0.0,
                r: 1.0,
                fx: 0.0,
                fy: 0.0,
                stops: vec![(0.0, Color::rgba(200, 210, 240, 90)), (1.0, Color::rgba(160, 180, 230, 0))],
            },
        ),
    ]
}

/// Builds the whole town's scenery.
pub fn build(town: &Town, hdr: bool) -> WorldArt {
    let mut b = Builder {
        pieces: Vec::new(),
        slots: Vec::new(),
        next: FIRST_PIECE,
        hdr,
    };
    let (w, h) = (town.w as f32, town.h as f32);
    b.fill(Layer::Ground, PathData::rect(0.0, 0.0, u(w), u(h)), GRASS);
    areas(&mut b, town);
    yards(&mut b, town);
    rivers(&mut b, town);
    streets(&mut b, town);
    walkways(&mut b, town);
    leaf_litter(&mut b, town);
    for bd in &town.buildings {
        building(&mut b, town, bd);
    }
    for f in &town.features {
        feature(&mut b, town, f);
    }
    for bd in &town.buildings {
        if town.buildings[bd.id].kind.answers() {
            let side = Vec2::new(-bd.door.facing.vec().y, bd.door.facing.vec().x);
            let at = bd.door.at + bd.door.facing.vec() * 0.25 + side * 0.55;
            b.decal(DecalKind::PorchLight { b: bd.id }, at, Layer::Ground, 2.0, bd.seed as u32);
        }
    }
    street_labels(&mut b, town);
    canopy(&mut b, town);
    WorldArt {
        pieces: b.pieces,
        shared: shared(hdr),
        slots: b.slots,
        hdr,
        width: w,
        height: h,
    }
}

fn areas(b: &mut Builder, town: &Town) {
    for a in &town.areas {
        let pts: Vec<(f32, f32)> = a.pts.iter().map(|&v| p(v)).collect();
        let c = match a.kind {
            AreaKind::Woods => WOODS,
            AreaKind::Lawn => LAWN,
            AreaKind::Green => GREEN,
            AreaKind::Parking => PARKING,
            AreaKind::Plaza => PLAZA,
        };
        b.fill(Layer::Ground, PathData::polygon(&pts), c);
        if a.kind == AreaKind::Parking {
            // Painted bays.
            let r = crate::world::bounds_of(&a.pts, 0.0);
            let mut d = PathData::new();
            let mut x = r.x + 1.2;
            while x < r.x1() - 0.5 {
                d.move_to(u(x), u(r.y + 0.3)).line_to(u(x), u(r.y + 2.2));
                d.move_to(u(x), u(r.y1() - 0.3)).line_to(u(x), u(r.y1() - 2.2));
                x += 1.3;
            }
            b.stroke(Layer::Ground, d, Color::rgba(200, 200, 190, 90), 0.8);
        }
    }
}

fn yards(b: &mut Builder, town: &Town) {
    for bd in town.buildings.iter().filter(|bd| matches!(bd.kind, BuildingKind::House | BuildingKind::Home)) {
        let r = bd.rect.grow(1.9);
        b.fill(Layer::Ground, PathData::round_rect(u(r.x), u(r.y), u(r.w), u(r.h), u(1.2)), YARD);
    }
}

fn rivers(b: &mut Builder, town: &Town) {
    for r in &town.rivers {
        let pts: Vec<(f32, f32)> = r.pts.iter().map(|&v| p(v)).collect();
        let line = PathData::smooth_open(&pts, 0.5);
        b.stroke(Layer::Ground, line.clone(), BANK, u(r.width + 1.4));
        b.stroke(Layer::Ground, line.clone(), WATER, u(r.width));
        // Ripples, and the moon on the water.
        let mut rng = seeded(r.name.len() as u64 * 77 + 5);
        let mut ripples = PathData::new();
        let len: f32 = r.pts.windows(2).map(|w| w[0].distance(w[1])).sum();
        let mut s = 1.0;
        while s < len {
            let (at, dir) = along(&r.pts, s);
            let n = Vec2::new(-dir.y, dir.x);
            let off = n * rng.random_range(-r.width * 0.35..r.width * 0.35);
            let a = at + off;
            let l = rng.random_range(0.6..1.6);
            let (x0, y0) = p(a);
            let (x1, y1) = p(a + dir * l);
            ripples.move_to(x0, y0).line_to(x1, y1);
            s += rng.random_range(0.8..2.2);
        }
        b.stroke(Layer::Ground, ripples, RIPPLE, 0.8);
    }
}

/// The point `s` tiles along a polyline, and the direction there.
fn along(pts: &[Vec2], s: f32) -> (Vec2, Vec2) {
    let mut left = s;
    for w in pts.windows(2) {
        let len = w[0].distance(w[1]);
        let dir = (w[1] - w[0]).normalize_or_zero();
        if left <= len {
            return (w[0] + dir * left, dir);
        }
        left -= len;
    }
    (pts.last().copied().unwrap_or(Vec2::ZERO), Vec2::X)
}

fn streets(b: &mut Builder, town: &Town) {
    let line = |s: &crate::world::town::Street| PathData::polyline(&s.pts.iter().map(|&v| p(v)).collect::<Vec<_>>());
    for s in town.streets.iter().filter(|s| s.kind == StreetKind::Street) {
        b.stroke(Layer::Ground, line(s), CURB, u(s.width + 2.0 * s.sidewalk() + 0.1));
        b.stroke(Layer::Ground, line(s), SIDEWALK, u(s.width + 2.0 * s.sidewalk() - 0.15));
    }
    for s in town.streets.iter().filter(|s| matches!(s.kind, StreetKind::Path)) {
        b.stroke(Layer::Ground, line(s), PATH, u(s.width));
    }
    for s in town.streets.iter().filter(|s| matches!(s.kind, StreetKind::Street | StreetKind::Lane)) {
        b.stroke(Layer::Ground, line(s), ROAD, u(s.width));
    }
    for s in town.streets.iter().filter(|s| s.kind == StreetKind::Street) {
        let mut st = StrokeStyle::new(LINE, 0.9);
        st.dash = vec![u(1.0), u(1.2)];
        b.add(Layer::Ground, Element::stroke_path(0, line(s), st));
    }
    // Bridges: wherever a way crosses water, a deck with rails.
    for s in &town.streets {
        let len = s.length();
        let mut on = false;
        let mut start = 0.0;
        let mut d = 0.0;
        let step = 0.25;
        while d <= len + step {
            let (pt, _) = s.at(d.min(len));
            let wet = town.rivers.iter().any(|r| crate::world::polyline_distance(pt, &r.pts) <= r.width / 2.0 + 0.6);
            if wet && !on {
                on = true;
                start = d;
            } else if (!wet || d > len) && on {
                on = false;
                bridge(b, s, start - 0.4, d.min(len) + 0.4);
            }
            d += step;
        }
    }
}

fn bridge(b: &mut Builder, s: &crate::world::town::Street, from: f32, to: f32) {
    let (a, dir) = s.at(from.max(0.0));
    let (z, _) = s.at(to);
    let n = Vec2::new(-dir.y, dir.x);
    let half = if s.kind == StreetKind::Street { s.half_span() } else { s.width / 2.0 + 0.2 };
    let deck = PathData::polygon(&[p(a + n * half), p(z + n * half), p(z - n * half), p(a - n * half)]);
    b.fill(Layer::Ground, deck, if s.kind == StreetKind::Street { SIDEWALK } else { DECK });
    if s.kind != StreetKind::Path && s.kind != StreetKind::Footbridge {
        let road = PathData::polygon(&[p(a + n * s.width / 2.0), p(z + n * s.width / 2.0), p(z - n * s.width / 2.0), p(a - n * s.width / 2.0)]);
        b.fill(Layer::Ground, road, ROAD);
    }
    let mut rails = PathData::new();
    for k in [half, -half] {
        let (x0, y0) = p(a + n * k);
        let (x1, y1) = p(z + n * k);
        rails.move_to(x0, y0).line_to(x1, y1);
    }
    b.stroke(Layer::Ground, rails, RAIL, u(0.22));
}

fn walkways(b: &mut Builder, town: &Town) {
    // The grid knows which tiles are walkways: draw them in runs.
    let g = &town.grid;
    let mut d = PathData::new();
    for y in 0..g.h {
        let mut x = 0;
        while x < g.w {
            if g.get(x, y) == Terrain::Path && !town.streets.iter().any(|s| crate::world::polyline_distance(crate::world::grid::centre(x, y), &s.pts) <= s.half_span() + 0.3) {
                let x0 = x;
                while x < g.w && g.get(x, y) == Terrain::Path {
                    x += 1;
                }
                d.extend(&PathData::rect(u(x0 as f32 + 0.15), u(y as f32), u((x - x0) as f32 - 0.3), u(1.0)));
            } else {
                x += 1;
            }
        }
    }
    if !d.is_empty() {
        b.fill(Layer::Ground, d, Color::hex(0x77736c));
    }
}

/// Leaves everywhere: it is October in Vermont (well, Halloween).
fn leaf_litter(b: &mut Builder, town: &Town) {
    let mut rng = seeded(0x1eaf);
    for f in &town.features {
        let FeatureKind::Tree(kind) = f.kind else { continue };
        if kind == TreeKind::Pine {
            continue;
        }
        let n = rng.random_range(4..9);
        for _ in 0..n {
            let a: f32 = rng.random_range(0.0..std::f32::consts::TAU);
            let r: f32 = rng.random_range(0.3..1.9);
            let at = f.pos + Vec2::new(a.cos(), a.sin()) * r;
            if !matches!(town.grid.at(at), Terrain::Grass | Terrain::Yard | Terrain::Lawn | Terrain::Green | Terrain::Woods | Terrain::Sidewalk) {
                continue;
            }
            let c = FOLIAGE[rng.random_range(0..FOLIAGE.len())].shade(-0.25);
            let s = u(rng.random_range(0.18..0.3));
            let rot: f32 = rng.random_range(0.0..std::f32::consts::TAU);
            let (x, y) = p(at);
            let t = Affine([s * rot.cos(), s * rot.sin(), -s * rot.sin(), s * rot.cos(), x, y]);
            b.add(
                Layer::Ground,
                Element::Use {
                    group: 0,
                    symbol: SYM_LEAF,
                    t: Some(t),
                    fill: Fill::Solid(c),
                },
            );
        }
    }
}

// ------------------------------------------------------------ buildings --

/// The depth of the front wall seen at the bottom of a building.
fn facade_depth(bd: &Building) -> f32 {
    match bd.kind {
        BuildingKind::House | BuildingKind::Home => 0.75 + 0.35 * bd.style.stories as f32,
        BuildingKind::StateHouse | BuildingKind::College => 2.0,
        BuildingKind::Tower => 1.4,
        _ if bd.kind.is_commercial() => 0.9 + 0.35 * bd.style.stories as f32,
        _ => 1.5,
    }
    .min(bd.rect.h * 0.45)
}

fn rect_path(r: &Rect) -> PathData {
    PathData::rect(u(r.x), u(r.y), u(r.w), u(r.h))
}

fn building(b: &mut Builder, town: &Town, bd: &Building) {
    let r = bd.rect;
    let mut rng = seeded(bd.seed);
    // Shadow, off to the south-east.
    b.fill(Layer::Ground, PathData::rect(u(r.x + 0.35), u(r.y + 0.4), u(r.w), u(r.h)), SHADOW);
    let fd = facade_depth(bd);
    let roof = Rect::new(r.x, r.y, r.w, r.h - fd);
    let front = Rect::new(r.x, r.y1() - fd, r.w, fd);
    let (wall, roof_c, trim) = match bd.kind {
        BuildingKind::House | BuildingKind::Home => (
            BODIES[bd.style.body as usize % BODIES.len()],
            ROOFS[bd.style.roof as usize % ROOFS.len()],
            TRIMS[bd.style.trim as usize % TRIMS.len()],
        ),
        BuildingKind::StateHouse | BuildingKind::Library | BuildingKind::Court | BuildingKind::Tower | BuildingKind::College => {
            (GRANITE, Color::hex(0x4a4c52), GRANITE_LIGHT)
        }
        BuildingKind::Church => (Color::hex(0xd8d2c4), Color::hex(0x3a3c44), Color::hex(0xf0ece2)),
        BuildingKind::Museum => (Color::hex(0xc9bfa8), Color::hex(0x5a3a30), Color::hex(0xf2ead8)),
        BuildingKind::Office => (Color::hex(0x6e6a64), Color::hex(0x3a3a40), Color::hex(0x8a8680)),
        _ => (if rng.random_bool(0.75) { BRICK } else { Color::hex(0x8a6a4a) }, Color::hex(0x2f2c30), Color::hex(0xd9cfb8)),
    };
    // The roof, from above.
    match bd.kind {
        BuildingKind::House | BuildingKind::Home => gable_roof(b, &roof, roof_c, r.w >= r.h),
        BuildingKind::Church => {
            gable_roof(b, &roof, roof_c, r.w < r.h);
        }
        BuildingKind::Tower => {
            b.fill(Layer::Ground, rect_path(&roof), GRANITE);
            let mut cren = PathData::new();
            let mut x = roof.x + 0.25;
            while x < roof.x1() - 0.3 {
                cren.extend(&PathData::rect(u(x), u(roof.y), u(0.35), u(0.35)));
                x += 0.7;
            }
            b.fill(Layer::Ground, cren, GRANITE_LIGHT);
        }
        _ => {
            // A flat roof with a parapet.
            b.fill(Layer::Ground, rect_path(&roof), roof_c);
            b.add(
                Layer::Ground,
                Element::stroke_path(0, rect_path(&roof.grow(-0.15)), StrokeStyle::new(roof_c.shade(0.12), u(0.18))),
            );
            // Rooftop clutter.
            for _ in 0..rng.random_range(1..4) {
                let x = rng.random_range(roof.x + 0.5..(roof.x1() - 1.2).max(roof.x + 0.6));
                let y = rng.random_range(roof.y + 0.4..(roof.y1() - 1.0).max(roof.y + 0.5));
                b.fill(Layer::Ground, PathData::rect(u(x), u(y), u(0.7), u(0.5)), roof_c.shade(0.2));
            }
        }
    }
    // The front wall, seen from the south.
    b.fill(Layer::Ground, rect_path(&front), wall);
    if matches!(wall, BRICK) || wall == Color::hex(0x8a6a4a) {
        let mut mortar = PathData::new();
        let mut y = front.y + 0.25;
        while y < front.y1() {
            mortar.move_to(u(front.x), u(y)).line_to(u(front.x1()), u(y));
            y += 0.25;
        }
        b.stroke(Layer::Ground, mortar, BRICK_DARK, 0.5);
    }
    b.stroke(Layer::Ground, PathData::polyline(&[(u(front.x), u(front.y)), (u(front.x1()), u(front.y))]), trim, u(0.12));
    windows(b, bd, &front, &mut rng);
    match bd.kind {
        BuildingKind::StateHouse => state_house(b, bd, &roof, &front),
        BuildingKind::College => college_tower(b, &roof),
        BuildingKind::Church => steeple(b, bd, &roof),
        BuildingKind::Library | BuildingKind::Court | BuildingKind::Museum => columns(b, bd, &front, 4),
        _ => {}
    }
    if bd.style.turret && matches!(bd.kind, BuildingKind::House) {
        let c = Vec2::new(r.x + 0.8, r.y + 0.8);
        let (x, y) = p(c);
        b.fill(Layer::Ground, PathData::circle(x, y, u(0.85)), roof_c.shade(-0.2));
        b.fill(Layer::Ground, PathData::circle(x - u(0.15), y - u(0.15), u(0.5)), roof_c.shade(0.15));
    }
    if matches!(bd.kind, BuildingKind::House | BuildingKind::Home) && rng.random_bool(0.6) {
        // A chimney.
        let x = rng.random_range(roof.x + 0.4..roof.x1() - 0.9);
        b.fill(Layer::Ground, PathData::rect(u(x), u(roof.y + 0.3), u(0.5), u(0.6)), Color::hex(0x6a3328));
    }
    door(b, bd, &front, wall, trim);
    if bd.kind != BuildingKind::House {
        sign(b, bd, &roof);
    }
    let _ = town;
}

fn gable_roof(b: &mut Builder, r: &Rect, c: Color, ridge_along_x: bool) {
    if ridge_along_x {
        let mid = r.y + r.h / 2.0;
        b.fill(Layer::Ground, PathData::rect(u(r.x), u(r.y), u(r.w), u(mid - r.y)), c.shade(0.12));
        b.fill(Layer::Ground, PathData::rect(u(r.x), u(mid), u(r.w), u(r.y1() - mid)), c.shade(-0.12));
        b.stroke(Layer::Ground, PathData::polyline(&[(u(r.x), u(mid)), (u(r.x1()), u(mid))]), c.shade(0.3), u(0.1));
    } else {
        let mid = r.x + r.w / 2.0;
        b.fill(Layer::Ground, PathData::rect(u(r.x), u(r.y), u(mid - r.x), u(r.h)), c.shade(0.12));
        b.fill(Layer::Ground, PathData::rect(u(mid), u(r.y), u(r.x1() - mid), u(r.h)), c.shade(-0.12));
        b.stroke(Layer::Ground, PathData::polyline(&[(u(mid), u(r.y)), (u(mid), u(r.y1()))]), c.shade(0.3), u(0.1));
    }
}

fn windows(b: &mut Builder, bd: &Building, front: &Rect, rng: &mut ChaCha8Rng) {
    if bd.kind == BuildingKind::Tower {
        return;
    }
    let stories = if bd.kind.is_commercial() || matches!(bd.kind, BuildingKind::House | BuildingKind::Home) {
        bd.style.stories.max(1) as usize
    } else {
        2
    };
    let row_h = front.h / stories as f32;
    let n = ((front.w / 1.2).floor() as usize).max(1);
    let gap = front.w / n as f32;
    let mut lit = PathData::new();
    let mut dark = PathData::new();
    let office = bd.kind == BuildingKind::Office;
    for s in 0..stories {
        for k in 0..n {
            let x = front.x + gap * (k as f32 + 0.5) - 0.22;
            let y = front.y + row_h * s as f32 + row_h * 0.22;
            let w = rect_path(&Rect::new(x, y, 0.44, (row_h * 0.55).min(0.7)));
            if !office && rng.random_bool(0.55) {
                lit.extend(&w);
            } else {
                dark.extend(&w);
            }
        }
    }
    if !lit.is_empty() {
        let c = b.glow(WINDOW_LIT, 130);
        b.fill(Layer::Ground, lit, c);
    }
    if !dark.is_empty() {
        b.fill(Layer::Ground, dark, WINDOW_DARK);
    }
}

fn door(b: &mut Builder, bd: &Building, front: &Rect, wall: Color, trim: Color) {
    let d = bd.door;
    match d.facing {
        Dir::S => {
            let x = d.at.x - 0.32;
            let h = (front.h * 0.6).min(0.95);
            let door = Rect::new(x, front.y1() - h, 0.64, h);
            b.fill(Layer::Ground, rect_path(&door.grow(0.06)), trim);
            b.fill(Layer::Ground, rect_path(&door), wall.shade(-0.45));
            if bd.style.porch && matches!(bd.kind, BuildingKind::House | BuildingKind::Home) {
                // A porch across the front: floor, posts, steps.
                let porch = Rect::new(front.x + 0.2, front.y1(), front.w - 0.4, 0.55);
                b.fill(Layer::Ground, rect_path(&porch), Color::hex(0x5a4a3e));
                let mut posts = PathData::new();
                let mut px = porch.x;
                while px <= porch.x1() + 0.01 {
                    posts.extend(&PathData::rect(u(px - 0.06), u(front.y), u(0.12), u(front.h + 0.5)));
                    px += (porch.w / 4.0).max(0.8);
                }
                b.fill(Layer::Ground, posts, trim);
            }
            b.fill(Layer::Ground, PathData::rect(u(d.at.x - 0.45), u(front.y1()), u(0.9), u(0.35)), Color::hex(0x7a7570));
        }
        _ => {
            // A stoop outside a side or back door, and the door in the wall.
            let out = d.facing.vec();
            let side = Vec2::new(-out.y, out.x);
            let a = d.at + side * 0.45;
            let z = d.at - side * 0.45 + out * 0.5;
            let stoop = Rect::new(a.x.min(z.x), a.y.min(z.y), (a.x - z.x).abs().max(0.3), (a.y - z.y).abs().max(0.3));
            b.fill(Layer::Ground, rect_path(&stoop), Color::hex(0x7a7570));
            let inner = d.at - out * 0.18;
            let (x, y) = p(inner);
            let (hw, hh) = if out.x != 0.0 { (u(0.18), u(0.36)) } else { (u(0.36), u(0.18)) };
            b.fill(Layer::Ground, PathData::rect(x - hw, y - hh, hw * 2.0, hh * 2.0), wall.shade(-0.5));
            if bd.kind.is_commercial() {
                // An awning.
                let aw = d.at + out * 0.3;
                let (ax, ay) = p(aw);
                let (w2, h2) = if out.x != 0.0 { (u(0.35), u(1.4)) } else { (u(1.4), u(0.35)) };
                let stripes = [Color::hex(0x7a1f2a), Color::hex(0xe8d8b8)];
                b.fill(Layer::Ground, PathData::rect(ax - w2, ay - h2, w2 * 2.0, h2 * 2.0), stripes[(bd.seed % 2) as usize]);
            }
        }
    }
}

/// The name of a place, across its roof on a sign board.
fn sign(b: &mut Builder, bd: &Building, roof: &Rect) {
    if bd.kind == BuildingKind::Tower {
        return;
    }
    let name = short_name(&bd.name);
    let size = u(0.62);
    let (path, adv) = text_path(&name, size);
    if adv <= 0.0 {
        return;
    }
    let max_w = u(roof.w - 0.6);
    let scale = (max_w / adv).min(1.0).min(u(roof.h * 0.55) / size);
    let w = adv * scale;
    let h = size * scale;
    let c = roof.center();
    let (cx, cy) = p(c);
    let board = PathData::round_rect(cx - w / 2.0 - u(0.18), cy - h * 0.75 - u(0.12), w + u(0.36), h * 0.95 + u(0.24), u(0.12));
    b.fill(Layer::Ground, board, Color::rgba(18, 16, 24, 210));
    let t = Affine([scale, 0.0, 0.0, scale, cx - w / 2.0, cy + h * 0.12]);
    let ink = match bd.kind {
        BuildingKind::StateHouse => Color::hex(0xf2d27a),
        BuildingKind::FireHouse => Color::hex(0xff8a7a),
        BuildingKind::Home => Color::hex(0xffb07a),
        _ if bd.kind.is_commercial() => Color::hex(0xfff0c8),
        _ => Color::hex(0xe8e4dc),
    };
    let ink = b.glow(ink, 115);
    b.add(
        Layer::Ground,
        Element::Path {
            group: 0,
            d: path,
            fill: Fill::Solid(ink),
            even_odd: false,
            stroke: None,
            t: Some(t),
        },
    );
}

/// "the Kellogg-Hubbard Library" on a sign is "Kellogg-Hubbard Library".
fn short_name(name: &str) -> String {
    let n = name.strip_prefix("the ").unwrap_or(name);
    let n = n.split(" and the ").next().unwrap_or(n);
    n.to_string()
}

fn columns(b: &mut Builder, bd: &Building, front: &Rect, n: usize) {
    if bd.door.facing != Dir::S {
        return;
    }
    let w = (front.w * 0.4).max(2.0).min(front.w);
    let x0 = bd.door.at.x - w / 2.0;
    let mut d = PathData::new();
    for k in 0..n {
        let x = x0 + w * (k as f32 + 0.5) / n as f32 - 0.12;
        d.extend(&PathData::rect(u(x), u(front.y + 0.1), u(0.24), u(front.h - 0.1)));
    }
    b.fill(Layer::Ground, PathData::rect(u(x0 - 0.2), u(front.y - 0.3), u(w + 0.4), u(0.5)), GRANITE_LIGHT);
    b.fill(Layer::Ground, d, Color::hex(0xe6e2d6));
}

/// The State House: granite portico with six columns, and the gold dome,
/// with Ceres on top.
fn state_house(b: &mut Builder, bd: &Building, roof: &Rect, front: &Rect) {
    columns(b, bd, front, 6);
    // A pediment over the portico.
    let w = front.w * 0.4 + 0.4;
    let x0 = bd.door.at.x - w / 2.0;
    let tri = PathData::polygon(&[(u(x0), u(front.y)), (u(x0 + w / 2.0), u(front.y - 1.0)), (u(x0 + w), u(front.y))]);
    b.fill(Layer::Ground, tri, GRANITE_LIGHT);
    // The dome: a drum, then the gold.
    let c = Vec2::new(roof.center().x, roof.y + roof.h * 0.5);
    let (x, y) = p(c);
    let r = u(roof.h.min(roof.w) * 0.42);
    b.fill(Layer::Ground, PathData::circle(x, y, r * 1.18), GRANITE_LIGHT);
    let mut ring = PathData::new();
    for k in 0..16 {
        let a = k as f32 / 16.0 * std::f32::consts::TAU;
        ring.extend(&PathData::circle(x + a.cos() * r * 1.1, y + a.sin() * r * 1.1, u(0.12)));
    }
    b.fill(Layer::Ground, ring, Color::hex(0xf2efe6));
    b.add(
        Layer::Ground,
        Element::Path {
            group: 0,
            d: PathData::circle(0.0, 0.0, 1.0),
            fill: Fill::Gradient(GRAD_DOME),
            even_odd: false,
            stroke: None,
            t: Some(Affine([r, 0.0, 0.0, r, x, y])),
        },
    );
    // Gilded ribs.
    let mut ribs = PathData::new();
    for k in 0..8 {
        let a = k as f32 / 8.0 * std::f32::consts::TAU;
        ribs.move_to(x + a.cos() * r * 0.2, y + a.sin() * r * 0.2).line_to(x + a.cos() * r, y + a.sin() * r);
    }
    b.stroke(Layer::Ground, ribs, Color::rgba(120, 80, 20, 140), 0.8);
    // Ceres, very small, at the top.
    b.fill(Layer::Ground, PathData::circle(x, y, u(0.22)), Color::hex(0x7a6a58));
}

fn college_tower(b: &mut Builder, roof: &Rect) {
    let c = roof.center();
    let s = 1.6;
    let t = Rect::new(c.x - s / 2.0, roof.y + 0.3, s, s);
    b.fill(Layer::Ground, rect_path(&t), GRANITE_LIGHT);
    let (x, y) = p(t.center());
    let h = u(s / 2.0);
    let mut d = PathData::new();
    d.move_to(x - h, y - h).line_to(x + h, y + h).move_to(x + h, y - h).line_to(x - h, y + h);
    b.stroke(Layer::Ground, d, Color::hex(0x3a3c44), u(0.12));
    b.fill(Layer::Ground, PathData::circle(x, y, u(0.18)), Color::hex(0xd4a640));
}

fn steeple(b: &mut Builder, bd: &Building, roof: &Rect) {
    // At the door's end of the church.
    let c = match bd.door.facing {
        Dir::N => Vec2::new(roof.center().x, roof.y + 1.0),
        Dir::S => Vec2::new(roof.center().x, roof.y1() - 1.0),
        Dir::E => Vec2::new(roof.x1() - 1.0, roof.center().y),
        Dir::W => Vec2::new(roof.x + 1.0, roof.center().y),
    };
    let (x, y) = p(c);
    let h = u(0.8);
    b.fill(Layer::Ground, PathData::rect(x - h, y - h, 2.0 * h, 2.0 * h), Color::hex(0xece6da));
    let mut d = PathData::new();
    d.move_to(x - h, y - h).line_to(x + h, y + h).move_to(x + h, y - h).line_to(x - h, y + h);
    b.stroke(Layer::Ground, d, Color::hex(0x5a5c66), u(0.1));
    let mut cross = PathData::new();
    cross.move_to(x, y - u(0.45)).line_to(x, y + u(0.45)).move_to(x - u(0.28), y - u(0.12)).line_to(x + u(0.28), y - u(0.12));
    b.stroke(Layer::Ground, cross, Color::hex(0xd8b04a), u(0.12));
}

// ------------------------------------------------------------ features --

fn feature(b: &mut Builder, town: &Town, f: &Feature) {
    let (x, y) = p(f.pos);
    let mut rng = seeded(f.seed as u64 ^ 0xfea7);
    match f.kind {
        FeatureKind::Tree(_) | FeatureKind::TpTree => {
            // The trunk and its shadow; the crown is in the canopy.
            b.fill(Layer::Ground, PathData::ellipse(x + u(0.5), y + u(0.35), u(1.1), u(0.55)), SHADOW);
            b.fill(Layer::Ground, PathData::circle(x, y, u(0.24)), BARK);
        }
        FeatureKind::Bush => {
            let s = f.size;
            let burning = rng.random_bool(0.35);
            let base = if burning { Color::hex(0x8c1f24) } else { Color::hex(0x23402a) };
            let mut d = PathData::new();
            for _ in 0..6 {
                let o = Vec2::new(rng.random_range(-0.4..0.4), rng.random_range(-0.3..0.3)) * s;
                d.extend(&PathData::circle(x + u(o.x), y + u(o.y), u(0.42 * s)));
            }
            b.fill(Layer::Ground, PathData::ellipse(x + u(0.25), y + u(0.3), u(0.8 * s), u(0.45 * s)), SHADOW);
            b.fill(Layer::Ground, d, base);
            let mut hl = PathData::new();
            for _ in 0..4 {
                let o = Vec2::new(rng.random_range(-0.4..0.3), rng.random_range(-0.4..0.1)) * s;
                hl.extend(&PathData::circle(x + u(o.x), y + u(o.y), u(0.2 * s)));
            }
            b.fill(Layer::Ground, hl, base.shade(0.18));
        }
        FeatureKind::Raspberry => {
            let s = f.size;
            let mut canes = PathData::new();
            for _ in 0..9 {
                let a: f32 = rng.random_range(0.0..std::f32::consts::TAU);
                let l = rng.random_range(0.4..0.9) * s;
                canes.move_to(x, y).quad_to(x + u(a.cos() * l * 0.5) + u(0.2), y + u(a.sin() * l * 0.5) - u(0.2), x + u(a.cos() * l), y + u(a.sin() * l));
            }
            b.stroke(Layer::Ground, canes, Color::hex(0x5a2a2e), u(0.08));
            let mut leaves = PathData::new();
            for _ in 0..10 {
                let o = Vec2::new(rng.random_range(-0.7..0.7), rng.random_range(-0.6..0.6)) * s;
                leaves.extend(&PathData::ellipse(x + u(o.x), y + u(o.y), u(0.18), u(0.12)));
            }
            b.fill(Layer::Ground, leaves, Color::hex(0x3a5a2a));
            b.decal(DecalKind::Berries { f: f.id }, f.pos, Layer::Ground, s, f.seed);
        }
        FeatureKind::LeafPile => b.decal(DecalKind::LeafPile { f: f.id }, f.pos, Layer::Ground, 1.6, f.seed),
        FeatureKind::Puddle => {
            let s = f.size;
            let mut pts = Vec::new();
            for k in 0..9 {
                let a = k as f32 / 9.0 * std::f32::consts::TAU;
                let r = rng.random_range(0.6..1.0) * s;
                pts.push((x + u(a.cos() * r * 0.9), y + u(a.sin() * r * 0.55)));
            }
            b.fill(Layer::Ground, PathData::smooth_closed(&pts, 1.0), Color::hex(0x3a3328));
            b.fill(Layer::Ground, PathData::ellipse(x, y, u(0.55 * s), u(0.28 * s)), Color::hex(0x2a3a52));
            b.fill(Layer::Ground, PathData::ellipse(x - u(0.2 * s), y - u(0.08 * s), u(0.2 * s), u(0.06 * s)), Color::rgba(200, 210, 240, 70));
        }
        FeatureKind::Streetlight => {
            b.add(
                Layer::Ground,
                Element::Path {
                    group: 0,
                    d: PathData::circle(0.0, 0.0, 1.0),
                    fill: Fill::Gradient(GRAD_GLOW),
                    even_odd: false,
                    stroke: None,
                    t: Some(Affine([u(2.6), 0.0, 0.0, u(1.9), x, y + u(0.4)])),
                },
            );
            b.fill(Layer::Ground, PathData::circle(x, y, u(0.12)), Color::hex(0x2a2a30));
        }
        FeatureKind::Pumpkin => b.decal(DecalKind::Pumpkin { f: f.id }, f.pos, Layer::Ground, 1.0, f.seed),
        FeatureKind::Bench => {
            let mut slats = PathData::new();
            for k in 0..3 {
                slats.extend(&PathData::rect(x - u(0.6), y - u(0.2) + u(0.14) * k as f32, u(1.2), u(0.1)));
            }
            b.fill(Layer::Ground, slats, Color::hex(0x6a4a30));
        }
        FeatureKind::Hydrant => {
            b.fill(Layer::Ground, PathData::circle(x, y, u(0.2)), Color::hex(0xc0302a));
            b.fill(Layer::Ground, PathData::circle(x - u(0.05), y - u(0.05), u(0.09)), Color::hex(0xe86a5a));
        }
        FeatureKind::FlyerPole => {
            b.fill(Layer::Ground, PathData::circle(x, y, u(0.16)), Color::hex(0x6a5038));
            b.decal(DecalKind::Flyers { f: f.id }, f.pos, Layer::Ground, 0.8, f.seed);
        }
        FeatureKind::CandyBowl => b.decal(DecalKind::Bowl { f: f.id }, f.pos, Layer::Ground, 0.8, f.seed),
        FeatureKind::EggBasket => b.decal(DecalKind::Basket { f: f.id }, f.pos, Layer::Ground, 1.2, f.seed),
        FeatureKind::Statue => {
            b.fill(Layer::Ground, PathData::rect(x - u(0.6), y - u(0.6), u(1.2), u(1.2)), GRANITE);
            b.fill(Layer::Ground, PathData::circle(x, y - u(0.2), u(0.35)), Color::hex(0x5a6a5a));
        }
        FeatureKind::Cow => cow(b, f, &mut rng),
        FeatureKind::Flag | FeatureKind::GmbFlag => {
            b.fill(Layer::Ground, PathData::circle(x, y, u(0.12)), Color::hex(0x8a8a90));
            b.decal(
                DecalKind::Flag {
                    f: f.id,
                    gmb: f.kind == FeatureKind::GmbFlag,
                },
                Vec2::new(f.pos.x + 0.7, f.pos.y - 2.0),
                Layer::Canopy,
                2.2,
                f.seed,
            );
        }
    }
    let _ = town;
}

/// A painted fibreglass cow, side on, on a little plinth. No two alike.
fn cow(b: &mut Builder, f: &Feature, rng: &mut ChaCha8Rng) {
    let (x, y) = p(f.pos);
    let flip = if rng.random_bool(0.5) { 1.0 } else { -1.0 };
    let base = [
        Color::hex(0xf2efe6),
        Color::hex(0xd8342a),
        Color::hex(0x2a5ab0),
        Color::hex(0xe8b030),
        Color::hex(0x7a3aa0),
        Color::hex(0x2a8a5a),
    ][rng.random_range(0..6)];
    let pattern = rng.random_range(0..5);
    b.fill(Layer::Ground, PathData::rect(x - u(0.85), y + u(0.05), u(1.7), u(0.32)), GRANITE);
    // Legs.
    let mut legs = PathData::new();
    for k in [-0.55f32, -0.35, 0.35, 0.55] {
        legs.extend(&PathData::rect(x + u(k * flip) - u(0.06), y - u(0.35), u(0.12), u(0.42)));
    }
    b.fill(Layer::Ground, legs, base.shade(-0.3));
    let body = PathData::round_rect(x - u(0.72), y - u(0.95), u(1.44), u(0.68), u(0.28));
    b.fill(Layer::Ground, body.clone(), base);
    let head = PathData::round_rect(x + u(0.62 * flip) - u(0.2), y - u(1.12), u(0.42), u(0.36), u(0.12));
    b.fill(Layer::Ground, head, base.shade(-0.08));
    let (x0, y0) = (x - u(0.6), y - u(0.85));
    let mut d = PathData::new();
    let accent = match pattern {
        0 => {
            // Holstein.
            for _ in 0..4 {
                d.extend(&PathData::ellipse(x0 + u(rng.random_range(0.1..1.1)), y0 + u(rng.random_range(0.08..0.45)), u(0.18), u(0.11)));
            }
            Color::hex(0x1a1a1e)
        }
        1 => {
            // Maple leaves.
            for _ in 0..3 {
                let cx = x0 + u(rng.random_range(0.15..1.05));
                let cy = y0 + u(rng.random_range(0.1..0.42));
                d.extend(&leaf_symbol().transformed(Affine([u(0.3), 0.0, 0.0, u(0.3), cx, cy])));
            }
            Color::hex(0xd23a1a)
        }
        2 => {
            // Stars.
            for _ in 0..5 {
                d.extend(&PathData::circle(x0 + u(rng.random_range(0.1..1.1)), y0 + u(rng.random_range(0.05..0.45)), u(0.05)));
            }
            Color::hex(0xfff2a0)
        }
        3 => {
            // Plaid, like a hunting jacket.
            for k in 0..4 {
                d.extend(&PathData::rect(x0 + u(0.3 * k as f32), y0 - u(0.08), u(0.1), u(0.6)));
            }
            d.extend(&PathData::rect(x0, y0 + u(0.18), u(1.2), u(0.1)));
            Color::hex(0x1a1a1e)
        }
        _ => {
            // Tie-dye swirls.
            for k in 0..3 {
                d.extend(&PathData::circle(x0 + u(0.3 + 0.3 * k as f32), y0 + u(0.25), u(0.14)));
            }
            Color::hex(0xff7ad0)
        }
    };
    b.fill(Layer::Ground, d, accent);
    b.fill(Layer::Ground, PathData::circle(x + u(0.72 * flip), y - u(0.98), u(0.04)), INK);
}

fn street_labels(b: &mut Builder, town: &Town) {
    for s in town.streets.iter().filter(|s| matches!(s.kind, StreetKind::Street | StreetKind::Lane)) {
        let name = s.name.replace(" Street", " St").replace(" Avenue", " Ave").replace(" Drive", " Dr").to_uppercase();
        let size = u(s.width * 0.42);
        let (path, adv) = text_path(&name, size);
        if adv <= 0.0 {
            continue;
        }
        let len = s.length();
        let mut d = 9.0;
        while d < len - 6.0 {
            let (at, dir) = s.at(d);
            // Text reads left to right, or bottom to top.
            let dir = if dir.x < -0.3 || (dir.x.abs() <= 0.3 && dir.y > 0.0) { -dir } else { dir };
            let (x, y) = p(at);
            let t = Affine([dir.x, dir.y, -dir.y, dir.x, x, y])
                .then(Affine::translate(0.0, 0.0))
                .pre(Affine([1.0, 0.0, 0.0, 1.0, -adv / 2.0, size * 0.35]));
            b.add(
                Layer::Ground,
                Element::Path {
                    group: 0,
                    d: path.clone(),
                    fill: Fill::Solid(Color::rgba(220, 220, 235, 70)),
                    even_odd: false,
                    stroke: None,
                    t: Some(t),
                },
            );
            d += 34.0;
        }
    }
}

trait Pre {
    fn pre(self, o: Affine) -> Affine;
}

impl Pre for Affine {
    /// `self` after `o`: maps through `o` first.
    fn pre(self, o: Affine) -> Affine {
        o.then(self)
    }
}

// ------------------------------------------------------------- canopy --

/// Half the trees are bare, half are in full colour: it is a game.
pub fn bare(f: &Feature) -> bool {
    f.seed % 2 == 0
}

fn canopy(b: &mut Builder, town: &Town) {
    for f in &town.features {
        let (x, y) = p(f.pos);
        let mut rng = seeded(f.seed as u64 ^ 0xc4a0);
        match f.kind {
            FeatureKind::Tree(_) | FeatureKind::TpTree if !matches!(f.kind, FeatureKind::Tree(TreeKind::Pine)) => {
                let kind = match f.kind {
                    FeatureKind::Tree(k) => k,
                    _ => TreeKind::Maple,
                };
                let s = f.size * if kind == TreeKind::Oak { 1.25 } else { 1.0 };
                let cy = y - u(1.1 * s);
                if bare(f) && f.kind != FeatureKind::TpTree {
                    bare_tree(b, x, y, s, kind, &mut rng);
                } else {
                    let colours: Vec<Color> = match kind {
                        TreeKind::Birch => vec![Color::hex(0xe8c23a), Color::hex(0xd4a82a)],
                        TreeKind::Oak => vec![Color::hex(0x8a3a1a), Color::hex(0xa8562a), Color::hex(0x6a3a1a)],
                        _ => {
                            let a = rng.random_range(0..FOLIAGE.len());
                            vec![FOLIAGE[a], FOLIAGE[(a + 1) % FOLIAGE.len()]]
                        }
                    };
                    let mut crown = PathData::new();
                    for _ in 0..7 {
                        let o = Vec2::new(rng.random_range(-0.7..0.7), rng.random_range(-0.55..0.55)) * s;
                        crown.extend(&PathData::circle(x + u(o.x), cy + u(o.y), u(rng.random_range(0.55..0.8) * s)));
                    }
                    b.fill(Layer::Canopy, crown, colours[0].shade(-0.25));
                    let mut top = PathData::new();
                    for _ in 0..6 {
                        let o = Vec2::new(rng.random_range(-0.6..0.45), rng.random_range(-0.6..0.25)) * s;
                        top.extend(&PathData::circle(x + u(o.x), cy + u(o.y), u(rng.random_range(0.35..0.55) * s)));
                    }
                    b.fill(Layer::Canopy, top, colours[1 % colours.len()]);
                    let mut glint = PathData::new();
                    for _ in 0..4 {
                        let o = Vec2::new(rng.random_range(-0.5..0.2), rng.random_range(-0.6..0.0)) * s;
                        glint.extend(&PathData::circle(x + u(o.x), cy + u(o.y), u(0.2 * s)));
                    }
                    b.fill(Layer::Canopy, glint, colours[colours.len() - 1].shade(0.25));
                }
                if f.kind == FeatureKind::TpTree {
                    b.decal(DecalKind::Tp { f: f.id }, Vec2::new(f.pos.x, f.pos.y - 1.1 * s), Layer::Canopy, 1.6, f.seed);
                }
            }
            FeatureKind::Tree(TreeKind::Pine) => {
                let cy = y - u(1.0);
                for (k, (r, c)) in [(1.05f32, PINE), (0.7, PINE_LIGHT), (0.35, PINE_LIGHT.shade(0.15))].into_iter().enumerate() {
                    let n = 14;
                    let mut pts = Vec::new();
                    for i in 0..n {
                        let a = i as f32 / n as f32 * std::f32::consts::TAU + k as f32 * 0.3;
                        let rr = if i % 2 == 0 { r } else { r * 0.72 };
                        pts.push((x + u(a.cos() * rr), cy - u(k as f32 * 0.2) + u(a.sin() * rr)));
                    }
                    b.fill(Layer::Canopy, PathData::polygon(&pts), c);
                }
            }
            FeatureKind::Streetlight => {
                let head = Element::Path {
                    group: 0,
                    d: PathData::circle(0.0, 0.0, 1.0),
                    fill: Fill::Gradient(GRAD_LAMP),
                    even_odd: false,
                    stroke: None,
                    t: Some(Affine([u(0.45), 0.0, 0.0, u(0.45), x, y - u(1.6)])),
                };
                let mut arm = PathData::new();
                arm.move_to(x, y).line_to(x, y - u(1.6));
                b.stroke(Layer::Canopy, arm, Color::hex(0x2a2a32), u(0.1));
                b.add(Layer::Canopy, head);
            }
            _ => {}
        }
    }
}

fn bare_tree(b: &mut Builder, x: f32, y: f32, s: f32, kind: TreeKind, rng: &mut ChaCha8Rng) {
    let mut d = PathData::new();
    let top = y - u(1.1 * s);
    d.move_to(x, y).line_to(x, top);
    for _ in 0..7 {
        let a: f32 = rng.random_range(-2.9..-0.25);
        let l = rng.random_range(0.6..1.1) * s;
        let (bx, by) = (x + u(rng.random_range(-0.1..0.1)), top + u(rng.random_range(0.0..0.5)));
        let ex = bx + u(a.cos() * l);
        let ey = by + u(a.sin() * l * 0.8);
        d.move_to(bx, by).line_to(ex, ey);
        let twig = a + rng.random_range(-0.7..0.7);
        d.move_to((bx + ex) / 2.0, (by + ey) / 2.0).line_to((bx + ex) / 2.0 + u(twig.cos() * 0.4), (by + ey) / 2.0 + u(twig.sin() * 0.35));
    }
    let c = if kind == TreeKind::Birch { Color::hex(0xc8c4bc) } else { Color::hex(0x5a4a44) };
    b.stroke(Layer::Canopy, d, c, u(0.07 * s.max(0.8)));
}

// --------------------------------------------------------------- decals --

/// The state each decal shows, from the game.
pub fn decal_state(g: &crate::sim::Game, slot: &DecalSlot) -> u32 {
    match slot.kind {
        DecalKind::PorchLight { b } => u32::from(g.houses[b].light),
        DecalKind::Pumpkin { .. } => 0,
        DecalKind::Berries { f } | DecalKind::LeafPile { f } => u32::from(g.feats[f].used),
        DecalKind::Tp { f } | DecalKind::Flyers { f } => g.feats[f].n,
        DecalKind::Bowl { f } => g.feats[f].n.min(14),
        DecalKind::Basket { f } => g.feats[f].n,
        DecalKind::Flag { .. } => 0,
    }
}

/// A pumpkin's flame flickers (on Silkie, by patching it): `phase` 0, 1 or
/// 2. Its state otherwise never changes.
pub fn decal_elements(slot: &DecalSlot, state: u32, flicker: u32, hdr: bool) -> Vec<(u32, Element)> {
    let mut out: Vec<(u32, Element)> = Vec::new();
    let (x, y) = p(slot.at);
    let mut rng = seeded(slot.seed as u64 ^ 0xdeca1);
    let glow = |c: Color, pct: u16| if hdr { c.with_bright(pct) } else { c };
    let mut push = |e: Element| {
        let id = slot.first + out.len() as u32;
        out.push((id, e));
    };
    match slot.kind {
        DecalKind::PorchLight { .. } => {
            let on = state == 1;
            push(Element::Path {
                group: 0,
                d: PathData::circle(0.0, 0.0, 1.0),
                fill: if on { Fill::Gradient(GRAD_GLOW) } else { Fill::None },
                even_odd: false,
                stroke: None,
                t: Some(Affine([u(1.7), 0.0, 0.0, u(1.3), x, y + u(0.5)])),
            });
            push(Element::fill_path(0, PathData::circle(x, y, u(0.16)), if on { glow(Color::hex(0xfff0b0), 300) } else { Color::hex(0x3a3a40) }));
        }
        DecalKind::Pumpkin { .. } => {
            push(Element::fill_path(0, PathData::ellipse(x + u(0.12), y + u(0.12), u(0.36), u(0.18)), SHADOW));
            let mut body = PathData::new();
            for k in [-0.18f32, 0.0, 0.18] {
                body.extend(&PathData::ellipse(x + u(k), y, u(0.2), u(0.26)));
            }
            push(Element::fill_path(0, body, Color::hex(0xd8661c)));
            push(Element::fill_path(0, PathData::rect(x - u(0.04), y - u(0.36), u(0.08), u(0.12)), Color::hex(0x3a5a22)));
            // The face, lit from inside.
            let k = [1.0f32, 0.82, 1.12][(flicker % 3) as usize];
            let mut face = PathData::new();
            face.extend(&PathData::polygon(&[(x - u(0.16), y - u(0.04)), (x - u(0.06), y - u(0.04)), (x - u(0.11), y - u(0.13) * k)]));
            face.extend(&PathData::polygon(&[(x + u(0.06), y - u(0.04)), (x + u(0.16), y - u(0.04)), (x + u(0.11), y - u(0.13) * k)]));
            face.extend(&PathData::polygon(&[(x - u(0.15), y + u(0.06)), (x + u(0.15), y + u(0.06)), (x, y + u(0.06) + u(0.1) * k)]));
            push(Element::fill_path(0, face, glow(Color::hex(0xffd860), (180.0 * k) as u16)));
            push(Element::Path {
                group: 0,
                d: PathData::circle(0.0, 0.0, 1.0),
                fill: Fill::Gradient(GRAD_GLOW),
                even_odd: false,
                stroke: None,
                t: Some(Affine([u(0.7) * k, 0.0, 0.0, u(0.5) * k, x, y + u(0.1)])),
            });
        }
        DecalKind::Berries { .. } => {
            let mut d = PathData::new();
            if state == 0 {
                for _ in 0..12 {
                    let o = Vec2::new(rng.random_range(-0.7..0.7), rng.random_range(-0.55..0.55));
                    d.extend(&PathData::circle(x + u(o.x), y + u(o.y), u(0.08)));
                }
            } else {
                d.move_to(x, y);
            }
            push(Element::fill_path(0, d, if state == 0 { glow(Color::hex(0xe02a4a), 110) } else { Color::rgba(0, 0, 0, 0) }));
        }
        DecalKind::LeafPile { .. } => {
            let (n, spread) = if state == 0 { (5, 0.55) } else { (5, 1.5) };
            for k in 0..n {
                let mut d = PathData::new();
                for _ in 0..(if state == 0 { 14 } else { 6 }) {
                    let a: f32 = rng.random_range(0.0..std::f32::consts::TAU);
                    let r = rng.random_range(0.0..spread);
                    let cx = x + u(a.cos() * r);
                    let cy = y + u(a.sin() * r * 0.7);
                    let s = u(if state == 0 { 0.3 } else { 0.22 });
                    let rot: f32 = rng.random_range(0.0..std::f32::consts::TAU);
                    d.extend(&leaf_symbol().transformed(Affine([s * rot.cos(), s * rot.sin(), -s * rot.sin(), s * rot.cos(), cx, cy])));
                }
                push(Element::fill_path(0, d, FOLIAGE[k % FOLIAGE.len()]));
            }
        }
        DecalKind::Tp { .. } => {
            let n = state.min(6);
            let mut d = PathData::new();
            for k in 0..n {
                let a = -2.6 + k as f32 * 0.5;
                let sx = x + u(a.cos() * 0.6);
                let sy = y + u(a.sin() * 0.4);
                d.move_to(sx, sy).cubic_to(sx + u(0.2), sy + u(0.4), sx - u(0.2), sy + u(0.7), sx + u(0.05), sy + u(1.1));
            }
            if n == 0 {
                d.move_to(x, y);
            }
            push(Element::stroke_path(0, d, StrokeStyle::new(Color::hex(0xf4f4f0), u(0.12))));
        }
        DecalKind::Bowl { .. } => {
            push(Element::fill_path(0, PathData::ellipse(x, y, u(0.32), u(0.2)), Color::hex(0xe07018)));
            let mut d = PathData::new();
            for k in 0..state.min(6) {
                d.extend(&PathData::circle(x - u(0.18) + u(0.07) * k as f32, y - u(0.05), u(0.06)));
            }
            if state == 0 {
                d.move_to(x, y);
            }
            push(Element::fill_path(0, d, Color::hex(0xf2e050)));
            push(Element::fill_path(0, PathData::rect(x + u(0.25), y - u(0.45), u(0.35), u(0.25)), Color::hex(0xf2ede0)));
        }
        DecalKind::Flyers { .. } => {
            let colours = [Color::hex(0xf4e060), Color::hex(0xf4f0e8), Color::hex(0x9ad0f0), Color::hex(0xf0a0c0)];
            for k in 0..4u32 {
                let shown = k < state;
                let d = PathData::rect(x - u(0.2) + u(0.1) * (k % 2) as f32, y - u(0.75) + u(0.18) * k as f32, u(0.3), u(0.2));
                push(Element::fill_path(0, d, if shown { colours[k as usize] } else { Color::rgba(0, 0, 0, 0) }));
            }
        }
        DecalKind::Basket { .. } => {
            push(Element::fill_path(0, PathData::ellipse(x, y, u(0.5), u(0.3)), Color::hex(0x8a6a3a)));
            let mut eggs = PathData::new();
            let shown = state.min(12);
            for k in 0..shown {
                let cx = x - u(0.32) + u(0.13) * (k % 6) as f32;
                let cy = y - u(0.06) - u(0.12) * (k / 6) as f32;
                eggs.extend(&PathData::ellipse(cx, cy, u(0.07), u(0.09)));
            }
            if shown == 0 {
                eggs.move_to(x, y);
            }
            push(Element::fill_path(0, eggs, Color::hex(0xf6ecd8)));
            // The count, once there are more than fit.
            let label = if state > 12 { format!("{state}") } else { String::new() };
            let (d, adv) = text_path(&label, u(0.5));
            let d = if label.is_empty() { PathData::polyline(&[(x, y)]) } else { d };
            push(Element::Path {
                group: 0,
                d,
                fill: Fill::Solid(Color::hex(0xffe8a0)),
                even_odd: false,
                stroke: None,
                t: Some(Affine([1.0, 0.0, 0.0, 1.0, x - adv / 2.0, y - u(0.45)])),
            });
        }
        DecalKind::Flag { gmb, .. } => {
            // The pole's foot is below and left of the decal's point.
            let (px, py) = (x - u(0.7), y + u(2.0));
            let mut pole = PathData::new();
            pole.move_to(px, py).line_to(px, py - u(2.5));
            push(Element::stroke_path(0, pole, StrokeStyle::new(Color::hex(0xc8c8d0), u(0.08))));
            // Flags ripple at half the rate candles flicker.
            let ph = (flicker / 2) as f32 * 1.3 + (slot.seed % 7) as f32;
            let top = py - u(2.4);
            let (w, h) = (u(1.3), u(0.8));
            // The cloth, rippling along its length.
            let wave = |t: f32| (ph + t * 4.0).sin() * u(0.08) * t;
            let mut cloth = Vec::new();
            for k in 0..=6 {
                let t = k as f32 / 6.0;
                cloth.push((px + w * t, top + wave(t)));
            }
            for k in (0..=6).rev() {
                let t = k as f32 / 6.0;
                cloth.push((px + w * t, top + h + wave(t)));
            }
            let field = if gmb { Color::hex(0x2e7a3a) } else { Color::hex(0xb8283a) };
            push(Element::fill_path(0, PathData::polygon(&cloth), field));
            let mut stripes = PathData::new();
            if !gmb {
                for s in 0..3 {
                    let y0 = top + h * (1.0 + 2.0 * s as f32) / 7.0;
                    let mut band = Vec::new();
                    for k in 0..=6 {
                        let t = k as f32 / 6.0;
                        band.push((px + w * t, y0 + wave(t)));
                    }
                    for k in (0..=6).rev() {
                        let t = k as f32 / 6.0;
                        band.push((px + w * t, y0 + h / 7.0 + wave(t)));
                    }
                    stripes.extend(&PathData::polygon(&band));
                }
            } else {
                stripes.move_to(px, top);
            }
            push(Element::fill_path(0, stripes, Color::hex(0xf2f0ea)));
            let cw = w * 0.42;
            let ch = h * if gmb { 0.5 } else { 4.0 / 7.0 };
            push(Element::fill_path(
                0,
                PathData::polygon(&[(px, top), (px + cw, top + wave(0.42)), (px + cw, top + ch + wave(0.42)), (px, top + ch)]),
                Color::hex(0x2a3a7a),
            ));
            let mut stars = PathData::new();
            for k in 0..(if gmb { 13 } else { 12 }) {
                let sx = px + cw * (0.15 + 0.7 * ((k % 4) as f32 / 3.0));
                let sy = top + ch * (0.2 + 0.6 * ((k / 4) as f32 / 3.0)) + wave(0.2);
                stars.extend(&PathData::circle(sx, sy, u(0.03)));
            }
            push(Element::fill_path(0, stars, Color::hex(0xf6f4ee)));
        }
    }
    debug_assert!(out.len() as u32 <= slot.count);
    out
}

// --------------------------------------------------------------- chunks --

/// Which pieces (by index) touch each chunk of `tiles` square, row by row.
pub fn chunk_index(art: &WorldArt, tiles: u32) -> (u32, u32, Vec<Vec<usize>>, Vec<Vec<usize>>) {
    let t = tiles.max(1) as f32;
    let cw = (art.width / t).ceil() as u32;
    let ch = (art.height / t).ceil() as u32;
    let mut pieces = vec![Vec::new(); (cw * ch) as usize];
    let mut slots = vec![Vec::new(); (cw * ch) as usize];
    let span = |r: &Rect| {
        let x0 = ((r.x / t).floor().max(0.0) as u32).min(cw - 1);
        let y0 = ((r.y / t).floor().max(0.0) as u32).min(ch - 1);
        let x1 = ((r.x1() / t).floor().max(0.0) as u32).min(cw - 1);
        let y1 = ((r.y1() / t).floor().max(0.0) as u32).min(ch - 1);
        (x0, y0, x1, y1)
    };
    for (i, pc) in art.pieces.iter().enumerate() {
        let (x0, y0, x1, y1) = span(&pc.bounds);
        for cy in y0..=y1 {
            for cx in x0..=x1 {
                pieces[(cy * cw + cx) as usize].push(i);
            }
        }
    }
    for (i, s) in art.slots.iter().enumerate() {
        let (x0, y0, x1, y1) = span(&s.bounds);
        for cy in y0..=y1 {
            for cx in x0..=x1 {
                slots[(cy * cw + cx) as usize].push(i);
            }
        }
    }
    (cw, ch, pieces, slots)
}

#[allow(clippy::too_many_arguments)]
/// The scene of one chunk of one layer: user units are pixels at
/// `px_per_tile`, the chunk's top left at the origin.
pub fn chunk_scene(
    art: &WorldArt,
    layer: Layer,
    origin: Vec2,
    tiles: u32,
    px_per_tile: u32,
    pieces: &[usize],
    slots: &[usize],
    states: &[u32],
    flicker: u32,
) -> Scene {
    let side = (tiles * px_per_tile) as f32;
    let mut s = Scene::new(side, side);
    let k = px_per_tile as f32 / TILE;
    s.set(
        0,
        Element::Group {
            parent: 0,
            t: Some(Affine([k, 0.0, 0.0, k, -origin.x * px_per_tile as f32, -origin.y * px_per_tile as f32])),
            opacity: 100.0,
            visible: true,
            multiply: false,
        },
    );
    for (id, e) in &art.shared {
        s.set(*id, e.clone());
    }
    // Pieces and decals, in drawing order (decals sit at their reserved ids).
    let mut order: Vec<(u32, Option<usize>, Option<usize>)> = Vec::new();
    for &i in pieces {
        let pc = &art.pieces[i];
        if pc.layer == layer {
            order.push((pc.id, Some(i), None));
        }
    }
    for &i in slots {
        let sl = &art.slots[i];
        if sl.layer == layer {
            order.push((sl.first, None, Some(i)));
        }
    }
    order.sort_by_key(|o| o.0);
    for (_, pi, si) in order {
        if let Some(i) = pi {
            s.set(art.pieces[i].id, art.pieces[i].elem.clone());
        }
        if let Some(i) = si {
            let sl = &art.slots[i];
            let st = states.get(i).copied().unwrap_or(0);
            for (id, e) in decal_elements(sl, st, flicker, art.hdr) {
                s.set(id, e);
            }
        }
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::{plan, town};

    fn art() -> (Town, WorldArt) {
        let t = plan::build(&town::parse(town::MONTPELIER).unwrap());
        let a = build(&t, false);
        (t, a)
    }

    #[test]
    fn every_piece_has_bounds_inside_the_town() {
        let (t, a) = art();
        assert!(a.pieces.len() > 3000, "{} pieces", a.pieces.len());
        for pc in &a.pieces {
            assert!(pc.bounds.x1() >= -2.0 && pc.bounds.x <= t.w as f32 + 2.0, "{:?}", pc.bounds);
        }
    }

    #[test]
    fn decals_keep_their_ids_in_every_state() {
        let (_, a) = art();
        for sl in &a.slots {
            let ids0: Vec<u32> = decal_elements(sl, 0, 0, false).into_iter().map(|e| e.0).collect();
            for st in [1, 2, 5, 13, 40] {
                let ids: Vec<u32> = decal_elements(sl, st, st, true).into_iter().map(|e| e.0).collect();
                assert_eq!(ids, ids0, "{:?}", sl.kind);
            }
            assert!(ids0.len() as u32 <= sl.count);
        }
    }

    #[test]
    fn a_chunk_scene_is_reasonably_small() {
        let (_, a) = art();
        let (cw, _, pieces, slots) = chunk_index(&a, 20);
        let i = (3 * cw + 6) as usize;
        let states = vec![0; a.slots.len()];
        let s = chunk_scene(&a, Layer::Ground, Vec2::new(120.0, 60.0), 20, 48, &pieces[i], &slots[i], &states, 0);
        let rec = s.records();
        assert!(rec.len() < 400_000, "{} bytes", rec.len());
    }
}
