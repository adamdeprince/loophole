//! Lays out the town from its description: the terrain grid, the houses
//! along every street marked `houses` (with addresses, porches, pumpkins and
//! yards), the woods' trees, streetlights, and an index for finding the
//! features near a point.

use super::grid::{Grid, Terrain, centre};
use super::town::{
    Area, AreaKind, BuildingKind, BuildingSpec, Door, FeatureKind, River, Street, StreetKind, TownSpec, TreeKind,
};
use super::{Dir, Rect, in_polygon, polyline_distance};
use glam::Vec2;
use rand::{RngExt, SeedableRng};
use rand_chacha::ChaCha8Rng;

/// How a house looks: indices into the scenery's palettes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Style {
    pub body: u8,
    pub trim: u8,
    pub roof: u8,
    pub porch: bool,
    pub stories: u8,
    /// A turret or a bay: the Victorian touch.
    pub turret: bool,
}

#[derive(Clone, Debug)]
pub struct Building {
    pub id: usize,
    pub kind: BuildingKind,
    /// "14 Elm Street" for a house; the place's own name otherwise.
    pub name: String,
    /// The street the door faces, when known.
    pub street: Option<String>,
    pub rect: Rect,
    pub door: Door,
    pub say: Option<String>,
    pub gives: Option<String>,
    pub style: Style,
    pub seed: u64,
}

impl Building {
    /// A name for the location line ("14 Elm Street", "the Fire Station").
    pub fn label(&self) -> &str {
        &self.name
    }
}

#[derive(Clone, Debug)]
pub struct Feature {
    pub id: usize,
    pub kind: FeatureKind,
    pub pos: Vec2,
    pub size: f32,
    /// The building whose yard or step it is in.
    pub owner: Option<usize>,
    pub seed: u32,
}

/// Features bucketed by area, for "what is near here" questions.
#[derive(Clone, Debug)]
pub struct FeatureIndex {
    bw: i32,
    bh: i32,
    buckets: Vec<Vec<usize>>,
}

const BUCKET: f32 = 8.0;

impl FeatureIndex {
    fn new(w: u32, h: u32, features: &[Feature]) -> FeatureIndex {
        let bw = ((w as f32 / BUCKET).ceil() as i32).max(1);
        let bh = ((h as f32 / BUCKET).ceil() as i32).max(1);
        let mut buckets = vec![Vec::new(); (bw * bh) as usize];
        for f in features {
            let bx = ((f.pos.x / BUCKET).floor() as i32).clamp(0, bw - 1);
            let by = ((f.pos.y / BUCKET).floor() as i32).clamp(0, bh - 1);
            buckets[(by * bw + bx) as usize].push(f.id);
        }
        FeatureIndex { bw, bh, buckets }
    }

    /// Ids of every feature whose position is within `r` of `p` (and maybe
    /// a few more: callers check distance).
    pub fn near(&self, p: Vec2, r: f32) -> impl Iterator<Item = usize> + '_ {
        let bx0 = (((p.x - r) / BUCKET).floor() as i32).clamp(0, self.bw - 1);
        let bx1 = (((p.x + r) / BUCKET).floor() as i32).clamp(0, self.bw - 1);
        let by0 = (((p.y - r) / BUCKET).floor() as i32).clamp(0, self.bh - 1);
        let by1 = (((p.y + r) / BUCKET).floor() as i32).clamp(0, self.bh - 1);
        (by0..=by1).flat_map(move |by| (bx0..=bx1).flat_map(move |bx| self.buckets[(by * self.bw + bx) as usize].iter().copied()))
    }
}

/// The whole town, laid out.
#[derive(Clone, Debug)]
pub struct Town {
    pub name: String,
    pub w: u32,
    pub h: u32,
    pub start: Vec2,
    pub streets: Vec<Street>,
    pub rivers: Vec<River>,
    pub areas: Vec<Area>,
    pub buildings: Vec<Building>,
    pub features: Vec<Feature>,
    pub grid: Grid,
    pub index: FeatureIndex,
    /// Index of the children's house.
    pub home: usize,
}

