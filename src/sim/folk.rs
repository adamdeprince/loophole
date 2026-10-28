//! Everybody else out on Halloween night: families trick-or-treating
//! (whose children empty the same bowls the raccoon wants, and grab
//! whatever it drops), teenagers, dog walkers, joggers, leaf peepers,
//! professors from the college, a legislator, maple farmers, and the
//! neighbours, each with their own temper.

use super::candy::Candy;
use super::costume::Wear;
use super::discover::Id;
use super::items::{self, ItemKind};
use super::raccoon::{self, Act};
use super::{DT, Game, SimEvent, Who, doors, facing_of, lines, move_body, steer};
use crate::world::Dir;
use crate::world::grid::Terrain;
use crate::world::nav;
use crate::world::plan::Town;
use crate::world::town::BuildingKind;
use glam::Vec2;
use rand::RngExt;
use rand_chacha::ChaCha8Rng;
use std::collections::BTreeSet;

pub const RADIUS: f32 = 0.27;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Kind {
    Parent,
    /// A child in costume, with a family.
    Kid,
    Teen,
    DogWalker,
    Dog,
    Jogger,
    /// Here for the foliage, with a camera.
    Tourist,
    Professor,
    Legislator,
    Farmer,
    Neighbour,
}

impl Kind {
    pub fn is_child(self) -> bool {
        matches!(self, Kind::Kid)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Temper {
    Friendly,
    Suspicious,
    Oblivious,
    Grumpy,
    Scared,
}

/// What a child is dressed as.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Costume {
    Witch,
    Pirate,
    Robot,
    /// Like the statues.
    Cow,
    /// A bottle of maple syrup.
    Syrup,
    Skeleton,
    Superhero,
    Pumpkin,
    /// A sheet with eye holes. Like somebody else we know.
    Ghost,
    Astronaut,
}

impl Costume {
    pub const ALL: [Costume; 10] = [
        Costume::Witch,
        Costume::Pirate,
        Costume::Robot,
        Costume::Cow,
        Costume::Syrup,
        Costume::Skeleton,
        Costume::Superhero,
        Costume::Pumpkin,
        Costume::Ghost,
        Costume::Astronaut,
    ];
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Goal {
    Stroll { to: Vec2 },
    /// Heading for a door (the family's next house).
    Door { b: usize },
    AtDoor { b: usize, t: f32 },
    /// Following a parent, or an owner.
    Follow,
    Flee { from: Vec2, t: f32 },
    /// Stopped to look at the raccoon.
    Stare { t: f32 },
    /// Going for candy on the ground.
    Grab { item: u32 },
    /// A teenager, going for the raccoon's bag.
    Snatch { t: f32 },
    /// A dog that has seen the raccoon.
    Chase { t: f32 },
}

#[derive(Clone, Debug)]
pub struct Walker {
    pub id: u32,
    pub kind: Kind,
    pub temper: Temper,
    pub costume: Option<Costume>,
    /// Picks clothes, hair and build in the art.
    pub look: u32,
    pub pos: Vec2,
    pub prev: Vec2,
    pub vel: Vec2,
    pub facing: Dir,
    pub goal: Goal,
    pub leader: Option<u32>,
    pub path: Vec<Vec2>,
    pub path_to: Option<Vec2>,
    pub path_age: f32,
    pub pail: u64,
    pub chat_cd: f32,
    pub seen_cd: f32,
    pub think: f32,
    /// The houses a family has been to tonight.
    pub visited: BTreeSet<usize>,
    pub stride: f32,
}

impl Walker {
    fn speed(&self) -> f32 {
        match self.kind {
            Kind::Jogger => 4.6,
            Kind::Dog => 3.4,
            Kind::Kid => 3.0,
            Kind::Teen => 3.2,
            _ => 2.4,
        }
    }

