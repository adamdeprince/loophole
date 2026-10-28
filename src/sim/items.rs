//! Things lying about: spilled candy, litter worth patching with, eggs and
//! the other thing hens leave. Also the changeable features (berries
//! regrowing, leaf piles re-raked, bowls refilled).

use super::candy::{Candy, Mat};
use super::{DT, FeatState, Game, Who};
use crate::world::grid::Terrain;
use crate::world::plan::Town;
use crate::world::town::FeatureKind;
use glam::Vec2;
use rand::RngExt;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ItemKind {
    Candy(Candy),
    Mat(Mat),
    Egg,
    Poop,
    /// What Gravy thought of the candy.
    Barf,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Item {
    pub id: u32,
    pub kind: ItemKind,
    pub pos: Vec2,
    pub vel: Vec2,
    /// Height above the ground while flying (tiles).
    pub z: f32,
    pub vz: f32,
    pub age: f32,
    /// Whose it was (for giving it back).
    pub from: Option<Who>,
    /// Somebody has it in hand.
    pub carried: Option<Who>,
}

impl Item {
    pub fn new(id: u32, kind: ItemKind, pos: Vec2, vel: Vec2, from: Option<Who>) -> Item {
        let flying = vel.length_squared() > 0.0;
        Item {
            id,
            kind,
            pos,
            vel,
            z: if flying { 0.3 } else { 0.0 },
            vz: if flying { 3.2 } else { 0.0 },
            age: 0.0,
            from,
            carried: None,
        }
    }

    /// Seconds before it goes on its own (eaten by something, blown away,
    /// trodden in); None for things that stay until somebody takes them.
    fn lifetime(&self) -> Option<f32> {
        match self.kind {
            ItemKind::Candy(_) => Some(240.0),
            ItemKind::Mat(_) => Some(600.0),
            ItemKind::Poop | ItemKind::Barf => Some(200.0),
            ItemKind::Egg => None,
        }
    }
}

pub fn initial_feats(town: &Town) -> Vec<FeatState> {
    town.features.iter().map(|f| fresh(f.kind)).collect()
}

fn fresh(kind: FeatureKind) -> FeatState {
    FeatState {
        used: false,
        regrow: 0.0,
        n: match kind {
            FeatureKind::CandyBowl => 14,
            FeatureKind::FlyerPole => 4,
            FeatureKind::TpTree => 5,
            _ => 0,
        },
    }
}

/// A new night: everything back as it was (the egg basket keeps its eggs).
pub fn reset_feats(g: &mut Game) {
    let town = g.town.clone();
    for (f, st) in town.features.iter().zip(g.feats.iter_mut()) {
        if f.kind != FeatureKind::EggBasket {
            *st = fresh(f.kind);
        }
    }
}

pub fn step(g: &mut Game) {
    let town = g.town.clone();
    // Things in flight land; things on the ground age.
    for it in &mut g.items {
        it.age += DT;
        if it.carried.is_some() {
            continue;
        }
        if it.z > 0.0 || it.vz != 0.0 {
            it.vz -= 14.0 * DT;
            it.z += it.vz * DT;
            let next = it.pos + it.vel * DT;
            if town.grid.at(next).walkable() {
                it.pos = next;
            } else {
                it.vel = -it.vel * 0.3;
            }
            if it.z <= 0.0 {
                it.z = 0.0;
                if it.vz.abs() > 1.5 {
                    it.vz = -it.vz * 0.35;
                } else {
                    it.vz = 0.0;
                    it.vel = Vec2::ZERO;
                }
                it.vel *= 0.6;
            }
        }
    }
    // Candy that lands in the river is gone.
    g.items.retain(|it| {
        let wet = it.carried.is_none() && it.z <= 0.0 && town.grid.at(it.pos) == Terrain::Water && it.kind != ItemKind::Egg;
        let old = it.lifetime().is_some_and(|l| it.age > l) && it.carried.is_none();
        !wet && !old
    });
    // Features grow back.
    for (i, st) in g.feats.iter_mut().enumerate() {
        if st.used && st.regrow > 0.0 {
            st.regrow -= DT;
            if st.regrow <= 0.0 {
                let kind = town.features[i].kind;
                st.used = false;
                if kind == FeatureKind::CandyBowl {
                    // Somebody noticed and refilled it.
                    st.n = 10;
                }
            }
        }
    }
    if g.tick % 120 == 0 {
        scatter_litter(g, false);
    }
}

/// Keeps a little litter about downtown and on the paths: paper, napkins
/// and the gum on the sidewalk. The amount near the raccoon is topped up
/// towards a density, so there is always some to find and never a heap.
pub fn scatter_litter(g: &mut Game, initial: bool) {
    let town = g.town.clone();
    let around = g.raccoon.pos;
    let radius = if initial { f32::INFINITY } else { 30.0 };
    let litter = |k: &ItemKind| matches!(k, ItemKind::Mat(Mat::Paper | Mat::Napkin | Mat::Gum));
    let have = g
        .items
        .iter()
        .filter(|it| litter(&it.kind) && it.pos.distance(around) < radius.min(1e6))
        .count();
    let want = if initial { 70 } else { 9 };
    let mut tries = 0;
    let mut placed = have;
    while placed < want && tries < want * 40 {
        tries += 1;
        let p = if initial {
            Vec2::new(g.rng.random_range(1.0..town.w as f32 - 1.0), g.rng.random_range(1.0..town.h as f32 - 1.0))
        } else {
            let a: f32 = g.rng.random_range(0.0..std::f32::consts::TAU);
            let d: f32 = g.rng.random_range(14.0..30.0);
            around + Vec2::new(a.cos(), a.sin()) * d
        };
        let t = town.grid.at(p);
        if !matches!(t, Terrain::Sidewalk | Terrain::Path | Terrain::Plaza | Terrain::Parking) {
            continue;
        }
        let downtown = town.streets.iter().any(|s| s.downtown && crate::world::polyline_distance(p, &s.pts) < s.half_span() + 1.0);
        if !downtown && g.rng.random_bool(0.6) {
            continue;
        }
        let m = match g.rng.random_range(0..10) {
            0..=3 => Mat::Paper,
            4..=6 => Mat::Gum,
            _ => Mat::Napkin,
        };
        let id = g.next_item;
        g.next_item += 1;
        g.items.push(Item::new(id, ItemKind::Mat(m), p, Vec2::ZERO, None));
        placed += 1;
    }
}

/// Puts something on the ground at `pos`.
pub fn drop_item(g: &mut Game, kind: ItemKind, pos: Vec2, from: Option<Who>) -> u32 {
    let id = g.next_item;
    g.next_item += 1;
    g.items.push(Item::new(id, kind, pos, Vec2::ZERO, from));
    id
}

pub fn find(g: &Game, id: u32) -> Option<usize> {
    g.items.iter().position(|it| it.id == id)
}

/// The nearest item on the ground matching `want`, within `r`.
pub fn nearest(g: &Game, p: Vec2, r: f32, want: impl Fn(&Item) -> bool) -> Option<u32> {
    g.items
        .iter()
        .filter(|it| it.carried.is_none() && it.z <= 0.05 && it.pos.distance(p) <= r && want(it))
        .min_by(|a, b| a.pos.distance(p).total_cmp(&b.pos.distance(p)))
        .map(|it| it.id)
}