impl Town {
    pub fn size(&self) -> Vec2 {
        Vec2::new(self.w as f32, self.h as f32)
    }

    /// Features near `p` within `r` (checked).
    pub fn features_near(&self, p: Vec2, r: f32) -> impl Iterator<Item = &Feature> + '_ {
        self.index
            .near(p, r + 1.0)
            .map(move |i| &self.features[i])
            .filter(move |f| f.pos.distance(p) <= r + f.kind.reach().max(f.size * 0.5))
    }

    /// The building whose door step is nearest `p`, within `r`.
    pub fn door_near(&self, p: Vec2, r: f32) -> Option<&Building> {
        self.buildings
            .iter()
            .filter(|b| b.door.step().distance(p) <= r)
            .min_by(|a, b| a.door.step().distance(p).total_cmp(&b.door.step().distance(p)))
    }

    /// The street `p` is on or beside, if any.
    pub fn street_at(&self, p: Vec2) -> Option<&Street> {
        self.streets
            .iter()
            .map(|s| (s, polyline_distance(p, &s.pts) - s.half_span()))
            .filter(|(_, d)| *d < 1.5)
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(s, _)| s)
    }

    /// Words for where `p` is: "on Main Street, outside Bare Paws Books".
    pub fn describe(&self, p: Vec2) -> String {
        if self.grid.at(p) == Terrain::Water {
            let r = self
                .rivers
                .iter()
                .min_by(|a, b| polyline_distance(p, &a.pts).total_cmp(&polyline_distance(p, &b.pts)));
            return format!("swimming in the {}", r.map_or("river", |r| r.name.as_str()));
        }
        let building = self
            .buildings
            .iter()
            .map(|b| (b, b.rect.distance(p)))
            .filter(|(_, d)| *d < 2.6)
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(b, _)| b);
        let area = self.areas.iter().find(|a| in_polygon(p, &a.pts));
        let street = self.street_at(p);
        match (street, building) {
            (Some(s), Some(b)) if b.kind == BuildingKind::House => format!("on {}, by number {}", s.name, b.name.split(' ').next().unwrap_or("")),
            (Some(s), Some(b)) => format!("on {}, outside {}", s.name, b.name),
            (Some(s), None) => format!("on {}", s.name),
            (None, Some(b)) if b.kind == BuildingKind::House => format!("in the yard of {}", b.name),
            (None, Some(b)) => format!("by {}", b.name),
            (None, None) => match area {
                Some(a) => format!("in {}", a.name),
                None => "in somebody's back yard".to_string(),
            },
        }
    }

    /// Whether a circle of radius `r` at `p` is clear of walls, water (when
    /// `swim` is false), the edge of the map and solid features.
    pub fn clear(&self, p: Vec2, r: f32, swim: bool) -> bool {
        for (x, y) in [(-r, -r), (r, -r), (-r, r), (r, r), (0.0, 0.0)] {
            let t = self.grid.at(p + Vec2::new(x, y));
            if !t.walkable() || (!swim && t == Terrain::Water) {
                return false;
            }
        }
        !self
            .index
            .near(p, r + 1.0)
            .any(|i| {
                let f = &self.features[i];
                let s = f.kind.solid_radius();
                s > 0.0 && f.pos.distance(p) < s + r
            })
    }
}

fn perp(d: Vec2) -> Vec2 {
    Vec2::new(-d.y, d.x)
}