    fn new(id: u32, kind: Kind, at: Vec2, rng: &mut ChaCha8Rng) -> Walker {
        let temper = match kind {
            Kind::Neighbour | Kind::Parent => match rng.random_range(0..10) {
                0..=3 => Temper::Friendly,
                4..=5 => Temper::Suspicious,
                6..=7 => Temper::Oblivious,
                8 => Temper::Grumpy,
                _ => Temper::Scared,
            },
            Kind::Kid => {
                if rng.random_bool(0.6) {
                    Temper::Friendly
                } else {
                    Temper::Suspicious
                }
            }
            Kind::Tourist | Kind::Professor | Kind::Legislator | Kind::Farmer => Temper::Friendly,
            Kind::Teen | Kind::Dog => Temper::Grumpy,
            Kind::Jogger | Kind::DogWalker => Temper::Oblivious,
        };
        Walker {
            id,
            kind,
            temper,
            costume: (kind == Kind::Kid).then(|| Costume::ALL[rng.random_range(0..Costume::ALL.len())]),
            look: rng.random(),
            pos: at,
            prev: at,
            vel: Vec2::ZERO,
            facing: Dir::S,
            goal: Goal::Stroll { to: at },
            leader: None,
            path: Vec::new(),
            path_to: None,
            path_age: 0.0,
            pail: 0,
            chat_cd: rng.random_range(4.0..20.0),
            seen_cd: 0.0,
            think: rng.random_range(0.0..2.0),
            visited: BTreeSet::new(),
            stride: 0.0,
        }
    }
}

/// A random walkable spot on a sidewalk or path, optionally near `near`.
fn spot(town: &Town, rng: &mut ChaCha8Rng, near: Option<(Vec2, f32)>) -> Vec2 {
    for _ in 0..400 {
        let p = match near {
            Some((c, r)) => c + Vec2::new(rng.random_range(-r..r), rng.random_range(-r..r)),
            None => Vec2::new(rng.random_range(1.0..town.w as f32 - 1.0), rng.random_range(1.0..town.h as f32 - 1.0)),
        };
        if matches!(town.grid.at(p), Terrain::Sidewalk | Terrain::Path | Terrain::Plaza) && town.clear(p, RADIUS, false) {
            return p;
        }
    }
    town.start
}

pub fn populate(town: &Town, rng: &mut ChaCha8Rng) -> Vec<Walker> {
    let mut out = Vec::new();
    let mut id = 1u32;
    let mut add = |out: &mut Vec<Walker>, kind: Kind, at: Vec2, rng: &mut ChaCha8Rng| {
        let w = Walker::new(id, kind, at, rng);
        id += 1;
        out.push(w);
        id - 1
    };
    let landmark = |k: BuildingKind| town.buildings.iter().find(|b| b.kind == k).map(|b| b.door.step());
    // Most of the people out tonight are children trick-or-treating: about
    // two in three. Families with a parent, and bands of older kids on
    // their own (the eldest leads).
    for f in 0..26 {
        let at = spot(town, rng, None);
        let lead = if f < 19 { add(&mut out, Kind::Parent, at, rng) } else { add(&mut out, Kind::Kid, at, rng) };
        for _ in 0..rng.random_range(2..5) {
            let off = Vec2::new(rng.random_range(-0.8..0.8), rng.random_range(-0.8..0.8));
            add(&mut out, Kind::Kid, at + off, rng);
            if let Some(w) = out.last_mut() {
                w.leader = Some(lead);
                w.goal = Goal::Follow;
            }
        }
    }
    // Teenagers in no costume at all, with pillowcases.
    for _ in 0..4 {
        let at = spot(town, rng, None);
        add(&mut out, Kind::Teen, at, rng);
    }
    for _ in 0..3 {
        let at = spot(town, rng, None);
        let owner = add(&mut out, Kind::DogWalker, at, rng);
        add(&mut out, Kind::Dog, at + Vec2::new(0.6, 0.3), rng);
        if let Some(w) = out.last_mut() {
            w.leader = Some(owner);
            w.goal = Goal::Follow;
        }
    }
    for _ in 0..2 {
        let at = spot(town, rng, None);
        add(&mut out, Kind::Jogger, at, rng);
    }
    let downtown = Vec2::new(135.0, 130.0);
    for _ in 0..3 {
        let at = spot(town, rng, Some((downtown, 25.0)));
        add(&mut out, Kind::Tourist, at, rng);
    }
    if let Some(c) = landmark(BuildingKind::College) {
        for _ in 0..2 {
            let at = spot(town, rng, Some((c, 12.0)));
            add(&mut out, Kind::Professor, at, rng);
        }
    }
    if let Some(s) = landmark(BuildingKind::StateHouse) {
        let at = spot(town, rng, Some((s, 12.0)));
        add(&mut out, Kind::Legislator, at, rng);
    }
    for _ in 0..2 {
        let at = spot(town, rng, Some((downtown, 30.0)));
        add(&mut out, Kind::Farmer, at, rng);
    }
    for _ in 0..6 {
        let at = spot(town, rng, None);
        add(&mut out, Kind::Neighbour, at, rng);
    }
    out
}

fn index_of(g: &Game, id: u32) -> Option<usize> {
    g.folk.iter().position(|w| w.id == id)
}

pub fn step(g: &mut Game) {
    for i in 0..g.folk.len() {
        g.folk[i].prev = g.folk[i].pos;
        let w = &mut g.folk[i];
        w.chat_cd = (w.chat_cd - DT).max(0.0);
        w.seen_cd = (w.seen_cd - DT).max(0.0);
        w.path_age += DT;
        w.think -= DT;
        if w.think <= 0.0 {
            w.think = g.rng.random_range(0.4..0.8);
            decide(g, i);
            notice(g, i);
        }
        act(g, i);
    }
    hakarl_lady(g);
}

/// Chooses what to do next when idle.
fn decide(g: &mut Game, i: usize) {
    let w = g.folk[i].clone();
    // Children go for candy on the ground first.
    if w.kind == Kind::Kid && !matches!(w.goal, Goal::Flee { .. } | Goal::Grab { .. } | Goal::AtDoor { .. } | Goal::Door { .. }) {
        if let Some(item) = items::nearest(g, w.pos, 4.5, |it| matches!(it.kind, ItemKind::Candy(_))) {
            g.folk[i].goal = Goal::Grab { item };
            return;
        }
    }
    let idle = match w.goal {
        Goal::Stroll { to } => w.pos.distance(to) < 0.8 || w.path_age > 40.0,
        _ => false,
    };
    if !idle {
        return;
    }
    let town = g.town.clone();
    let leads = w.kind == Kind::Parent || (w.kind == Kind::Kid && w.leader.is_none());
    match w.kind {
        _ if leads => {
            // The next house with its light on, not visited yet.
            let next = town
                .buildings
                .iter()
                .filter(|b| g.houses[b.id].light && g.houses[b.id].answers() && !w.visited.contains(&b.id))
                .filter(|b| matches!(b.kind, BuildingKind::House | BuildingKind::Home) || b.kind.is_commercial())
                .map(|b| (b.id, b.door.step().distance(w.pos)))
                .filter(|(_, d)| *d < 30.0)
                .min_by(|a, b| a.1.total_cmp(&b.1));
            match next {
                Some((b, _)) => g.folk[i].goal = Goal::Door { b },
                None => {
                    let to = spot(&town, &mut g.rng, Some((w.pos, 20.0)));
                    g.folk[i].goal = Goal::Stroll { to };
                    // A new street: forget the old one.
                    if g.folk[i].visited.len() > 40 {
                        g.folk[i].visited.clear();
                    }
                }
            }
        }
        _ => {
            let r = if matches!(w.kind, Kind::Professor | Kind::Legislator) { 12.0 } else { 30.0 };
            let to = spot(&town, &mut g.rng, Some((w.pos, r)));
            g.folk[i].goal = Goal::Stroll { to };
        }
    }
}

/// Seeing the raccoon.
fn notice(g: &mut Game, i: usize) {
    let w = g.folk[i].clone();
    let d = w.pos.distance(g.raccoon.pos);
    if d > 5.0 || w.seen_cd > 0.0 || matches!(w.goal, Goal::Flee { .. }) {
        return;
    }
    let passing = g.raccoon.passing();
    let me = Who::Folk(w.id);
    g.folk[i].seen_cd = g.rng.random_range(18.0..35.0);
    let rpos = g.raccoon.pos;
    match w.kind {
        Kind::Kid => {
            // A badly kept costume frightens children: they see the fur, the
            // whiskers, the mud. They run, and drop things.
            let spotted = passing < 0.5 || (passing < 0.62 && g.rng.random_bool(0.3));
            if spotted {
                let line = lines::pick(&mut g.rng, lines::RIVAL_RACCOON);
                g.say(me, line);
                g.folk[i].goal = Goal::Flee { from: rpos, t: 3.0 };
                g.events.push(SimEvent::Flee { pos: w.pos });
                doors::alarm(g, w.pos, 0.1);
                if g.rng.random_bool(0.45) {
                    spill_pail(g, i, rpos);
                }
                // Their friends run too.
                let lead = w.leader.unwrap_or(w.id);
                for k in 0..g.folk.len() {
                    let o = &g.folk[k];
                    if k != i && o.kind == Kind::Kid && (o.leader == Some(lead) || o.id == lead) && o.pos.distance(w.pos) < 4.0 {
                        g.folk[k].goal = Goal::Flee { from: rpos, t: 3.0 };
                        g.folk[k].seen_cd = 20.0;
                        if g.rng.random_bool(0.3) {
                            spill_pail(g, k, rpos);
                        }
                    }
                }
            } else if w.costume == Some(Costume::Ghost) {
                let line = lines::pick(&mut g.rng, lines::TWINS);
                g.say(me, line);
                g.discover(Id::Twins);
            } else if g.rng.random_bool(0.5) {
                let line = lines::pick(&mut g.rng, lines::RIVAL);
                g.say(me, line);
            }
        }
        Kind::Teen => {
            if g.raccoon.bag.total() > 25 && g.rng.random_bool(0.6) {
                g.folk[i].goal = Goal::Snatch { t: 6.0 };
            }
        }
        Kind::Dog => {
            g.say(me, "WOOF! WOOF!");
            g.events.push(SimEvent::Bark { pos: w.pos });
            g.discover(Id::DogBark);
            if passing < 0.7 || g.rng.random_bool(0.3) {
                g.folk[i].goal = Goal::Chase { t: 3.0 };
            }
        }
        Kind::Tourist => {
            let line = lines::pick(&mut g.rng, lines::TOURIST);
            g.say(me, line);
            g.events.push(SimEvent::Flash { pos: w.pos });
            g.discover(Id::Photographed);
            g.folk[i].goal = Goal::Stare { t: 2.0 };
        }
        Kind::Professor => {
            let line = lines::pick(&mut g.rng, lines::PROFESSOR);
            g.say(me, line);
            g.folk[i].goal = Goal::Stare { t: 1.5 };
        }
        Kind::Legislator => {
            let line = lines::pick(&mut g.rng, lines::LEGISLATOR);
            g.say(me, line);
        }
        Kind::Farmer => {
            let line = lines::pick(&mut g.rng, lines::FARMER);
            g.say(me, line);
            if passing > 0.4 && g.rng.random_bool(0.5) {
                g.raccoon.bag.add(Candy::MapleLeaf, 1);
                g.say_message("A maple farmer hands you a maple sugar leaf. \"Grade A.\"");
            }
        }
        Kind::Parent | Kind::Neighbour => match w.temper {
            Temper::Friendly => {
                let line = lines::pick(&mut g.rng, lines::FOLK_FRIENDLY);
                g.say(me, line);
                g.raccoon.happy = (g.raccoon.happy + 0.02).min(1.0);
            }
            Temper::Suspicious => {
                if passing < 0.62 {
                    let line = lines::pick(&mut g.rng, lines::FOLK_SUSPICIOUS);
                    g.say(me, line);
                    g.folk[i].goal = Goal::Stare { t: 2.5 };
                    doors::alarm(g, w.pos, 0.12);
                }
            }
            Temper::Oblivious => {
                let line = lines::pick(&mut g.rng, lines::FOLK_OBLIVIOUS);
                g.say(me, line);
            }
            Temper::Grumpy => {
                let line = lines::pick(&mut g.rng, lines::FOLK_GRUMPY);
                g.say(me, line);
            }
            Temper::Scared => {
                if passing < 0.55 {
                    let line = lines::pick(&mut g.rng, lines::FOLK_SCARED);
                    g.say(me, line);
                    g.events.push(SimEvent::Scream { pos: w.pos });
                    g.folk[i].goal = Goal::Flee { from: rpos, t: 3.0 };
                    doors::alarm(g, w.pos, 0.1);
                }
            }
        },
        Kind::DogWalker | Kind::Jogger => {}
    }
}

/// A frightened child drops some of their candy as they run.
fn spill_pail(g: &mut Game, i: usize, from: Vec2) {
    let n = g.rng.random_range(1..4).min(g.folk[i].pail.max(1) as u32 + 1);
    let pos = g.folk[i].pos;
    let away = (pos - from).normalize_or_zero();
    for _ in 0..n {
        let c = doors::random_treat(&mut g.rng);
        let a: f32 = g.rng.random_range(-1.2..1.2);
        let dir = Vec2::new(away.x * a.cos() - away.y * a.sin(), away.x * a.sin() + away.y * a.cos());
        let vel = -dir * g.rng.random_range(1.0..2.6);
        let id = g.next_item;
        g.next_item += 1;
        g.items.push(items::Item::new(id, ItemKind::Candy(c), pos, vel, Some(Who::Folk(g.folk[i].id))));
    }
    g.folk[i].pail = g.folk[i].pail.saturating_sub(n as u64);
    g.events.push(SimEvent::CandyDropped { pos, n, by: Who::Folk(g.folk[i].id) });
    g.say_message("A frightened kid drops candy as they run. Finders keepers!");
}

/// Walks walker `i` towards `to`; returns the distance left.
fn go(g: &mut Game, i: usize, to: Vec2, speed: f32) -> f32 {
    let town = g.town.clone();
    let w = &mut g.folk[i];
    let left = w.pos.distance(to);
    let mut aim = to;
    if !nav::line_clear(&town, w.pos, to, false) {
        let stale = w.path_to.is_none_or(|p| p.distance(to) > 2.0) || w.path.is_empty() || (w.path_age > 3.0 && w.vel.length() < 0.2);
        if stale {
            w.path = nav::path(&town, w.pos, to, false).unwrap_or_default();
            w.path_to = Some(to);
            w.path_age = 0.0;
            if !w.path.is_empty() {
                w.path.remove(0);
            }
        }
        while w.path.len() > 1 && w.path[0].distance(w.pos) < 0.4 {
            w.path.remove(0);
        }
        if let Some(&p) = w.path.first() {
            aim = p;
        }
    } else {
        w.path.clear();
    }
    let d = aim - w.pos;
    let want = if d.length() > 0.05 { d.normalize() * speed.min(d.length() / DT) } else { Vec2::ZERO };
    w.vel = steer(w.vel, want, 18.0);
    w.facing = facing_of(w.vel, w.facing);
    let before = w.pos;
    let dv = w.vel * DT;
    move_body(&town, &mut w.pos, dv, RADIUS, false);
    w.stride += w.pos.distance(before);
    left
}

fn halt(g: &mut Game, i: usize) {
    g.folk[i].vel = Vec2::ZERO;
}

fn act(g: &mut Game, i: usize) {
    let w = g.folk[i].clone();
    let speed = w.speed();
    match w.goal {
        Goal::Stroll { to } => {
            go(g, i, to, speed);
        }
        Goal::Door { b } => {
            let step = g.town.buildings[b].door.step();
            let facing = g.town.buildings[b].door.facing;
            // Parents wait at the bottom of the walk; the kids go up.
            let wait = step + facing.vec() * 1.6;
            if go(g, i, wait, speed) < 0.6 {
                g.folk[i].goal = Goal::AtDoor { b, t: 3.0 };
            }
        }
        Goal::AtDoor { b, t } => {
            halt(g, i);
            let t = t - DT;
            if t <= 0.0 {
                g.folk[i].visited.insert(b);
                // Each child takes their share of the bowl.
                let mut kids: Vec<usize> = (0..g.folk.len()).filter(|&k| g.folk[k].leader == Some(w.id) && g.folk[k].kind == Kind::Kid).collect();
                if w.kind == Kind::Kid {
                    kids.push(i);
                }
                let mom = g.town.home == b;
                for k in kids {
                    if mom {
                        continue;
                    }
                    let n = doors::rival_takes(g, b);
                    g.folk[k].pail = g.folk[k].pail.saturating_add(n as u64);
                }
                g.folk[i].goal = Goal::Stroll { to: w.pos };
                g.folk[i].path_age = 100.0;
            } else {
                g.folk[i].goal = Goal::AtDoor { b, t };
            }
        }
        Goal::Follow => {
            let Some(li) = w.leader.and_then(|l| index_of(g, l)) else {
                g.folk[i].goal = Goal::Stroll { to: w.pos };
                return;
            };
            let leader = g.folk[li].clone();
            // At a door the children run up to it.
            let target = match leader.goal {
                Goal::AtDoor { b, .. } if w.kind == Kind::Kid => {
                    let step = g.town.buildings[b].door.step();
                    step + Vec2::new(((w.id % 3) as f32 - 1.0) * 0.5, 0.0)
                }
                _ => {
                    let k = (w.id % 4) as f32;
                    leader.pos - leader.facing.vec() * (0.9 + k * 0.3) + Vec2::new((k - 1.5) * 0.45, 0.0)
                }
            };
            if w.pos.distance(target) > 0.5 {
                let sp = if w.pos.distance(target) > 4.0 { speed * 1.3 } else { speed };
                go(g, i, target, sp);
            } else {
                halt(g, i);
            }
        }
        Goal::Flee { from, t } => {
            let t = t - DT;
            let away = w.pos + (w.pos - from).normalize_or_zero() * 4.0;
            go(g, i, away, speed * 1.4);
            g.folk[i].goal = if t <= 0.0 {
                if w.leader.is_some() { Goal::Follow } else { Goal::Stroll { to: w.pos } }
            } else {
                Goal::Flee { from, t }
            };
        }
        Goal::Stare { t } => {
            halt(g, i);
            let d = g.raccoon.pos - w.pos;
            g.folk[i].facing = Dir::of(d);
            let t = t - DT;
            g.folk[i].goal = if t <= 0.0 {
                if w.leader.is_some() { Goal::Follow } else { Goal::Stroll { to: w.pos } }
            } else {
                Goal::Stare { t }
            };
        }
        Goal::Grab { item } => match items::find(g, item) {
            Some(idx) if g.items[idx].carried.is_none() && g.items[idx].z <= 0.05 => {
                let at = g.items[idx].pos;
                if go(g, i, at, speed * 1.2) < 0.45 {
                    let it = g.items.remove(idx);
                    g.folk[i].pail = g.folk[i].pail.saturating_add(1);
                    g.events.push(SimEvent::Snatch { pos: at });
                    if it.from == Some(Who::Raccoon) && g.folk[i].chat_cd <= 0.0 {
                        g.say(Who::Folk(w.id), "Finders keepers!");
                        g.folk[i].chat_cd = 6.0;
                    }
                    g.folk[i].goal = if w.leader.is_some() { Goal::Follow } else { Goal::Stroll { to: w.pos } };
                }
            }
            _ => g.folk[i].goal = if w.leader.is_some() { Goal::Follow } else { Goal::Stroll { to: w.pos } },
        },
        Goal::Snatch { t } => {
            let t = t - DT;
            let rpos = g.raccoon.pos;
            if t <= 0.0 || g.raccoon.act == Act::AtDoor {
                g.folk[i].goal = Goal::Stroll { to: w.pos };
            } else if go(g, i, rpos, speed * 1.5) < 0.6 {
                let line = lines::pick(&mut g.rng, lines::TEEN);
                g.say(Who::Folk(w.id), line);
                let n = raccoon::spill(g, Who::Folk(w.id), 0.06, 2);
                g.events.push(SimEvent::Snatch { pos: rpos });
                g.discover(Id::TeenSnatch);
                g.say_message(format!("A teenager knocks {n} pieces out of your bag."));
                g.folk[i].goal = Goal::Stroll { to: w.pos + (w.pos - rpos).normalize_or_zero() * 10.0 };
            } else {
                g.folk[i].goal = Goal::Snatch { t };
            }
        }
        Goal::Chase { t } => {
            let t = t - DT;
            let rpos = g.raccoon.pos;
            let owner_far = w.leader.and_then(|l| index_of(g, l)).is_some_and(|o| g.folk[o].pos.distance(w.pos) > 6.0);
            if t <= 0.0 || owner_far {
                g.folk[i].goal = Goal::Follow;
            } else if go(g, i, rpos, speed * 1.2) < 0.55 && g.raccoon.shaken <= 0.0 && !g.raccoon.busy() {
                let roll = g.rng.random();
                if g.raccoon.costume.wear(Wear::Pull, 0.8, roll).is_some() {
                    g.events.push(SimEvent::Tear { pos: rpos });
                }
                g.raccoon.shaken = 2.0;
                g.say(Who::Folk(w.id), "GRRRR!");
                if let Some(o) = w.leader {
                    g.say(Who::Folk(o), "Maple! Leave it! Sorry, kid!");
                }
                g.say_message("A dog grabs the sheet and gives it a shake.");
                g.folk[i].goal = Goal::Follow;
            } else {
                g.folk[i].goal = Goal::Chase { t };
            }
        }
    }
    // Joggers and children on the run knock the raccoon about.
    if matches!(w.kind, Kind::Jogger) && g.folk[i].vel.length() > 3.5 {
        let d = g.folk[i].pos.distance(g.raccoon.pos);
        if d < 0.55 && g.raccoon.shaken <= 0.0 && !g.raccoon.busy() {
            let push = (g.raccoon.pos - g.folk[i].pos).normalize_or_zero() * 2.5;
            raccoon::knock_down(g, Who::Folk(w.id), 0.08, 1, push);
            g.say(Who::Folk(w.id), "Whoa! Sorry, little guy!");
        }
    }
}

/// The children's mother offers her hákarl to every child who comes near
/// her porch. They do not stay to try it.
fn hakarl_lady(g: &mut Game) {
    if g.tick % 30 != 0 {
        return;
    }
    let step = g.town.buildings[g.town.home].door.step();
    let mut fled = false;
    for i in 0..g.folk.len() {
        let w = &g.folk[i];
        if w.kind != Kind::Kid || w.pos.distance(step) > 3.2 || matches!(w.goal, Goal::Flee { .. }) {
            continue;
        }
        let id = w.id;
        let line = lines::pick(&mut g.rng, lines::RIVAL_HAKARL);
        g.say(Who::Folk(id), line);
        g.folk[i].goal = Goal::Flee { from: step, t: 4.0 };
        fled = true;
    }
    if fled {
        g.say(Who::Door(g.town.home as u32), "Hákarl? Anybody? It's my grandmother's recipe!");
        g.events.push(SimEvent::Flee { pos: step });
        if g.raccoon.pos.distance(step) < 18.0 {
            g.discover(Id::RivalsFlee);
        }
    }
}
