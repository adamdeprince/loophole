//! Finding the way: distance fields (everyone heading for one place, such
//! as Loki taking an egg home) and A* paths (one walker, one destination),
//! over the terrain grid. Trees and poles are left to steering.

use super::plan::Town;
use super::grid::{Terrain, centre};
use glam::Vec2;
use std::cmp::Reverse;
use std::collections::BinaryHeap;

const ORTHO: u32 = 10;
const DIAG: u32 = 14;

/// The cost of entering a tile, or None where the walker cannot go.
fn cost(t: Terrain, swim: bool) -> Option<u32> {
    cost_on(t, swim, 2)
}

fn cost_on(t: Terrain, swim: bool, road: u32) -> Option<u32> {
    match t {
        Terrain::Building => None,
        Terrain::Water if !swim => None,
        Terrain::Water => Some(4),
        Terrain::Woods => Some(2),
        // Walkers keep to sidewalks and paths when they can.
        Terrain::Road => Some(road),
        _ => Some(1),
    }
}

const NEIGHBOURS: [(i32, i32, u32); 8] = [
    (1, 0, ORTHO),
    (-1, 0, ORTHO),
    (0, 1, ORTHO),
    (0, -1, ORTHO),
    (1, 1, DIAG),
    (1, -1, DIAG),
    (-1, 1, DIAG),
    (-1, -1, DIAG),
];

/// Distances from every tile to a goal.
#[derive(Clone, Debug)]
pub struct Field {
    w: i32,
    h: i32,
    dist: Vec<u32>,
    swim: bool,
}

impl Field {
    /// Dijkstra outwards from the tile holding `goal`.
    pub fn from(town: &Town, goal: Vec2, swim: bool) -> Field {
        let g = &town.grid;
        let n = (g.w as usize) * (g.h as usize);
        let mut dist = vec![u32::MAX; n];
        let mut heap = BinaryHeap::new();
        let (gx, gy) = (goal.x.floor() as i32, goal.y.floor() as i32);
        if g.in_bounds(gx, gy) {
            dist[(gy * g.w + gx) as usize] = 0;
            heap.push(Reverse((0u32, gx, gy)));
        }
        while let Some(Reverse((d, x, y))) = heap.pop() {
            if d > dist[(y * g.w + x) as usize] {
                continue;
            }
            for &(dx, dy, step) in &NEIGHBOURS {
                let (nx, ny) = (x + dx, y + dy);
                let Some(c) = cost(g.get(nx, ny), swim) else { continue };
                // No cutting corners past walls.
                if dx != 0 && dy != 0 && (cost(g.get(x + dx, y), swim).is_none() || cost(g.get(x, y + dy), swim).is_none()) {
                    continue;
                }
                let nd = d.saturating_add(step * c);
                let i = (ny * g.w + nx) as usize;
                if nd < dist[i] {
                    dist[i] = nd;
                    heap.push(Reverse((nd, nx, ny)));
                }
            }
        }
        Field {
            w: g.w,
            h: g.h,
            dist,
            swim,
        }
    }

    fn d(&self, x: i32, y: i32) -> u32 {
        if x < 0 || y < 0 || x >= self.w || y >= self.h {
            u32::MAX
        } else {
            self.dist[(y * self.w + x) as usize]
        }
    }

    pub fn reachable(&self, p: Vec2) -> bool {
        self.d(p.x.floor() as i32, p.y.floor() as i32) != u32::MAX
    }

    /// Distance in tiles (roughly) from `p` to the goal.
    pub fn distance(&self, p: Vec2) -> Option<f32> {
        let d = self.d(p.x.floor() as i32, p.y.floor() as i32);
        (d != u32::MAX).then_some(d as f32 / ORTHO as f32)
    }

    /// The centre of the neighbouring tile nearest the goal: where to head
    /// next from `p`.
    pub fn next(&self, p: Vec2) -> Option<Vec2> {
        let (x, y) = (p.x.floor() as i32, p.y.floor() as i32);
        let here = self.d(x, y);
        if here == 0 || here == u32::MAX {
            return None;
        }
        NEIGHBOURS
            .iter()
            .map(|&(dx, dy, _)| (self.d(x + dx, y + dy), dx, dy))
            .filter(|(d, _, _)| *d < here)
            .min_by_key(|(d, _, _)| *d)
            .map(|(_, dx, dy)| centre(x + dx, y + dy))
    }

    pub fn swims(&self) -> bool {
        self.swim
    }
}

/// An A* path of tile centres from `from` to `to`, smoothed so that it
/// only turns where it has to. None if there is no way.
pub fn path(town: &Town, from: Vec2, to: Vec2, swim: bool) -> Option<Vec<Vec2>> {
    path_on(town, from, to, swim, 2)
}