/// Lays out the town from its description.
pub fn build(spec: &TownSpec) -> Town {
    let mut rng = ChaCha8Rng::seed_from_u64(spec.seed);
    let mut grid = Grid::new(spec.width, spec.height);
    // Areas, then water, then streets over them, then buildings.
    for a in &spec.areas {
        let b = super::bounds_of(&a.pts, 0.0);
        let t = match a.kind {
            AreaKind::Woods => Terrain::Woods,
            AreaKind::Lawn => Terrain::Lawn,
            AreaKind::Green => Terrain::Green,
            AreaKind::Parking => Terrain::Parking,
            AreaKind::Plaza => Terrain::Plaza,
        };
        for (x, y) in grid.tiles_in(b.x, b.y, b.x1(), b.y1()).collect::<Vec<_>>() {
            if in_polygon(centre(x, y), &a.pts) {
                grid.set(x, y, t);
            }
        }
    }
    for r in &spec.rivers {
        let b = super::bounds_of(&r.pts, r.width / 2.0 + 1.0);
        for (x, y) in grid.tiles_in(b.x, b.y, b.x1(), b.y1()).collect::<Vec<_>>() {
            if polyline_distance(centre(x, y), &r.pts) <= r.width / 2.0 {
                grid.set(x, y, Terrain::Water);
            }
        }
    }
    // Paths first, so that roads and sidewalks win where they cross.
    let mut order: Vec<&Street> = spec.streets.iter().collect();
    order.sort_by_key(|s| match s.kind {
        StreetKind::Path => 0,
        StreetKind::Footbridge => 1,
        StreetKind::Lane => 2,
        StreetKind::Street => 3,
    });
    for s in &order {
        let b = super::bounds_of(&s.pts, s.half_span() + 1.0);
        for (x, y) in grid.tiles_in(b.x, b.y, b.x1(), b.y1()).collect::<Vec<_>>() {
            let d = polyline_distance(centre(x, y), &s.pts);
            if d > s.half_span().max(0.5) {
                continue;
            }
            let under = grid.get(x, y);
            let t = if under == Terrain::Water || under == Terrain::Bridge {
                Terrain::Bridge
            } else if d <= s.width / 2.0 {
                match s.kind {
                    StreetKind::Street | StreetKind::Lane => Terrain::Road,
                    _ => Terrain::Path,
                }
            } else if under == Terrain::Road {
                // The sidewalk of one street does not cut across another.
                Terrain::Road
            } else {
                Terrain::Sidewalk
            };
            grid.set(x, y, t);
        }
    }
    let mut buildings: Vec<Building> = Vec::new();
    for bs in &spec.buildings {
        let id = buildings.len();
        let b = place_building(id, bs, &spec.streets, rng.random());
        stamp(&mut grid, &b.rect);
        buildings.push(b);
    }
    // Houses along streets.
    for s in spec.streets.iter().filter(|s| s.houses) {
        lay_out_houses(s, &mut grid, &mut buildings, &mut rng);
    }
    // Walkways from doors to the street, and yards round houses.
    for b in &buildings {
        walkway(&mut grid, b);
    }
    for b in buildings.iter().filter(|b| matches!(b.kind, BuildingKind::House | BuildingKind::Home)) {
        let r = b.rect.grow(2.4);
        for (x, y) in grid.tiles_in(r.x, r.y, r.x1(), r.y1()).collect::<Vec<_>>() {
            if grid.get(x, y) == Terrain::Grass {
                grid.set(x, y, Terrain::Yard);
            }
        }
    }
    let mut features = Vec::new();
    let add = |features: &mut Vec<Feature>, kind: FeatureKind, pos: Vec2, size: f32, owner: Option<usize>, seed: u32| {
        let id = features.len();
        features.push(Feature {
            id,
            kind,
            pos,
            size,
            owner,
            seed,
        });
    };
    for f in &spec.features {
        let t = grid.at(f.pos);
        if t == Terrain::Building || (t == Terrain::Water && f.kind != FeatureKind::Puddle) {
            tracing::warn!("{:?} at {:?} is inside a building or water; left out", f.kind, f.pos);
            continue;
        }
        add(&mut features, f.kind, f.pos, f.size, None, rng.random());
    }
    // Yards: pumpkins, bushes, trees, leaf piles and the rest.
    for b in &buildings {
        yard(&grid, b, &mut rng, &mut |k, p, s| {
            let seed = rng_seed(b.seed, p);
            add(&mut features, k, p, s, Some(b.id), seed)
        });
    }
    // Woods, greens, riverbanks and street trees.
    let mut wild = Vec::new();
    for y in 0..grid.h {
        for x in 0..grid.w {
            let p = centre(x, y) + Vec2::new(rng.random_range(-0.35..0.35), rng.random_range(-0.35..0.35));
            let roll: f32 = rng.random();
            match grid.get(x, y) {
                Terrain::Woods => {
                    if roll < 0.15 {
                        let k = match rng.random_range(0..10) {
                            0..=3 => TreeKind::Pine,
                            4..=6 => TreeKind::Maple,
                            7..=8 => TreeKind::Oak,
                            _ => TreeKind::Birch,
                        };
                        wild.push((FeatureKind::Tree(k), p, 1.0));
                    } else if roll < 0.19 {
                        wild.push((FeatureKind::Bush, p, rng.random_range(0.8..1.3)));
                    } else if roll < 0.197 && edge_of(&grid, x, y, Terrain::Woods) {
                        wild.push((FeatureKind::Raspberry, p, rng.random_range(1.0..1.5)));
                    }
                }
                Terrain::Green => {
                    if roll < 0.02 {
                        wild.push((FeatureKind::Tree(TreeKind::Maple), p, 1.3));
                    }
                }
                Terrain::Grass => {
                    let bank = near(&grid, x, y, Terrain::Water, 2);
                    if bank && roll < 0.07 {
                        let k = if rng.random_bool(0.5) { TreeKind::Birch } else { TreeKind::Maple };
                        wild.push((FeatureKind::Tree(k), p, 1.0));
                    } else if bank && roll < 0.12 {
                        wild.push((FeatureKind::Bush, p, rng.random_range(0.8..1.4)));
                    } else if roll < 0.012 {
                        wild.push((FeatureKind::Tree(TreeKind::Maple), p, 1.0));
                    } else if roll < 0.016 {
                        wild.push((FeatureKind::Bush, p, 1.0));
                    }
                }
                _ => {}
            }
        }
    }
    for s in spec.streets.iter().filter(|s| s.kind == StreetKind::Street) {
        let len = s.length();
        let step = if s.downtown { 6.0 } else { 10.0 };
        let mut d = 3.0;
        let mut side = 1.0;
        while d < len {
            let (p, dir) = s.at(d);
            let n = perp(dir) * side;
            let lamp = p + n * (s.width / 2.0 + 0.85);
            if grid.at(lamp) == Terrain::Sidewalk {
                wild.push((FeatureKind::Streetlight, lamp, 1.0));
            }
            // Maples along residential streets.
            if !s.downtown && rng.random_bool(0.55) {
                let t = p + n * (s.half_span() + 0.7) + dir * 2.5;
                if matches!(grid.at(t), Terrain::Grass | Terrain::Yard) {
                    wild.push((FeatureKind::Tree(TreeKind::Maple), t, 1.0));
                }
            }
            side = -side;
            d += step;
        }
    }
    for (k, p, s) in wild {
        // Keep clear of doors, walkways and what is already there.
        let blocked = features.iter().any(|f| {
            let gap = (f.kind.solid_radius().max(0.4) + k.solid_radius().max(0.4)) * 1.1;
            f.pos.distance(p) < gap
        }) || buildings.iter().any(|b| b.door.step().distance(p) < 2.0 || b.rect.grow(0.4).contains(p));
        let t = grid.at(p);
        if blocked || t == Terrain::Water || t == Terrain::Building || (k != FeatureKind::Streetlight && t.paved()) {
            continue;
        }
        let seed = rng.random();
        add(&mut features, k, p, s, None, seed);
    }
    let home = buildings.iter().position(|b| b.kind == BuildingKind::Home).expect("the parser insists on a home");
    // The egg basket on the children's porch.
    {
        let hb = &buildings[home];
        let side = perp(hb.door.facing.vec());
        let p = hb.door.step() + side * 0.9;
        add(&mut features, FeatureKind::EggBasket, p, 1.0, Some(home), 7);
    }
    let index = FeatureIndex::new(spec.width, spec.height, &features);
    Town {
        name: spec.name.clone(),
        w: spec.width,
        h: spec.height,
        start: spec.start,
        streets: spec.streets.clone(),
        rivers: spec.rivers.clone(),
        areas: spec.areas.clone(),
        buildings,
        features,
        grid,
        index,
        home,
    }
}

