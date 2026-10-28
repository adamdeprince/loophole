//! A black cat who lives on State Street near Main. She wanders, sits,
//! washes, crosses the raccoon's path (bad luck at the next door), hisses
//! at a shabby costume (cats know), ignores Skaði's cat ears entirely, and
//! goes up a tree when Loki chases her.

use super::discover::Id;
use super::{DT, Game, SimEvent, Who, facing_of, move_body, steer};
use crate::world::Dir;
use crate::world::plan::Town;
use crate::world::town::FeatureKind;
use glam::Vec2;
use rand::RngExt;

pub const RADIUS: f32 = 0.18;
pub const HOME: Vec2 = Vec2::new(117.0, 133.4);
const ROAM: f32 = 14.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum State {
    Prowl { to: Vec2, t: f32 },
    Sit { t: f32 },
    /// Strolling across in front of the raccoon, slowly, on purpose.
    Cross { to: Vec2 },
    Hiss { t: f32 },
    Flee { to: Vec2 },
    /// Up a tree (out of sight) until things calm down.
    Treed { tree: Vec2, t: f32 },
}

#[derive(Clone, Debug)]
pub struct Cat {
    pub pos: Vec2,
    pub prev: Vec2,
    pub vel: Vec2,
    pub facing: Dir,
    pub state: State,
    pub cross_cd: f32,
    pub hiss_cd: f32,
    pub stride: f32,
}

impl Cat {
    pub fn new(town: &Town) -> Cat {
        let at = if town.clear(HOME, RADIUS, false) { HOME } else { town.start + Vec2::new(-6.0, 1.5) };
        Cat {
            pos: at,
            prev: at,
            vel: Vec2::ZERO,
            facing: Dir::E,
            state: State::Sit { t: 3.0 },
            cross_cd: 20.0,
            hiss_cd: 0.0,
            stride: 0.0,
        }
    }

    pub fn hidden(&self) -> bool {
        matches!(self.state, State::Treed { .. })
    }
}

fn walk(g: &mut Game, to: Vec2, speed: f32) -> f32 {
    let town = g.town.clone();
    let c = &mut g.cat;
    let left = c.pos.distance(to);
    let d = to - c.pos;
    let want = if d.length() > 0.05 { d.normalize() * speed.min(d.length() / DT) } else { Vec2::ZERO };
    c.vel = steer(c.vel, want, 20.0);
    c.facing = facing_of(c.vel, c.facing);
    let before = c.pos;
    let dv = c.vel * DT;
    if move_body(&town, &mut c.pos, dv, RADIUS, false) && c.pos.distance(before) < 0.001 {
        // Stuck on something: give up on that and sit.
        return 0.0;
    }
    c.stride += c.pos.distance(before);
    left
}

