//! Loki and Skaði, who think the raccoon is another kid and tag along.
//!
//! Loki (six, in a dinosaur hoodie) is random: he dashes about, crashes
//! into people, dives into leaf piles, moves things, pulls tails, and
//! takes the hen's eggs home. Skaði (fourteen, cat ears) means what she
//! does: she comforts her brother, hands back what the raccoon dropped,
//! patches its sheet, gets the hen off the road, and has opinions about
//! taking more than one.

use super::candy::{Candy, Mat};
use super::costume::Wear;
use super::discover::Id;
use super::items::{self, ItemKind};
use super::raccoon::{self, Act};
use super::{DT, Game, SimEvent, Who, facing_of, lines, move_body, steer};
use crate::world::Dir;
use crate::world::grid::Terrain;
use crate::world::nav;
use crate::world::plan::Town;
use crate::world::town::FeatureKind;
use glam::Vec2;
use rand::RngExt;
use rand_chacha::ChaCha8Rng;

pub const LOKI: usize = 0;
pub const SKADI: usize = 1;
pub const RADIUS: f32 = 0.28;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Purpose {
    /// Skaði: back to the raccoon, who dropped it.
    GiveBack,
    /// Skaði: to mend the raccoon's sheet with.
    Patch,
    /// Loki: a present for his new friend.
    Gift,
    /// Loki: somewhere else, for no reason.
    Elsewhere(Vec2),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum State {
    Follow,
    Wander { to: Vec2, t: f32 },
    /// Running flat out (Loki, mostly): this is how bumps happen.
    Dash { to: Vec2, t: f32 },
    FetchEgg { item: u32 },
    CarryEgg,
    Crying { t: f32 },
    /// Skaði on her way to her crying brother.
    GoComfort,
    Comfort { t: f32 },
    ChaseHen { t: f32 },
    HoldHen { t: f32 },
    /// Both children have the hen.
    Tug { t: f32 },
    Sulk { t: f32 },
    Fetch { item: u32, purpose: Purpose },
    Bring { what: ItemKind, purpose: Purpose },
    /// Jumping up and down (hákarl!).
    Jump { t: f32 },
    LeafDive { feature: usize, t: f32 },
    PullTail { t: f32 },
    /// Skaði carrying the hen somewhere safer.
    RescueHen { to: Vec2 },
}

#[derive(Clone, Debug)]
pub struct Kid {
    pub who: Who,
    pub pos: Vec2,
    pub prev: Vec2,
    pub vel: Vec2,
    pub facing: Dir,
    pub state: State,
    /// Seconds until the next decision.
    pub think: f32,
    pub path: Vec<Vec2>,
    pub path_to: Option<Vec2>,
    pub path_age: f32,
    /// Candy in the pail (Loki) or pillowcase (Skaði).
    pub pail: u64,
    pub bump_cd: f32,
    pub mom_cd: f32,
    pub chat_cd: f32,
    pub stride: f32,
    /// Loki's folded-newspaper tricorn, from Ethan Allen.
    pub tricorn: bool,
    /// Skaði's key to Fort Ticonderoga (a key, anyway), from Ethan Allen.
    pub key: bool,
}

impl Kid {
    fn new(who: Who, at: Vec2) -> Kid {
        Kid {
            who,
            pos: at,
            prev: at,
            vel: Vec2::ZERO,
            facing: Dir::S,
            state: State::Follow,
            think: 1.0,
            path: Vec::new(),
            path_to: None,
            path_age: 0.0,
            pail: 0,
            bump_cd: 3.0,
            mom_cd: 0.0,
            chat_cd: 6.0,
            stride: 0.0,
            tricorn: false,
            key: false,
        }
    }

    pub fn holding_hen(&self) -> bool {
        matches!(self.state, State::HoldHen { .. } | State::Tug { .. } | State::RescueHen { .. })
    }

    pub fn carrying_egg(&self) -> bool {
        matches!(self.state, State::CarryEgg)
    }

    fn walk_speed(&self) -> f32 {
        if self.who == Who::Loki { 3.2 } else { 3.4 }
    }

    fn run_speed(&self) -> f32 {
        if self.who == Who::Loki { 5.6 } else { 6.2 }
    }
}

pub fn start(town: &Town, _rng: &mut ChaCha8Rng) -> [Kid; 2] {
    let s = town.start;
    [Kid::new(Who::Loki, s + Vec2::new(-1.6, 1.2)), Kid::new(Who::Skadi, s + Vec2::new(1.4, 1.3))]
}

/// A child is handed something at a door (or by their mother).
pub fn receive(g: &mut Game, i: usize, c: Candy) {
    g.kids[i].pail = g.kids[i].pail.saturating_add(1);
    if c == Candy::Hakarl {
        g.kids[i].state = State::Jump { t: 2.2 };
        let who = g.kids[i].who;
        let pos = g.kids[i].pos;
        let line = lines::pick(&mut g.rng, lines::JOY);
        g.say(who, line);
        g.events.push(SimEvent::Jump { who, pos });
        g.discover(Id::KidsLoveHakarl);
    }
}

/// Skaði sees the raccoon empty a "please take one" bowl.
pub fn skadi_objects_to_greed(g: &mut Game, feature: usize, took: u32) {
    let k = &g.kids[SKADI];
    if k.pos.distance(g.raccoon.pos) > 9.0 || took < 2 {
        return;
    }
    let back = (took / 2).max(1) as u64;
    let mut put = 0;
    for _ in 0..back {
        let Some(c) = g.raccoon.bag.kinds().map(|(c, _)| c).filter(|c| c.is_treat()).last() else { break };
        g.raccoon.bag.take(c);
        put += 1;
    }
    if put > 0 {
        g.feats[feature].n += put as u32;
        g.feats[feature].used = false;
        let line = lines::pick(&mut g.rng, lines::SKADI_FAIR);
        g.say(Who::Skadi, line);
        g.discover(Id::SkadiPutsBack);
        g.say_message(format!("Skaði puts {put} back in the bowl."));
    }
}

pub fn step(g: &mut Game) {
    for i in 0..2 {
        g.kids[i].prev = g.kids[i].pos;
        let k = &mut g.kids[i];
        k.bump_cd = (k.bump_cd - DT).max(0.0);
        k.mom_cd = (k.mom_cd - DT).max(0.0);
        k.chat_cd = (k.chat_cd - DT).max(0.0);
        k.path_age += DT;
        k.think -= DT;
        if k.think <= 0.0 {
            k.think = if i == LOKI { g.rng.random_range(0.6..1.6) } else { 0.5 };
            decide(g, i);
        }
        act(g, i);
        chatter(g, i);
    }
    contacts(g);
    mom(g);
}

fn decide(g: &mut Game, i: usize) {
    let me = g.kids[i].clone();
    let interruptible = matches!(me.state, State::Follow | State::Wander { .. });
    if !interruptible {
        return;
    }
    let rpos = g.raccoon.pos;
    if i == SKADI {
        // Her brother first.
        if matches!(g.kids[LOKI].state, State::Crying { .. }) {
            g.kids[i].state = State::GoComfort;
            return;
        }
        // Whatever the raccoon dropped.
        if let Some(item) = items::nearest(g, me.pos, 9.0, |it| it.from == Some(Who::Raccoon) && matches!(it.kind, ItemKind::Candy(_))) {
            g.kids[i].state = State::Fetch {
                item,
                purpose: Purpose::GiveBack,
            };
            return;
        }
        // A torn sheet and a scrap to mend it with.
        if g.raccoon.costume.damaged() > 0 && g.rng.random_bool(0.25) {
            if let Some(item) = items::nearest(g, me.pos, 10.0, |it| matches!(it.kind, ItemKind::Mat(_))) {
                g.kids[i].state = State::Fetch {
                    item,
                    purpose: Purpose::Patch,
                };
                return;
            }
        }
        // The hen in the road.
        if g.hen.held_by.is_none() && g.town.grid.at(g.hen.pos) == Terrain::Road && g.hen.pos.distance(me.pos) < 12.0 {
            g.kids[i].state = State::ChaseHen { t: 12.0 };
            return;
        }
        if g.rng.random_bool(0.03) && g.hen.held_by.is_none() && g.hen.pos.distance(me.pos) < 14.0 {
            g.kids[i].state = State::ChaseHen { t: 10.0 };
            return;
        }
        g.kids[i].state = State::Follow;
        return;
    }
    // Loki.
    if let Some(item) = items::nearest(g, me.pos, 16.0, |it| it.kind == ItemKind::Egg) {
        g.kids[i].state = State::FetchEgg { item };
        return;
    }
    let roll: f32 = g.rng.random();
    if roll < 0.2 {
        // Dash past (or through) the raccoon.
        let a: f32 = g.rng.random_range(0.0..std::f32::consts::TAU);
        let to = rpos + Vec2::new(a.cos(), a.sin()) * g.rng.random_range(2.0..5.0);
        let through = g.rng.random_bool(0.35);
        let to = if through { rpos + (rpos - me.pos).normalize_or_zero() * 3.0 } else { to };
        g.kids[i].state = State::Dash { to, t: 3.0 };
    } else if roll < 0.32 {
        let pile = g
            .town
            .features_near(me.pos, 9.0)
            .filter(|f| f.kind == FeatureKind::LeafPile)
            .map(|f| f.id)
            .find(|&id| !g.feats[id].used);
        if let Some(feature) = pile {
            g.kids[i].state = State::LeafDive { feature, t: 0.0 };
        }
    } else if roll < 0.38 && g.hen.held_by.is_none() && g.hen.pos.distance(me.pos) < 14.0 {
        g.kids[i].state = State::ChaseHen { t: 10.0 };
    } else if roll < 0.46 {
        if let Some(item) = items::nearest(g, me.pos, 8.0, |it| matches!(it.kind, ItemKind::Mat(_) | ItemKind::Candy(_)) && it.from != Some(Who::Raccoon)) {
            let purpose = if g.rng.random_bool(0.45) {
                Purpose::Gift
            } else {
                let a: f32 = g.rng.random_range(0.0..std::f32::consts::TAU);
                Purpose::Elsewhere(me.pos + Vec2::new(a.cos(), a.sin()) * g.rng.random_range(4.0..9.0))
            };
            g.kids[i].state = State::Fetch { item, purpose };
        }
    } else if roll < 0.49 && me.pos.distance(rpos) < 3.0 {
        g.kids[i].state = State::PullTail { t: 0.0 };
    } else if roll < 0.62 {
        let a: f32 = g.rng.random_range(0.0..std::f32::consts::TAU);
        let to = rpos + Vec2::new(a.cos(), a.sin()) * g.rng.random_range(2.5..6.0);
        g.kids[i].state = State::Wander { to, t: 5.0 };
    } else {
        g.kids[i].state = State::Follow;
    }
}

/// Walks (or runs) child `i` towards `to`, by path when the way is not
/// straight. Returns the distance left.
fn go(g: &mut Game, i: usize, to: Vec2, run: bool) -> f32 {
    let town = g.town.clone();
    let k = &mut g.kids[i];
    let left = k.pos.distance(to);
    let mut aim = to;
    if !nav::line_clear(&town, k.pos, to, false) {
        let stale = k.path_to.is_none_or(|p| p.distance(to) > 2.0) || k.path_age > 2.0 || k.path.is_empty();
        if stale {
            k.path = nav::path(&town, k.pos, to, false).unwrap_or_default();
            k.path_to = Some(to);
            k.path_age = 0.0;
            if !k.path.is_empty() {
                k.path.remove(0);
            }
        }
        while let Some(&p) = k.path.first() {
            if p.distance(k.pos) < 0.4 && k.path.len() > 1 {
                k.path.remove(0);
            } else {
                break;
            }
        }
        if let Some(&p) = k.path.first() {
            aim = p;
        }
    } else {
        k.path.clear();
    }
    let speed = if run { k.run_speed() } else { k.walk_speed() } * town.grid.at(k.pos).pace().max(0.6);
    let d = aim - k.pos;
    let want = if d.length() > 0.08 { d.normalize() * speed.min(d.length() / DT) } else { Vec2::ZERO };
    k.vel = steer(k.vel, want, 30.0);
    k.facing = facing_of(k.vel, k.facing);
    let before = k.pos;
    let dv = k.vel * DT;
    move_body(&town, &mut k.pos, dv, RADIUS, false);
    k.stride += k.pos.distance(before);
    left
}

fn stand(g: &mut Game, i: usize) {
    let k = &mut g.kids[i];
    k.vel = steer(k.vel, Vec2::ZERO, 30.0);
    let town = g.town.clone();
    let dv = k.vel * DT;
    move_body(&town, &mut k.pos, dv, RADIUS, false);
}

fn act(g: &mut Game, i: usize) {
    let state = g.kids[i].state;
    let rpos = g.raccoon.pos;
    let me = g.kids[i].pos;
    match state {
        State::Follow => {
            // Loki a little behind on the right, Skaði beside on the left.
            let back = -g.raccoon.facing.vec();
            let side = Vec2::new(-back.y, back.x);
            let off = if i == LOKI { back * 1.9 + side * 0.9 } else { back * 1.2 - side * 1.1 };
            let spot = rpos + off;
            let far = me.distance(rpos) > 7.0;
            if me.distance(spot) > 0.9 {
                go(g, i, spot, far);
            } else {
                stand(g, i);
                g.kids[i].facing = g.raccoon.facing;
            }
        }
        State::Wander { to, t } => {
            let t = t - DT;
            if go(g, i, to, false) < 0.5 || t <= 0.0 {
                g.kids[i].state = State::Follow;
            } else {
                g.kids[i].state = State::Wander { to, t };
            }
        }
        State::Dash { to, t } => {
            let t = t - DT;
            if go(g, i, to, true) < 0.5 || t <= 0.0 {
                g.kids[i].state = State::Follow;
            } else {
                g.kids[i].state = State::Dash { to, t };
            }
        }
        State::FetchEgg { item } => match items::find(g, item) {
            Some(idx) if g.items[idx].carried.is_none() => {
                let at = g.items[idx].pos;
                if go(g, i, at, true) < 0.5 {
                    g.items[idx].carried = Some(Who::Loki);
                    g.kids[i].state = State::CarryEgg;
                    g.kids[i].path.clear();
                    g.say(Who::Loki, "I found an egg! I'm taking it home!");
                }
            }
            _ => g.kids[i].state = State::Follow,
        },
        State::CarryEgg => {
            let home = g.town.buildings[g.town.home].door.step();
            // Carry the egg along.
            let pos = g.kids[i].pos;
            if let Some(it) = g.items.iter_mut().find(|it| it.carried == Some(Who::Loki) && it.kind == ItemKind::Egg) {
                it.pos = pos;
            } else {
                g.kids[i].state = State::Follow;
                return;
            }
            if go(g, i, home, true) < 0.9 {
                g.items.retain(|it| !(it.carried == Some(Who::Loki) && it.kind == ItemKind::Egg));
                if let Some(b) = g.town.features.iter().find(|f| f.kind == FeatureKind::EggBasket) {
                    g.feats[b.id].n = g.feats[b.id].n.saturating_add(1);
                }
                g.events.push(SimEvent::EggHome { pos: home });
                g.discover(Id::EggDelivered);
                g.say(Who::Door(g.town.home as u32), "Another egg! Good job, Loki! Have some hákarl.");
                receive(g, LOKI, Candy::Hakarl);
                g.kids[i].mom_cd = 60.0;
            }
        }
        State::Crying { t } => {
            stand(g, i);
            let t = t - DT;
            g.kids[i].state = if t <= 0.0 { State::Follow } else { State::Crying { t } };
        }
        State::GoComfort => {
            let loki = g.kids[LOKI].pos;
            if !matches!(g.kids[LOKI].state, State::Crying { .. }) {
                g.kids[i].state = State::Follow;
            } else if go(g, i, loki + Vec2::new(0.45, 0.0), true) < 0.35 {
                g.kids[i].state = State::Comfort { t: 2.6 };
                g.kids[i].facing = Dir::W;
                g.kids[LOKI].state = State::Crying { t: 2.6 };
                let line = lines::pick(&mut g.rng, lines::SKADI_COMFORT);
                g.say(Who::Skadi, line);
                g.events.push(SimEvent::Hug { pos: loki });
                g.discover(Id::SkadiComforts);
            }
        }
        State::Comfort { t } => {
            stand(g, i);
            let t = t - DT;
            if t <= 0.0 {
                g.kids[i].state = State::Follow;
                g.kids[LOKI].state = State::Follow;
                g.say(Who::Loki, "*sniff* ...okay.");
            } else {
                g.kids[i].state = State::Comfort { t };
            }
        }
        State::ChaseHen { t } => {
            let t = t - DT;
            if g.hen.held_by.is_some_and(|h| h != g.kids[i].who) {
                // The other one has her: grab on too.
                let other = 1 - i;
                if g.kids[other].holding_hen() && matches!(g.kids[other].state, State::HoldHen { .. }) && me.distance(g.hen.pos) < 1.4 {
                    g.kids[i].state = State::Tug { t: 2.4 };
                    g.kids[other].state = State::Tug { t: 2.4 };
                    let hp = g.hen.pos;
                    g.events.push(SimEvent::Tug { pos: hp });
                    g.discover(Id::TugOfWar);
                    g.say(Who::Loki, "MINE!");
                    g.say(Who::Skadi, "Loki, let GO!");
                    g.say(Who::Hen, "BWAAAK!");
                    return;
                }
            }
            let hp = g.hen.pos;
            if t <= 0.0 {
                g.kids[i].state = State::Follow;
            } else if go(g, i, hp, true) < 0.55 && g.hen.held_by.is_none() && !g.hen.busy() {
                g.hen.held_by = Some(g.kids[i].who);
                g.kids[i].state = State::HoldHen { t: g.rng.random_range(2.5..4.0) };
                g.events.push(SimEvent::Hug { pos: hp });
                if i == SKADI && g.town.grid.at(hp) == Terrain::Road {
                    let safe = safe_spot(g, hp);
                    g.kids[i].state = State::RescueHen { to: safe };
                    g.say(Who::Skadi, "Gullinkombi! Not in the road!");
                } else {
                    let who = g.kids[i].who;
                    g.say(who, if i == LOKI { "Chicken hug!" } else { "Aww, come here." });
                }
            } else {
                g.kids[i].state = State::ChaseHen { t };
            }
        }
        State::HoldHen { t } => {
            stand(g, i);
            let t = t - DT;
            if t <= 0.0 {
                let p = g.kids[i].pos + g.kids[i].facing.vec() * 0.5;
                release_hen(g, p);
                g.kids[i].state = State::Follow;
            } else {
                g.kids[i].state = State::HoldHen { t };
            }
        }
        State::RescueHen { to } => {
            if go(g, i, to, false) < 0.5 {
                release_hen(g, to);
                g.kids[i].state = State::Follow;
            }
        }
        State::Tug { t } => {
            stand(g, i);
            let t = t - DT;
            if t > 0.0 {
                g.kids[i].state = State::Tug { t };
                return;
            }
            // Resolve once, from Skaði's side (she is the second updated).
            if i == SKADI || !matches!(g.kids[1 - i].state, State::Tug { .. }) {
                let skadi_wins = g.rng.random_bool(0.6);
                let (win, lose) = if skadi_wins { (SKADI, LOKI) } else { (LOKI, SKADI) };
                g.hen.held_by = Some(g.kids[win].who);
                g.kids[win].state = State::HoldHen { t: 2.2 };
                g.kids[lose].state = State::Sulk { t: 3.0 };
                let who = g.kids[win].who;
                let pos = g.kids[win].pos;
                g.events.push(SimEvent::TugWon { who, pos });
                if lose == LOKI {
                    g.say(Who::Loki, "No fair!");
                } else {
                    g.say(Who::Skadi, "Fine. Be gentle with her.");
                }
            } else {
                g.kids[i].state = State::Tug { t: 0.0 };
            }
        }
        State::Sulk { t } => {
            stand(g, i);
            let t = t - DT;
            g.kids[i].state = if t <= 0.0 { State::Follow } else { State::Sulk { t } };
        }
        State::Fetch { item, purpose } => match items::find(g, item) {
            Some(idx) if g.items[idx].carried.is_none() => {
                let at = g.items[idx].pos;
                if go(g, i, at, i == SKADI && purpose == Purpose::GiveBack) < 0.5 {
                    let it = g.items.remove(idx);
                    // Loki eats candy he finds, unless it is a present.
                    if i == LOKI && matches!(it.kind, ItemKind::Candy(_)) && purpose != Purpose::Gift && g.rng.random_bool(0.6) {
                        g.kids[i].pail = g.kids[i].pail.saturating_add(1);
                        g.say(Who::Loki, "Mmm! Finders keepers!");
                        g.kids[i].state = State::Follow;
                    } else {
                        g.kids[i].state = State::Bring { what: it.kind, purpose };
                    }
                }
            }
            _ => g.kids[i].state = State::Follow,
        },
        State::Bring { what, purpose } => {
            let to = match purpose {
                Purpose::Elsewhere(p) => p,
                _ => rpos,
            };
            let left = go(g, i, to, false);
            let reach = if matches!(purpose, Purpose::Elsewhere(_)) { 0.5 } else { 1.0 };
            if left < reach || (matches!(purpose, Purpose::Elsewhere(_)) && g.kids[i].path_age > 8.0) {
                deliver_to(g, i, what, purpose);
                g.kids[i].state = State::Follow;
            }
        }
        State::Jump { t } => {
            stand(g, i);
            let t = t - DT;
            g.kids[i].state = if t <= 0.0 { State::Follow } else { State::Jump { t } };
        }
        State::LeafDive { feature, t } => {
            let at = g.town.features[feature].pos;
            if t > 0.0 {
                stand(g, i);
                let t = t - DT;
                g.kids[i].state = if t <= 0.0 { State::Follow } else { State::LeafDive { feature, t } };
            } else if g.feats[feature].used {
                g.kids[i].state = State::Follow;
            } else if go(g, i, at, true) < 0.6 {
                g.feats[feature].used = true;
                g.feats[feature].regrow = 120.0;
                g.events.push(SimEvent::Leaves { pos: at });
                g.say(Who::Loki, "CANNONBALL!");
                g.discover(Id::LokiLeafPile);
                g.kids[i].state = State::LeafDive { feature, t: 1.5 };
            }
        }
        State::PullTail { t } => {
            let tail = rpos - g.raccoon.facing.vec() * 0.6;
            if t > 0.0 {
                stand(g, i);
                let t = t - DT;
                g.kids[i].state = if t <= 0.0 { State::Follow } else { State::PullTail { t } };
            } else if go(g, i, tail, false) < 0.45 {
                if g.raccoon.busy() {
                    g.kids[i].state = State::Follow;
                    return;
                }
                let roll = g.rng.random();
                if g.raccoon.costume.wear(Wear::Pull, 0.7, roll).is_some() {
                    let pos = g.raccoon.pos;
                    g.events.push(SimEvent::Tear { pos });
                }
                g.say(Who::Loki, "Your tail is SO REAL!");
                g.say(Who::Raccoon, "*indignant chitter*");
                g.discover(Id::TailPulled);
                g.kids[i].state = State::PullTail { t: 0.8 };
            }
        }
    }
}

fn deliver_to(g: &mut Game, i: usize, what: ItemKind, purpose: Purpose) {
    let pos = g.kids[i].pos;
    match purpose {
        Purpose::GiveBack => {
            if let ItemKind::Candy(c) = what {
                g.raccoon.bag.add(c, 1);
                if g.kids[i].chat_cd <= 0.0 {
                    let line = lines::pick(&mut g.rng, lines::SKADI_GIVEBACK);
                    g.say(Who::Skadi, line);
                    g.kids[i].chat_cd = 5.0;
                }
            }
        }
        Purpose::Patch => {
            if let ItemKind::Mat(m) = what {
                if g.raccoon.costume.patch(m).is_some() {
                    let line = lines::pick(&mut g.rng, lines::SKADI_FIX);
                    g.say(Who::Skadi, line);
                    g.events.push(SimEvent::Patched { pos: g.raccoon.pos, mat: m });
                    g.discover(Id::SkadiFixes);
                    g.say_message(format!("Skaði patched the sheet with {}.", m.name()));
                } else {
                    g.raccoon.mats.add(m, 1);
                }
            }
        }
        Purpose::Gift => {
            match what {
                ItemKind::Candy(c) => g.raccoon.bag.add(c, 1),
                ItemKind::Mat(m) => g.raccoon.mats.add(m, 1),
                _ => {}
            }
            g.say(Who::Loki, "I got you a present!");
            g.discover(Id::LokiGift);
            let name = match what {
                ItemKind::Candy(c) => c.name(),
                ItemKind::Mat(m) => m.name(),
                _ => "something",
            };
            g.say_message(format!("Loki gives you {name}."));
        }
        Purpose::Elsewhere(_) => {
            items::drop_item(g, what, pos, Some(Who::Loki));
        }
    }
}

fn release_hen(g: &mut Game, at: Vec2) {
    let town = g.town.clone();
    let spot = if town.clear(at, 0.25, false) { at } else { g.hen.pos };
    g.hen.held_by = None;
    g.hen.pos = spot;
    g.hen.prev = spot;
    g.hen.vel = Vec2::ZERO;
}

/// The nearest point off the road (a sidewalk or a lawn) to `p`.
fn safe_spot(g: &Game, p: Vec2) -> Vec2 {
    let town = &g.town;
    for r in 1..10 {
        for k in 0..16 {
            let a = k as f32 / 16.0 * std::f32::consts::TAU;
            let q = p + Vec2::new(a.cos(), a.sin()) * r as f32;
            let t = town.grid.at(q);
            if t != Terrain::Road && t != Terrain::Water && t.walkable() && town.clear(q, 0.3, false) {
                return q;
            }
        }
    }
    p
}

/// Bumps: Loki running into the raccoon, Skaði running into it, and the
/// raccoon touching Loki while he carries an egg.
fn contacts(g: &mut Game) {
    let rpos = g.raccoon.pos;
    if matches!(g.raccoon.act, Act::Fallen { .. }) {
        return;
    }
    // Loki.
    let loki = g.kids[LOKI].clone();
    let d = loki.pos.distance(rpos);
    if d < 0.58 && loki.bump_cd <= 0.0 {
        if loki.carrying_egg() {
            g.items.retain(|it| !(it.carried == Some(Who::Loki) && it.kind == ItemKind::Egg));
            g.events.push(SimEvent::EggBroken { pos: loki.pos });
            g.events.push(SimEvent::Cry { pos: loki.pos });
            g.kids[LOKI].state = State::Crying { t: 9.0 };
            g.kids[LOKI].bump_cd = 4.0;
            let line = lines::pick(&mut g.rng, lines::LOKI_CRY);
            g.say(Who::Loki, line);
            g.discover(Id::EggBroken);
            g.say_message("Crack. Loki's egg. Loki is crying.");
            // Skaði drops whatever she is doing (gently, if it is the hen).
            if g.kids[SKADI].holding_hen() {
                let p = g.kids[SKADI].pos + g.kids[SKADI].facing.vec() * 0.5;
                release_hen(g, p);
            }
            if let State::Bring { what, .. } = g.kids[SKADI].state {
                let p = g.kids[SKADI].pos;
                items::drop_item(g, what, p, Some(Who::Raccoon));
            }
            g.kids[SKADI].state = State::GoComfort;
            g.say(Who::Skadi, "Loki! I'm coming!");
            return;
        }
        let fast = loki.vel.length() > 3.6;
        if fast && g.raccoon.shaken <= 0.0 && g.raccoon.act != Act::AtDoor {
            let push = (rpos - loki.pos).normalize_or_zero() * 2.0;
            g.raccoon.vel = push;
            g.raccoon.shaken = 1.5;
            let n = raccoon::spill(g, Who::Loki, 0.1, 1);
            g.kids[LOKI].bump_cd = 4.0;
            g.kids[LOKI].state = State::Follow;
            let line = lines::pick(&mut g.rng, lines::LOKI_OOPS);
            g.say(Who::Loki, line);
            g.discover(Id::LokiBump);
            if n > 0 {
                g.say_message(format!("Loki crashes into you. {n} pieces go on the ground."));
            }
            return;
        }
    }
    // Skaði, in a hurry.
    let skadi = g.kids[SKADI].clone();
    let d = skadi.pos.distance(rpos);
    if d < 0.62 && skadi.bump_cd <= 0.0 && skadi.vel.length() > 3.6 && g.raccoon.shaken <= 0.0 && g.raccoon.act != Act::AtDoor {
        let push = (rpos - skadi.pos).normalize_or_zero() * 3.0;
        let n = raccoon::knock_down(g, Who::Skadi, 0.25, 2, push);
        g.kids[SKADI].bump_cd = 5.0;
        let line = lines::pick(&mut g.rng, lines::SKADI_SORRY);
        g.say(Who::Skadi, line);
        g.discover(Id::SkadiBump);
        g.say_message(format!("Skaði bowls you over. {n} pieces go flying."));
        return;
    }
    // Not bumping: keep a polite distance.
    for i in 0..2 {
        let k = &mut g.kids[i];
        let dv = k.pos - rpos;
        let d = dv.length();
        if d < 0.62 && d > 0.0 && !matches!(k.state, State::PullTail { .. } | State::Bring { .. }) {
            let push = dv / d * (0.62 - d) * 0.5;
            let town = g.town.clone();
            move_body(&town, &mut g.kids[i].pos, push, RADIUS, false);
        }
    }
}

/// Their mother hands hákarl to her children when they come by.
fn mom(g: &mut Game) {
    let step = g.town.buildings[g.town.home].door.step();
    for i in 0..2 {
        if g.kids[i].mom_cd <= 0.0 && g.kids[i].pos.distance(step) < 2.2 && matches!(g.kids[i].state, State::Follow | State::Wander { .. }) {
            g.kids[i].mom_cd = 90.0;
            g.say(Who::Door(g.town.home as u32), "There you are! Hákarl!");
            receive(g, i, Candy::Hakarl);
        }
    }
}

/// Now and then a child says something.
fn chatter(g: &mut Game, i: usize) {
    if g.kids[i].chat_cd > 0.0 || !matches!(g.kids[i].state, State::Follow) {
        return;
    }
    g.kids[i].chat_cd = g.rng.random_range(14.0..30.0);
    if g.kids[i].pos.distance(g.raccoon.pos) > 8.0 {
        return;
    }
    let (who, line) = if i == LOKI {
        (Who::Loki, lines::pick(&mut g.rng, lines::LOKI))
    } else {
        (Who::Skadi, lines::pick(&mut g.rng, lines::SKADI))
    };
    g.say(who, line);
}

/// The mats a child can use for patching, for tests.
pub fn patch_mats() -> [Mat; 3] {
    [Mat::Paper, Mat::Napkin, Mat::Gum]
}