fn rng_seed(seed: u64, p: Vec2) -> u32 {
    let h = seed ^ ((p.x * 977.0) as i64 as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15) ^ ((p.y * 613.0) as i64 as u64).rotate_left(29);
    (h ^ (h >> 32)) as u32
}

fn stamp(grid: &mut Grid, r: &Rect) {
    for (x, y) in grid.tiles_in(r.x, r.y, r.x1(), r.y1()).collect::<Vec<_>>() {
        grid.set(x, y, Terrain::Building);
    }
}

fn edge_of(grid: &Grid, x: i32, y: i32, t: Terrain) -> bool {
    [(1, 0), (-1, 0), (0, 1), (0, -1)].iter().any(|(dx, dy)| grid.get(x + dx, y + dy) != t)
}

fn near(grid: &Grid, x: i32, y: i32, t: Terrain, r: i32) -> bool {
    (-r..=r).any(|dy| (-r..=r).any(|dx| grid.get(x + dx, y + dy) == t))
}

fn door_point(rect: &Rect, facing: Dir, at: f32) -> Vec2 {
    match facing {
        Dir::S => Vec2::new(rect.x + rect.w * at, rect.y1()),
        Dir::N => Vec2::new(rect.x + rect.w * at, rect.y),
        Dir::E => Vec2::new(rect.x1(), rect.y + rect.h * at),
        Dir::W => Vec2::new(rect.x, rect.y + rect.h * at),
    }
}

