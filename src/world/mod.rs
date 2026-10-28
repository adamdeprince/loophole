//! The town: its description (streets, rivers, buildings and features read
//! from a text file), the terrain grid derived from it, the houses laid out
//! along its streets, and the queries the simulation asks of it.

pub mod plan;
pub mod grid;
pub mod nav;
pub mod town;

use glam::Vec2;

/// The four ways a door can face, and a creature can look.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub enum Dir {
    N,
    E,
    S,
    W,
}

impl Dir {
    pub fn vec(self) -> Vec2 {
        match self {
            Dir::N => Vec2::new(0.0, -1.0),
            Dir::E => Vec2::new(1.0, 0.0),
            Dir::S => Vec2::new(0.0, 1.0),
            Dir::W => Vec2::new(-1.0, 0.0),
        }
    }

    /// The direction a vector mostly points in.
    pub fn of(v: Vec2) -> Dir {
        if v.x.abs() > v.y.abs() {
            if v.x > 0.0 { Dir::E } else { Dir::W }
        } else if v.y > 0.0 {
            Dir::S
        } else {
            Dir::N
        }
    }

    pub fn opposite(self) -> Dir {
        match self {
            Dir::N => Dir::S,
            Dir::E => Dir::W,
            Dir::S => Dir::N,
            Dir::W => Dir::E,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Dir::N => "up",
            Dir::E => "right",
            Dir::S => "down",
            Dir::W => "left",
        }
    }

    pub fn parse(s: &str) -> Option<Dir> {
        match s {
            "N" | "n" => Some(Dir::N),
            "E" | "e" => Some(Dir::E),
            "S" | "s" => Some(Dir::S),
            "W" | "w" => Some(Dir::W),
            _ => None,
        }
    }
}

/// An axis-aligned rectangle in tiles.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    pub fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Rect { x, y, w, h }
    }

    pub fn x1(&self) -> f32 {
        self.x + self.w
    }

    pub fn y1(&self) -> f32 {
        self.y + self.h
    }

    pub fn center(&self) -> Vec2 {
        Vec2::new(self.x + self.w / 2.0, self.y + self.h / 2.0)
    }

    pub fn contains(&self, p: Vec2) -> bool {
        p.x >= self.x && p.x < self.x1() && p.y >= self.y && p.y < self.y1()
    }

    pub fn grow(&self, m: f32) -> Rect {
        Rect::new(self.x - m, self.y - m, self.w + 2.0 * m, self.h + 2.0 * m)
    }

    pub fn intersects(&self, o: &Rect) -> bool {
        self.x < o.x1() && o.x < self.x1() && self.y < o.y1() && o.y < self.y1()
    }

    /// The distance from `p` to the nearest point of the rectangle (zero
    /// inside).
    pub fn distance(&self, p: Vec2) -> f32 {
        let dx = (self.x - p.x).max(0.0).max(p.x - self.x1());
        let dy = (self.y - p.y).max(0.0).max(p.y - self.y1());
        (dx * dx + dy * dy).sqrt()
    }
}

/// Distance from `p` to the segment `a`-`b`, and the parameter (0 to 1) of
/// the nearest point along it.
pub fn seg_distance(p: Vec2, a: Vec2, b: Vec2) -> (f32, f32) {
    let ab = b - a;
    let len2 = ab.length_squared();
    if len2 <= f32::EPSILON {
        return (p.distance(a), 0.0);
    }
    let t = ((p - a).dot(ab) / len2).clamp(0.0, 1.0);
    (p.distance(a + ab * t), t)
}

/// Distance from `p` to a polyline.
pub fn polyline_distance(p: Vec2, pts: &[Vec2]) -> f32 {
    match pts {
        [] => f32::INFINITY,
        [a] => p.distance(*a),
        _ => pts.windows(2).map(|w| seg_distance(p, w[0], w[1]).0).fold(f32::INFINITY, f32::min),
    }
}

/// Whether `p` is inside a polygon (even-odd).
pub fn in_polygon(p: Vec2, poly: &[Vec2]) -> bool {
    let mut inside = false;
    let n = poly.len();
    if n < 3 {
        return false;
    }
    let mut j = n - 1;
    for i in 0..n {
        let (a, b) = (poly[i], poly[j]);
        if (a.y > p.y) != (b.y > p.y) && p.x < (b.x - a.x) * (p.y - a.y) / (b.y - a.y) + a.x {
            inside = !inside;
        }
        j = i;
    }
    inside
}

/// The bounding rectangle of some points, grown by `m`.
pub fn bounds_of(pts: &[Vec2], m: f32) -> Rect {
    let mut x0 = f32::INFINITY;
    let mut y0 = f32::INFINITY;
    let mut x1 = f32::NEG_INFINITY;
    let mut y1 = f32::NEG_INFINITY;
    for p in pts {
        x0 = x0.min(p.x);
        y0 = y0.min(p.y);
        x1 = x1.max(p.x);
        y1 = y1.max(p.y);
    }
    if !x0.is_finite() {
        return Rect::new(0.0, 0.0, 0.0, 0.0);
    }
    Rect::new(x0 - m, y0 - m, x1 - x0 + 2.0 * m, y1 - y0 + 2.0 * m)
}
