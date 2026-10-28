//! Gravy: a very big, very slow brown dog whose coat hangs in cords, who
//! lives around the corner of State and Main. He ambles, lies down exactly
//! where the raccoon wants to go, and will not get up until somebody rubs
//! his belly. He eats any candy he finds on the ground, and it does not
//! agree with him.

use super::discover::Id;
use super::items::{self, ItemKind};
use super::{DT, Game, SimEvent, Who, facing_of, move_body, steer};
use crate::world::Dir;
use crate::world::plan::Town;
use glam::Vec2;
use rand::RngExt;

pub const RADIUS: f32 = 0.55;
/// Where he lives: the corner of State and Main.
pub const HOME: Vec2 = Vec2::new(129.0, 127.4);
const ROAM: f32 = 11.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum State {
    Amble { to: Vec2, t: f32 },
    /// Lying down (in somebody's way, ideally). He stays until rubbed.
    Lie,
    /// On his back, legs going, being rubbed.
    Rubbed { t: f32 },
    /// After a rub he is not lying down again for a while.
    Content { t: f32 },
    GoCandy { item: u32 },
    Eat { t: f32 },
    /// Something is coming back up.
    Queasy { t: f32 },
    Barf { t: f32 },
}

#[derive(Clone, Debug)]
pub struct Gravy {
    pub pos: Vec2,
    pub prev: Vec2,
    pub vel: Vec2,
    pub facing: Dir,
    pub state: State,
    /// Seconds before he next looks for somewhere inconvenient to lie.
    pub lie_cd: f32,
    pub woof_cd: f32,
    pub stride: f32,
}

impl Gravy {
    pub fn new(town: &Town) -> Gravy {
        let at = open_near(town, HOME);
        Gravy {
            pos: at,
            prev: at,
            vel: Vec2::ZERO,
            facing: Dir::W,
            state: State::Amble { to: at, t: 1.0 },
            lie_cd: 8.0,
            woof_cd: 5.0,
            stride: 0.0,
        }
    }

    pub fn lying(&self) -> bool {
        matches!(self.state, State::Lie | State::Rubbed { .. })
    }
}

pub fn step(g: &mut Game) {
    g.gravy.prev = g.gravy.pos;
    g.gravy.lie_cd = (g.gravy.lie_cd - DT).max(0.0);
    g.gravy.woof_cd = (g.gravy.woof_cd - DT).max(0.0);
    let state = g.gravy.state;
    match state {
        State::Amble { to, t } => {
            // Candy on the ground beats everything.
            if g.tick % 15 == 0 {
                if let Some(item) = items::nearest(g, g.gravy.pos, 6.0, |it| matches!(it.kind, ItemKind::Candy(_))) {
                    g.gravy.state = State::GoCandy { item };
                    return;
                }
            }
            // The raccoon is coming: lie down in front of it.
            let r = &g.raccoon;
            if g.gravy.lie_cd <= 0.0 && r.vel.length() > 1.0 && r.pos.distance(g.gravy.pos) < 9.0 {
                let ahead = r.pos + r.vel.normalize() * 3.5;
                if ahead.distance(HOME) < ROAM + 3.0 && g.town.clear(ahead, RADIUS, false) {
                    g.gravy.state = State::Amble { to: ahead, t: 6.0 };
                    g.gravy.lie_cd = 1e9;
                }
            }
            let t = t - DT;
            if walk(g, to, 0.9) < 0.3 || t <= 0.0 {
                if g.gravy.lie_cd > 1e8 || g.rng.random_bool(0.4) {
                    g.gravy.state = State::Lie;
                    g.gravy.lie_cd = 0.0;
                } else {
                    let a: f32 = g.rng.random_range(0.0..std::f32::consts::TAU);
                    let to = HOME + Vec2::new(a.cos(), a.sin()) * g.rng.random_range(0.0..ROAM);
                    let to = if g.town.clear(to, RADIUS, false) { to } else { HOME };
                    g.gravy.state = State::Amble { to, t: 20.0 };
                }
            } else if let State::Amble { .. } = g.gravy.state {
                g.gravy.state = State::Amble { to, t };
            }
        }
        State::Lie => {
            g.gravy.vel = Vec2::ZERO;
            // Even lying down, he can smell candy nearby.
            if g.tick % 30 == 0 {
                if let Some(item) = items::nearest(g, g.gravy.pos, 2.5, |it| matches!(it.kind, ItemKind::Candy(_))) {
                    g.gravy.state = State::GoCandy { item };
                }
            }
        }
        State::Rubbed { t } => {
            let t = t - DT;
            g.gravy.state = if t <= 0.0 {
                g.say(Who::Gravy, "*contented groan*");
                State::Content { t: 25.0 }
            } else {
                State::Rubbed { t }
            };
        }
        State::Content { t } => {
            let t = t - DT;
            let away = g.gravy.pos + (g.gravy.pos - g.raccoon.pos).normalize_or_zero() * 3.0;
            let to = if away.distance(HOME) < ROAM { away } else { HOME };
            walk(g, to, 0.8);
            g.gravy.state = if t <= 0.0 {
                g.gravy.lie_cd = g.rng.random_range(6.0..14.0);
                State::Amble { to: g.gravy.pos, t: 1.0 }
            } else {
                State::Content { t }
            };
        }
        State::GoCandy { item } => match items::find(g, item) {
            Some(i) if g.items[i].carried.is_none() => {
                let at = g.items[i].pos;
                if walk(g, at, 1.4) < 0.5 {
                    g.items.remove(i);
                    g.gravy.state = State::Eat { t: 1.0 };
                    if g.gravy.woof_cd <= 0.0 {
                        g.say(Who::Gravy, "*CHOMP*");
                        g.gravy.woof_cd = 4.0;
                    }
                }
            }
            _ => g.gravy.state = State::Amble { to: g.gravy.pos, t: 1.0 },
        },
        State::Eat { t } => {
            g.gravy.vel = Vec2::ZERO;
            let t = t - DT;
            g.gravy.state = if t <= 0.0 { State::Queasy { t: g.rng.random_range(3.0..7.0) } } else { State::Eat { t } };
        }
        State::Queasy { t } => {
            let a = g.gravy.pos + Vec2::new(0.5, 0.0);
            walk(g, a, 0.4);
            let t = t - DT;
            g.gravy.state = if t <= 0.0 {
                g.say(Who::Gravy, "*HURK*");
                State::Barf { t: 1.2 }
            } else {
                State::Queasy { t }
            };
        }
        State::Barf { t } => {
            g.gravy.vel = Vec2::ZERO;
            let t = t - DT;
            if t <= 0.0 {
                let p = g.gravy.pos + g.gravy.facing.vec() * 0.9;
                items::drop_item(g, ItemKind::Barf, p, Some(Who::Gravy));
                g.events.push(SimEvent::Barf { pos: p });
                if g.raccoon.pos.distance(p) < 14.0 {
                    g.discover(Id::GravyBarf);
                }
                g.gravy.state = State::Amble { to: g.gravy.pos, t: 2.0 };
            } else {
                g.gravy.state = State::Barf { t };
            }
        }
    }
    blocking(g);
}