fn place_building(id: usize, bs: &BuildingSpec, streets: &[Street], seed: u64) -> Building {
    let door = Door {
        at: door_point(&bs.rect, bs.door, bs.door_at),
        facing: bs.door,
    };
    let step = door.step() + bs.door.vec() * 1.5;
    let street = streets
        .iter()
        .filter(|s| matches!(s.kind, StreetKind::Street | StreetKind::Lane))
        .map(|s| (s, polyline_distance(step, &s.pts) - s.half_span()))
        .filter(|(_, d)| *d < 3.0)
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(s, _)| s.name.clone());
    let mut r = ChaCha8Rng::seed_from_u64(seed);
    Building {
        id,
        kind: bs.kind,
        name: bs.name.clone(),
        street,
        rect: bs.rect,
        door,
        say: bs.say.clone(),
        gives: bs.gives.clone(),
        style: Style {
            body: r.random_range(0..8),
            trim: r.random_range(0..4),
            roof: r.random_range(0..5),
            porch: true,
            stories: if bs.kind.is_commercial() { r.random_range(2..5) } else { 2 },
            turret: false,
        },
        seed,
    }
}

/// Whether a house could stand on `r`: every tile under it (and a margin)
/// is open grass, and no other building is close.
fn fits(grid: &Grid, buildings: &[Building], r: &Rect) -> bool {
    let m = r.grow(0.7);
    if m.x < 0.5 || m.y < 0.5 || m.x1() > grid.w as f32 - 0.5 || m.y1() > grid.h as f32 - 0.5 {
        return false;
    }
    let ok = grid
        .tiles_in(m.x, m.y, m.x1(), m.y1())
        .all(|(x, y)| matches!(grid.get(x, y), Terrain::Grass));
    ok && !buildings.iter().any(|b| b.rect.grow(1.2).intersects(r))
}