pub fn step(g: &mut Game) {
    g.cat.prev = g.cat.pos;
    g.cat.cross_cd = (g.cat.cross_cd - DT).max(0.0);
    g.cat.hiss_cd = (g.cat.hiss_cd - DT).max(0.0);
    // Loki coming at a run sends her up the nearest tree.
    let loki = &g.kids[0];
    if !g.cat.hidden() && loki.vel.length() > 3.0 && loki.pos.distance(g.cat.pos) < 2.5 {
        let tree = g
            .town
            .features_near(g.cat.pos, 10.0)
            .filter(|f| matches!(f.kind, FeatureKind::Tree(_)))
            .min_by(|a, b| a.pos.distance(g.cat.pos).total_cmp(&b.pos.distance(g.cat.pos)))
            .map(|f| f.pos);
        if let Some(tree) = tree {
            g.cat.state = State::Flee { to: tree };
            g.say(Who::Cat, "MRRROW!");
            g.say(Who::Loki, "KITTY! Come back!");
            g.discover(Id::CatTreed);
        }
    }
    let state = g.cat.state;
    match state {
        State::Prowl { to, t } => {
            let t = t - DT;
            if walk(g, to, 1.6) < 0.3 || t <= 0.0 {
                g.cat.state = State::Sit { t: g.rng.random_range(2.0..8.0) };
            } else {
                g.cat.state = State::Prowl { to, t };
            }
            look_around(g);
        }
        State::Sit { t } => {
            g.cat.vel = Vec2::ZERO;
            let t = t - DT;
            if t <= 0.0 {
                let a: f32 = g.rng.random_range(0.0..std::f32::consts::TAU);
                let to = HOME + Vec2::new(a.cos(), a.sin() * 0.5) * g.rng.random_range(0.0..ROAM);
                let to = if g.town.clear(to, RADIUS, false) { to } else { HOME };
                g.cat.state = State::Prowl { to, t: 15.0 };
            } else {
                g.cat.state = State::Sit { t };
            }
            look_around(g);
        }
        State::Cross { to } => {
            if walk(g, to, 1.2) < 0.3 {
                g.cat.state = State::Sit { t: 4.0 };
            }
            // She has to actually cross in front of the raccoon.
            let r = &g.raccoon;
            if r.vel.length() > 0.5 {
                let ahead = r.pos + r.vel.normalize() * 1.5;
                if ahead.distance(g.cat.pos) < 0.8 && r.bad_luck == 0 {
                    g.raccoon.bad_luck = 1;
                    g.discover(Id::BlackCat);
                    g.say_message("A black cat crosses your path. Bad luck at the next door.");
                    g.say(Who::Cat, "*slow blink*");
                }
            }
        }
        State::Hiss { t } => {
            g.cat.vel = Vec2::ZERO;
            let t = t - DT;
            g.cat.state = if t <= 0.0 { State::Flee { to: g.cat.pos + (g.cat.pos - g.raccoon.pos).normalize_or_zero() * 4.0 } } else { State::Hiss { t } };
        }
        State::Flee { to } => {
            if walk(g, to, 4.5) < 0.4 {
                let is_tree = g.town.features_near(to, 0.2).any(|f| matches!(f.kind, FeatureKind::Tree(_)));
                g.cat.state = if is_tree { State::Treed { tree: to, t: g.rng.random_range(12.0..25.0) } } else { State::Sit { t: 5.0 } };
            }
        }
        State::Treed { tree, t } => {
            g.cat.vel = Vec2::ZERO;
            g.cat.pos = tree;
            let t = t - DT;
            if t <= 0.0 {
                let down = tree + Vec2::new(0.5, 0.4);
                let town = g.town.clone();
                g.cat.pos = if town.clear(down, RADIUS, false) { down } else { tree + Vec2::new(-0.5, 0.4) };
                g.cat.state = State::Sit { t: 3.0 };
            } else {
                g.cat.state = State::Treed { tree, t };
            }
        }
    }
}

/// What she does about the raccoon and the children when she notices them.
fn look_around(g: &mut Game) {
    let d = g.cat.pos.distance(g.raccoon.pos);
    // A shabby sheet does not fool a cat.
    if d < 2.2 && g.cat.hiss_cd <= 0.0 && g.raccoon.passing() < 0.6 {
        g.cat.state = State::Hiss { t: 1.2 };
        g.cat.hiss_cd = 20.0;
        g.cat.facing = Dir::of(g.raccoon.pos - g.cat.pos);
        g.say(Who::Cat, "HSSSSS!");
        g.events.push(SimEvent::Scream { pos: g.cat.pos });
        g.discover(Id::CatHiss);
        return;
    }
    // Cross in front of a raccoon that is walking along.
    let r = &g.raccoon;
    if g.cat.cross_cd <= 0.0 && d < 7.0 && r.vel.length() > 1.0 {
        let v = r.vel.normalize();
        let ahead = r.pos + v * 3.0;
        let side = Vec2::new(-v.y, v.x);
        let from_left = (g.cat.pos - ahead).dot(side) > 0.0;
        let to = ahead - side * if from_left { 2.5 } else { -2.5 };
        if g.town.clear(to, RADIUS, false) {
            g.cat.state = State::Cross { to };
            g.cat.cross_cd = g.rng.random_range(40.0..80.0);
        }
    }
    // Skaði says hello. The cat does not.
    let s = &g.kids[1];
    if s.pos.distance(g.cat.pos) < 2.0 && g.cat.hiss_cd <= 0.0 && g.rng.random_bool(0.02) {
        g.say(Who::Skadi, "Hi kitty! Look, I'm a cat too. Meow?");
        g.say(Who::Cat, "*ignores her*");
        g.discover(Id::SkadiMeetsCat);
        g.cat.hiss_cd = 15.0;
    }
}
