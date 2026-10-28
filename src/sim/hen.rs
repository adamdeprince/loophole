//! Gullinkombi, the hen: no plan at all, ruled by her stomach. Satisfied,
//! she potters about and gets in the way. Peckish, candy within a few
//! steps catches her eye (and the bag). Hungry, she goes for food from
//! across the street. Full, she lays eggs and leaves droppings.

use super::candy::Candy;
use super::discover::Id;
use super::items::{self, ItemKind};
use super::raccoon::{self, Act};
use super::{DT, Game, SimEvent, Who, facing_of, lines, move_body, steer};
use crate::world::Dir;
use crate::world::plan::Town;
use crate::world::town::FeatureKind;
use glam::Vec2;
use rand::RngExt;
use rand_chacha::ChaCha8Rng;

pub const RADIUS: f32 = 0.22;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Food {
    Item(u32),
    Berries(usize),
    Bag,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum State {
    Wander { to: Vec2, t: f32 },
    Peck { t: f32 },
    /// Plonked down right where somebody wants to walk.
    Sit { t: f32 },
    GoFood { food: Food, t: f32 },
    Eat { t: f32 },
    Lay { t: f32 },
    Poop { t: f32 },
    Flee { from: Vec2, t: f32 },
    /// Off to sit down somewhere particular (in somebody's way).
    Plonk { to: Vec2, t: f32 },
}

#[derive(Clone, Debug)]
pub struct Hen {
    pub pos: Vec2,
    pub prev: Vec2,
    pub vel: Vec2,
    pub facing: Dir,
    pub state: State,
    /// 0 satisfied to 1 starving.
    pub hunger: f32,
    /// How much she has eaten lately: a full hen lays and poops more.
    pub fullness: f32,
    pub egg_t: f32,
    pub poop_t: f32,
    pub held_by: Option<Who>,
    pub cluck_cd: f32,
    pub stride: f32,
    pub path: Vec<Vec2>,
}

impl Hen {
    pub fn new(town: &Town, rng: &mut ChaCha8Rng) -> Hen {
        let at = town.start + Vec2::new(2.5, 2.0);
        Hen {
            pos: at,
            prev: at,
            vel: Vec2::ZERO,
            facing: Dir::E,
            state: State::Peck { t: 2.0 },
            hunger: 0.35,
            fullness: 0.3,
            egg_t: rng.random_range(30.0..60.0),
            poop_t: rng.random_range(12.0..25.0),
            held_by: None,
            cluck_cd: 4.0,
            stride: 0.0,
            path: Vec::new(),
        }
    }

    pub fn mood(&self) -> &'static str {
        if self.fullness > 0.6 {
            "full"
        } else if self.hunger > 0.7 {
            "hungry"
        } else if self.hunger > 0.3 {
            "peckish"
        } else {
            "content"
        }
    }

    /// Doing something she will not be picked up in the middle of.
    pub fn busy(&self) -> bool {
        matches!(self.state, State::Lay { .. })
    }

    fn full(&self) -> bool {
        self.fullness > 0.6
    }
}

pub fn step(g: &mut Game) {
    g.hen.prev = g.hen.pos;
    let h = &mut g.hen;
    h.hunger = (h.hunger + DT / 150.0).min(1.0);
    h.fullness = (h.fullness - DT / 140.0).max(0.0);
    h.cluck_cd = (h.cluck_cd - DT).max(0.0);
    if let Some(holder) = h.held_by {
        // In somebody's arms (or two somebodies').
        let pos = match holder {
            Who::Loki => g.kids[0].pos + g.kids[0].facing.vec() * 0.25,
            Who::Skadi => g.kids[1].pos + g.kids[1].facing.vec() * 0.25,
            _ => h.pos,
        };
        let tug = matches!(g.kids[0].state, super::kids::State::Tug { .. }) && matches!(g.kids[1].state, super::kids::State::Tug { .. });
        g.hen.pos = if tug { (g.kids[0].pos + g.kids[1].pos) / 2.0 } else { pos };
        g.hen.vel = Vec2::ZERO;
        return;
    }
    // Eggs and the other thing.
    h.egg_t -= DT;
    h.poop_t -= DT;
    if h.egg_t <= 0.0 && !matches!(h.state, State::Lay { .. }) {
        h.state = State::Lay { t: 1.6 };
        h.egg_t = if h.full() { g.rng.random_range(22.0..34.0) } else { g.rng.random_range(70.0..110.0) };
    }
    if g.hen.poop_t <= 0.0 && matches!(g.hen.state, State::Wander { .. } | State::Peck { .. } | State::Sit { .. }) {
        g.hen.state = State::Poop { t: 0.6 };
        g.hen.poop_t = if g.hen.full() { g.rng.random_range(8.0..13.0) } else { g.rng.random_range(30.0..45.0) };
    }
    think(g);
    act(g);
    raccoon_contact(g);
}