fn lay_out_houses(s: &Street, grid: &mut Grid, buildings: &mut Vec<Building>, rng: &mut ChaCha8Rng) {
    let len = s.length();
    for side in [-1.0f32, 1.0] {
        let mut d = rng.random_range(1.0..4.0);
        let mut number: u32 = if side < 0.0 { 1 } else { 2 } + 2 * rng.random_range(0..4);
        while d < len {
            let (p, dir) = s.at(d);
            let n = perp(dir) * side;
            let frontage = rng.random_range(4.2..5.8);
            let depth = rng.random_range(3.6..4.6);
            let setback = rng.random_range(1.7..2.7);
            let c = p + n * (s.half_span() + setback + depth / 2.0) + dir * (frontage / 2.0);
            // Houses stand square to the map: the frontage runs along x for
            // a street running east-west, along y for one running north-south.
            let along_x = dir.x.abs() >= dir.y.abs();
            let (w, h) = if along_x { (frontage, depth) } else { (depth, frontage) };
            let rect = Rect::new(c.x - w / 2.0, c.y - h / 2.0, w, h);
            if fits(grid, buildings, &rect) {
                let facing = Dir::of(-n);
                let at = rng.random_range(0.3..0.7);
                let id = buildings.len();
                let seed: u64 = rng.random();
                let mut sr = ChaCha8Rng::seed_from_u64(seed);
                buildings.push(Building {
                    id,
                    kind: BuildingKind::House,
                    name: format!("{number} {}", s.name),
                    street: Some(s.name.clone()),
                    rect,
                    door: Door {
                        at: door_point(&rect, facing, at),
                        facing,
                    },
                    say: None,
                    gives: None,
                    style: Style {
                        body: sr.random_range(0..8),
                        trim: sr.random_range(0..4),
                        roof: sr.random_range(0..5),
                        porch: sr.random_bool(0.7),
                        stories: sr.random_range(1..3),
                        turret: sr.random_bool(0.18),
                    },
                    seed,
                });
                stamp(grid, &rect);
                number += 2 * rng.random_range(1..4);
                d += frontage + rng.random_range(1.6..3.2);
            } else {
                d += 1.3;
            }
        }
    }
}

/// A path from a door straight out to whatever paved way it faces.
fn walkway(grid: &mut Grid, b: &Building) {
    let dir = b.door.facing.vec();
    let mut p = b.door.at + dir * 0.5;
    let mut cells = Vec::new();
    for _ in 0..12 {
        let t = grid.at(p);
        if t.paved() || t == Terrain::Water {
            for (x, y) in cells {
                grid.set(x, y, Terrain::Path);
            }
            return;
        }
        if t == Terrain::Building {
            return;
        }
        cells.push((p.x.floor() as i32, p.y.floor() as i32));
        p += dir;
    }
}