/// A path for somebody who keeps off roads: a road tile costs `road`
/// against a sidewalk's one. Corners are only cut where the straight line
/// stays off the road too.
pub fn path_on(town: &Town, from: Vec2, to: Vec2, swim: bool, road: u32) -> Option<Vec<Vec2>> {
    let cost = |t: Terrain, swim: bool| cost_on(t, swim, road);
    let g = &town.grid;
    let (sx, sy) = (from.x.floor() as i32, from.y.floor() as i32);
    let (tx, ty) = (to.x.floor() as i32, to.y.floor() as i32);
    if !g.in_bounds(sx, sy) || cost(g.get(tx, ty), swim).is_none() {
        return None;
    }
    let n = (g.w as usize) * (g.h as usize);
    let idx = |x: i32, y: i32| (y * g.w + x) as usize;
    let mut best = vec![u32::MAX; n];
    let mut came = vec![u32::MAX; n];
    let h = |x: i32, y: i32| {
        let dx = (x - tx).unsigned_abs();
        let dy = (y - ty).unsigned_abs();
        ORTHO * dx.max(dy) + (DIAG - ORTHO) * dx.min(dy)
    };
    let mut heap = BinaryHeap::new();
    best[idx(sx, sy)] = 0;
    heap.push(Reverse((h(sx, sy), 0u32, sx, sy)));
    let mut found = false;
    while let Some(Reverse((_, d, x, y))) = heap.pop() {
        if (x, y) == (tx, ty) {
            found = true;
            break;
        }
        if d > best[idx(x, y)] {
            continue;
        }
        for &(dx, dy, step) in &NEIGHBOURS {
            let (nx, ny) = (x + dx, y + dy);
            let Some(c) = cost(g.get(nx, ny), swim) else { continue };
            if dx != 0 && dy != 0 && (cost(g.get(x + dx, y), swim).is_none() || cost(g.get(x, y + dy), swim).is_none()) {
                continue;
            }
            // A trunk or a pole in the tile: possible, but better round.
            let c = if town.is_cluttered(nx, ny) { c + 3 } else { c };
            let nd = d.saturating_add(step * c);
            let i = idx(nx, ny);
            if nd < best[i] {
                best[i] = nd;
                came[i] = idx(x, y) as u32;
                heap.push(Reverse((nd + h(nx, ny), nd, nx, ny)));
            }
        }
    }
    if !found {
        return None;
    }
    let mut tiles = vec![(tx, ty)];
    let mut cur = idx(tx, ty);
    while cur != idx(sx, sy) {
        let prev = came[cur];
        if prev == u32::MAX {
            return None;
        }
        cur = prev as usize;
        tiles.push(((cur as i32) % g.w, (cur as i32) / g.w));
    }
    tiles.reverse();
    // String pulling: keep a waypoint only where the straight line from the
    // last kept one stops being walkable.
    let mut out = vec![from];
    let mut anchor = from;
    for i in 1..tiles.len() {
        let next = if i + 1 < tiles.len() { centre(tiles[i + 1].0, tiles[i + 1].1) } else { to };
        if !line_clear(town, anchor, next, swim) || (road > 2 && crosses_road(town, anchor, next)) {
            let c = centre(tiles[i].0, tiles[i].1);
            out.push(c);
            anchor = c;
        }
    }
    out.push(to);
    Some(out)
}

/// Whether the straight line from `a` to `b` touches a road anywhere it did
/// not start or end.
fn crosses_road(town: &Town, a: Vec2, b: Vec2) -> bool {
    let start = town.grid.at(a) == Terrain::Road;
    let end = town.grid.at(b) == Terrain::Road;
    if start && end {
        return false;
    }
    let len = a.distance(b);
    let steps = (len / 0.25).ceil().max(1.0) as i32;
    (1..steps).any(|i| town.grid.at(a.lerp(b, i as f32 / steps as f32)) == Terrain::Road)
}

/// Whether a walker can go straight from `a` to `b`: no walls, no water
/// (unless swimming), and no trunk or pole in the way.
pub fn line_clear(town: &Town, a: Vec2, b: Vec2, swim: bool) -> bool {
    let len = a.distance(b);
    walls_clear(town, a, b, swim) && !town.features_near((a + b) / 2.0, len / 2.0 + 1.0).any(|f| {
        let s = f.kind.solid_radius();
        s > 0.0 && crate::world::seg_distance(f.pos, a, b).0 < s + 0.25
    })
}

/// Whether the straight line from `a` to `b` stays clear of walls (and of
/// water, unless swimming).
pub fn walls_clear(town: &Town, a: Vec2, b: Vec2, swim: bool) -> bool {
    let len = a.distance(b);
    let steps = (len / 0.25).ceil().max(1.0) as i32;
    (0..=steps).all(|i| {
        let p = a.lerp(b, i as f32 / steps as f32);
        // A little width, so paths do not graze corners.
        [Vec2::ZERO, Vec2::new(0.3, 0.0), Vec2::new(-0.3, 0.0), Vec2::new(0.0, 0.3), Vec2::new(0.0, -0.3)]
            .iter()
            .all(|o| cost(town.grid.at(p + *o), swim).is_some())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::{plan, town};

    #[test]
    fn a_path_from_the_state_house_to_the_college() {
        let t = plan::build(&town::parse(town::MONTPELIER).unwrap());
        let sh = t.buildings.iter().find(|b| b.kind == town::BuildingKind::StateHouse).unwrap();
        let col = t.buildings.iter().find(|b| b.kind == town::BuildingKind::College).unwrap();
        let p = path(&t, sh.door.step(), col.door.step(), false).expect("a way across town");
        assert!(p.len() >= 2);
        // Walls and water only: trunks and poles are left to steering.
        for w in p.windows(2) {
            assert!(walls_clear(&t, w[0], w[1], false), "{:?} to {:?}", w[0], w[1]);
        }
    }
}