/// The nearest spot to `p` with room for a dog his size.
fn open_near(town: &Town, p: Vec2) -> Vec2 {
    for r in 0..40 {
        for k in 0..16 {
            let a = k as f32 / 16.0 * std::f32::consts::TAU;
            let q = p + Vec2::new(a.cos(), a.sin()) * r as f32 * 0.5;
            if town.clear(q, RADIUS, false) {
                return q;
            }
        }
    }
    town.start
}

fn walk(g: &mut Game, to: Vec2, speed: f32) -> f32 {
    let town = g.town.clone();
    let d = &mut g.gravy;
    let left = d.pos.distance(to);
    let dir = to - d.pos;
    let want = if dir.length() > 0.05 { dir.normalize() * speed.min(dir.length() / DT) } else { Vec2::ZERO };
    d.vel = steer(d.vel, want, 4.0);
    d.facing = facing_of(d.vel, d.facing);
    if matches!(d.facing, Dir::N | Dir::S) {
        // He only ever looks left or right (the art is side on).
        d.facing = if d.vel.x < 0.0 { Dir::W } else { Dir::E };
    }
    let before = d.pos;
    let dv = d.vel * DT;
    move_body(&town, &mut d.pos, dv, RADIUS, false);
    d.stride += d.pos.distance(before);
    left
}

/// He is big: nobody walks through him, and lying down he is a wall.
fn blocking(g: &mut Game) {
    let d = g.gravy.pos.distance(g.raccoon.pos);
    let reach = RADIUS + super::raccoon::RADIUS;
    if d < reach && d > 0.0 {
        let n = (g.raccoon.pos - g.gravy.pos) / d;
        let town = g.town.clone();
        move_body(&town, &mut g.raccoon.pos, n * (reach - d), super::raccoon::RADIUS, true);
        if g.gravy.state == State::Lie && g.gravy.woof_cd <= 0.0 {
            g.say(Who::Gravy, "*does not move*");
            g.say_message("Gravy is lying across the way and will not budge. Rub his belly (press Space).");
            g.discover(Id::GravyBlocks);
            g.gravy.woof_cd = 8.0;
        }
    }
}

/// The raccoon rubs his belly: he rolls over, kicks a leg, and then (in
/// his own time) gets up.
pub fn rub(g: &mut Game) -> bool {
    if g.gravy.state != State::Lie || g.gravy.pos.distance(g.raccoon.pos) > RADIUS + 1.0 {
        return false;
    }
    g.gravy.state = State::Rubbed { t: 2.6 };
    g.raccoon.happy = (g.raccoon.happy + 0.12).min(1.0);
    g.say(Who::Gravy, "*thump thump thump*");
    g.events.push(SimEvent::Hug { pos: g.gravy.pos });
    g.discover(Id::BellyRub);
    g.say_message("You rub Gravy's belly. His leg goes like a drum. You feel better too.");
    true
}