fn yard(grid: &Grid, b: &Building, rng: &mut ChaCha8Rng, put: &mut dyn FnMut(FeatureKind, Vec2, f32)) {
    if !matches!(b.kind, BuildingKind::House | BuildingKind::Home) && !b.kind.is_commercial() {
        return;
    }
    let out = b.door.facing.vec();
    let side = perp(out);
    let step = b.door.step();
    let open = |p: Vec2| matches!(grid.at(p), Terrain::Grass | Terrain::Yard | Terrain::Path | Terrain::Sidewalk);
    let grassy = |p: Vec2| matches!(grid.at(p), Terrain::Grass | Terrain::Yard);
    if b.kind.is_commercial() {
        if rng.random_bool(0.5) {
            let p = step + side * 1.1;
            if open(p) {
                put(FeatureKind::Pumpkin, p, 1.0);
            }
        }
        return;
    }
    if rng.random_bool(0.6) {
        let s = if rng.random_bool(0.5) { 1.0 } else { -1.0 };
        let p = step + side * (0.75 * s);
        if open(p) {
            put(FeatureKind::Pumpkin, p, 1.0);
        }
    }
    if rng.random_bool(0.035) {
        let p = step - side * 0.8;
        if open(p) {
            put(FeatureKind::CandyBowl, p, 1.0);
        }
    }
    // Shrubs either side of the front.
    let half = if b.door.facing == Dir::N || b.door.facing == Dir::S { b.rect.w } else { b.rect.h } / 2.0;
    let front = b.rect.center() + out * ((if b.door.facing == Dir::N || b.door.facing == Dir::S { b.rect.h } else { b.rect.w }) / 2.0 + 0.6);
    for s in [-1.0f32, 1.0] {
        if rng.random_bool(0.55) {
            let p = front + side * s * (half - 0.4);
            if grassy(p) && p.distance(step) > 1.0 {
                put(FeatureKind::Bush, p, rng.random_range(0.8..1.2));
            }
        }
    }
    let lawn = step + out * rng.random_range(0.9..1.5) + side * rng.random_range(-2.6..2.6);
    let roll: f32 = rng.random();
    if grassy(lawn) && lawn.distance(step) > 1.2 {
        if roll < 0.28 {
            put(FeatureKind::LeafPile, lawn, rng.random_range(0.9..1.3));
        } else if roll < 0.62 {
            let corner = front + side * (half + 0.9) * if rng.random_bool(0.5) { 1.0 } else { -1.0 };
            if grassy(corner) {
                let kind = if rng.random_bool(0.045) {
                    FeatureKind::TpTree
                } else {
                    FeatureKind::Tree(if rng.random_bool(0.7) { TreeKind::Maple } else { TreeKind::Oak })
                };
                put(kind, corner, 1.0);
            }
        } else if roll < 0.68 {
            put(FeatureKind::Puddle, lawn, rng.random_range(0.8..1.2));
        }
    }
    // The back yard.
    let back = b.rect.center() - out * ((if b.door.facing == Dir::N || b.door.facing == Dir::S { b.rect.h } else { b.rect.w }) / 2.0 + 1.3);
    let roll: f32 = rng.random();
    if grassy(back) {
        if roll < 0.05 {
            put(FeatureKind::Raspberry, back, rng.random_range(1.0..1.4));
        } else if roll < 0.3 {
            put(FeatureKind::Bush, back + side * rng.random_range(-1.5..1.5), rng.random_range(0.9..1.4));
        } else if roll < 0.45 {
            put(FeatureKind::LeafPile, back, 1.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::town::{MONTPELIER, parse};

    fn montpelier() -> Town {
        build(&parse(MONTPELIER).unwrap())
    }

    #[test]
    fn montpelier_has_neighbourhoods() {
        let t = montpelier();
        let houses = t.buildings.iter().filter(|b| b.kind == BuildingKind::House).count();
        assert!(houses > 100, "only {houses} houses");
        assert!(t.features.iter().filter(|f| matches!(f.kind, FeatureKind::Tree(_))).count() > 300);
        assert!(t.features.iter().any(|f| f.kind == FeatureKind::Pumpkin));
        assert!(t.features.iter().any(|f| f.kind == FeatureKind::LeafPile));
        assert!(t.grid.count(Terrain::Bridge) > 10);
        // Every house has a number on its street.
        for b in t.buildings.iter().filter(|b| b.kind == BuildingKind::House) {
            assert!(b.name.split(' ').next().unwrap().parse::<u32>().is_ok(), "{}", b.name);
        }
    }

    #[test]
    fn every_door_can_be_reached_from_the_start() {
        let t = montpelier();
        let field = crate::world::nav::Field::from(&t, t.start, false);
        for b in &t.buildings {
            if b.kind == BuildingKind::Tower {
                continue;
            }
            assert!(field.reachable(b.door.step()), "{} at {:?} cannot be reached", b.name, b.door.step());
        }
    }

    #[test]
    fn the_start_is_open() {
        let t = montpelier();
        assert!(t.clear(t.start, 0.3, false));
        assert!(t.describe(t.start).contains("Street"), "{}", t.describe(t.start));
    }

    #[test]
    fn buildings_do_not_overlap() {
        let t = montpelier();
        for a in &t.buildings {
            for b in &t.buildings {
                if a.id < b.id {
                    assert!(!a.rect.intersects(&b.rect), "{} overlaps {}", a.name, b.name);
                }
            }
        }
    }
}