/// Picks something to eat, depending on how hungry she is.
fn find_food(g: &Game) -> Option<Food> {
    let h = &g.hen;
    let reach = if h.hunger > 0.7 {
        12.0
    } else if h.hunger > 0.3 {
        3.5
    } else {
        return None;
    };
    if let Some(id) = items::nearest(g, h.pos, reach, |it| matches!(it.kind, ItemKind::Candy(_))) {
        return Some(Food::Item(id));
    }
    let bag_reach = if h.hunger > 0.7 { 10.0 } else { 2.2 };
    if !g.raccoon.bag.is_empty() && g.raccoon.pos.distance(h.pos) < bag_reach {
        return Some(Food::Bag);
    }
    g.town
        .features_near(h.pos, reach)
        .filter(|f| f.kind == FeatureKind::Raspberry && !g.feats[f.id].used)
        .min_by(|a, b| a.pos.distance(h.pos).total_cmp(&b.pos.distance(h.pos)))
        .map(|f| Food::Berries(f.id))
}

fn think(g: &mut Game) {
    let free = matches!(g.hen.state, State::Wander { .. } | State::Peck { .. } | State::Sit { .. } | State::Plonk { .. });
    if !free {
        return;
    }
    if g.tick % 20 == 0 {
        if let Some(food) = find_food(g) {
            g.hen.state = State::GoFood { food, t: 14.0 };
            if g.hen.cluck_cd <= 0.0 {
                g.say(Who::Hen, "Bok? BOK!");
                g.hen.cluck_cd = 8.0;
            }
        }
    }
}

fn walk(g: &mut Game, to: Vec2, speed: f32) -> f32 {
    let town = g.town.clone();
    let h = &mut g.hen;
    let left = h.pos.distance(to);
    let mut aim = to;
    if !crate::world::nav::line_clear(&town, h.pos, to, false) {
        if h.path.last().is_none_or(|p| p.distance(to) > 1.5) {
            h.path = crate::world::nav::path(&town, h.pos, to, false).unwrap_or_default();
        }
        while h.path.len() > 1 && h.path[0].distance(h.pos) < 0.4 {
            h.path.remove(0);
        }
        if let Some(&p) = h.path.first() {
            aim = p;
        }
    } else {
        h.path.clear();
    }
    let d = aim - h.pos;
    let want = if d.length() > 0.05 { d.normalize() * speed.min(d.length() / DT) } else { Vec2::ZERO };
    h.vel = steer(h.vel, want, 20.0);
    h.facing = facing_of(h.vel, h.facing);
    let before = h.pos;
    let dv = h.vel * DT;
    move_body(&town, &mut h.pos, dv, RADIUS, false);
    h.stride += h.pos.distance(before);
    left
}

fn stop(g: &mut Game) {
    g.hen.vel = Vec2::ZERO;
}

fn act(g: &mut Game) {
    let state = g.hen.state;
    match state {
        State::Wander { to, t } => {
            let t = t - DT;
            if walk(g, to, 1.3) < 0.3 || t <= 0.0 {
                g.hen.state = next_idle(g);
            } else {
                g.hen.state = State::Wander { to, t };
            }
        }
        State::Peck { t } | State::Sit { t } => {
            stop(g);
            let t = t - DT;
            if t <= 0.0 {
                g.hen.state = next_idle(g);
            } else {
                g.hen.state = if matches!(state, State::Peck { .. }) { State::Peck { t } } else { State::Sit { t } };
            }
        }
        State::GoFood { food, t } => {
            let t = t - DT;
            let target = match food {
                Food::Item(id) => items::find(g, id).filter(|&i| g.items[i].carried.is_none()).map(|i| g.items[i].pos),
                Food::Berries(f) => (!g.feats[f].used).then(|| g.town.features[f].pos),
                Food::Bag => (!g.raccoon.bag.is_empty()).then_some(g.raccoon.pos),
            };
            let Some(to) = target else {
                g.hen.state = next_idle(g);
                return;
            };
            if t <= 0.0 {
                g.hen.state = next_idle(g);
                return;
            }
            let reach = if food == Food::Bag { 0.7 } else { 0.4 };
            if walk(g, to, 2.6) < reach {
                eat(g, food);
            } else {
                g.hen.state = State::GoFood { food, t };
            }
        }
        State::Eat { t } => {
            stop(g);
            let t = t - DT;
            g.hen.state = if t <= 0.0 { next_idle(g) } else { State::Eat { t } };
        }
        State::Lay { t } => {
            stop(g);
            let t = t - DT;
            if t <= 0.0 {
                let p = g.hen.pos - g.hen.facing.vec() * 0.2;
                items::drop_item(g, ItemKind::Egg, p, Some(Who::Hen));
                g.events.push(SimEvent::EggLaid { pos: p });
                g.say(Who::Hen, "BAGAWK!");
                g.hen.state = next_idle(g);
            } else {
                g.hen.state = State::Lay { t };
            }
        }
        State::Poop { t } => {
            stop(g);
            let t = t - DT;
            if t <= 0.0 {
                let p = g.hen.pos - g.hen.facing.vec() * 0.25;
                items::drop_item(g, ItemKind::Poop, p, Some(Who::Hen));
                g.events.push(SimEvent::Poop { pos: p });
                g.hen.state = next_idle(g);
            } else {
                g.hen.state = State::Poop { t };
            }
        }
        State::Plonk { to, t } => {
            let t = t - DT;
            if walk(g, to, 2.0) < 0.3 || t <= 0.0 {
                g.hen.state = State::Sit { t: g.rng.random_range(3.0..6.0) };
            } else {
                g.hen.state = State::Plonk { to, t };
            }
        }
        State::Flee { from, t } => {
            let t = t - DT;
            let away = g.hen.pos + (g.hen.pos - from).normalize_or_zero() * 3.0;
            walk(g, away, 3.8);
            g.hen.state = if t <= 0.0 { next_idle(g) } else { State::Flee { from, t } };
        }
    }
}

fn eat(g: &mut Game, food: Food) {
    match food {
        Food::Item(id) => {
            if let Some(i) = items::find(g, id) {
                let it = g.items.remove(i);
                if it.from == Some(Who::Raccoon) {
                    g.discover(Id::HenEatsDropped);
                }
            }
        }
        Food::Berries(f) => {
            g.feats[f].used = true;
            g.feats[f].regrow = 150.0;
            g.events.push(SimEvent::Berries { pos: g.town.features[f].pos });
        }
        Food::Bag => {
            // A peck at the bag: something falls out, and she has it.
            if g.raccoon.act == Act::AtDoor || g.raccoon.bag.is_empty() {
                g.hen.state = next_idle(g);
                return;
            }
            let c = g.raccoon.bag.kinds().map(|(c, _)| c).find(|c| *c != Candy::Toothbrush && *c != Candy::Pennies);
            if let Some(c) = c {
                g.raccoon.bag.take(c);
                g.discover(Id::HenStealsCandy);
                g.say(Who::Raccoon, "Hey!");
                g.say_message(format!("Gullinkombi pecks {} out of the bag and eats it.", c.name()));
            }
        }
    }
    g.hen.hunger = (g.hen.hunger - 0.3).max(0.0);
    g.hen.fullness = (g.hen.fullness + 0.28).min(1.0);
    g.hen.state = State::Eat { t: 0.9 };
    if g.hen.cluck_cd <= 0.0 {
        let l = lines::pick(&mut g.rng, lines::HEN);
        g.say(Who::Hen, l);
        g.hen.cluck_cd = 6.0;
    }
}

/// What a hen does when she is not doing anything: peck, wander after the
/// children, or plonk herself down in the raccoon's way.
fn next_idle(g: &mut Game) -> State {
    let near = (g.raccoon.pos + g.kids[0].pos + g.kids[1].pos) / 3.0;
    let roll: f32 = g.rng.random();
    if g.hen.pos.distance(near) > 10.0 {
        return State::Wander { to: near, t: 12.0 };
    }
    if roll < 0.12 && g.raccoon.vel.length() > 1.0 {
        // Right where the raccoon is going.
        let ahead = g.raccoon.pos + g.raccoon.vel.normalize() * 2.5;
        if g.town.clear(ahead, RADIUS, false) {
            return State::Plonk { to: ahead, t: 3.0 };
        }
    }
    if roll < 0.45 {
        return State::Peck { t: g.rng.random_range(1.0..3.0) };
    }
    if roll < 0.55 {
        return State::Sit { t: g.rng.random_range(3.0..7.0) };
    }
    let a: f32 = g.rng.random_range(0.0..std::f32::consts::TAU);
    let to = near + Vec2::new(a.cos(), a.sin()) * g.rng.random_range(1.5..6.0);
    State::Wander { to, t: 6.0 }
}

/// The raccoon and the hen meet: she is in the way. Running into her
/// trips the raccoon.
fn raccoon_contact(g: &mut Game) {
    if g.hen.held_by.is_some() {
        return;
    }
    let d = g.hen.pos.distance(g.raccoon.pos);
    if d >= 0.5 {
        return;
    }
    if g.raccoon.running && g.raccoon.vel.length() > 4.0 && g.raccoon.shaken <= 0.0 && !g.raccoon.busy() {
        let push = g.raccoon.vel * 0.3;
        raccoon::knock_down(g, Who::Hen, 0.0, 1, push);
        g.hen.state = State::Flee { from: g.raccoon.pos, t: 1.5 };
        g.events.push(SimEvent::Squawk { pos: g.hen.pos });
        g.say(Who::Hen, "BWAAAAK!");
        g.say_message("Tripped over Gullinkombi.");
        return;
    }
    // In the way: the raccoon is held back, and she complains.
    let dv = g.raccoon.pos - g.hen.pos;
    let n = if dv.length() > 0.0 { dv.normalize() } else { Vec2::X };
    let town = g.town.clone();
    move_body(&town, &mut g.raccoon.pos, n * (0.5 - d), super::raccoon::RADIUS, true);
    if g.hen.cluck_cd <= 0.0 {
        g.say(Who::Hen, "Bok.");
        g.hen.cluck_cd = 5.0;
        if matches!(g.hen.state, State::Sit { .. }) {
            // She does not move for anybody.
            g.hen.state = State::Sit { t: 2.0 };
        }
    }
}
