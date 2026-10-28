//! Presentation: the game seen through the renderer. Which frame of which
//! sheet everybody is showing, where (between ticks), the speech bubbles,
//! the state of every decal, and short effects (sparkles, puffs, splashes)
//! that the simulation does not need to know about.

use crate::art::people;
use crate::art::raccoon::bag_size;
use crate::gfx::scenery::{self, WorldArt};
use crate::render::{Bubble, Layer, SheetId, Sprite};
use crate::sim::folk::{self, Kind};
use crate::sim::items::ItemKind;
use crate::sim::kids::State as KidState;
use crate::sim::raccoon::Act;
use crate::sim::{Game, SimEvent, Who, cat, gravy, hen};
use crate::world::Dir;
use crate::world::town::BuildingKind;
use glam::Vec2;
use rand::{RngExt, SeedableRng};
use rand_chacha::ChaCha8Rng;

#[derive(Clone, Debug)]
struct Effect {
    anim: &'static str,
    pos: Vec2,
    vel: Vec2,
    age: f32,
    life: f32,
    key: u32,
}

pub struct Show {
    effects: Vec<Effect>,
    rng: ChaCha8Rng,
    next_key: u32,
    pub time: f32,
    /// Particles on (off on e-ink displays).
    pub effects_on: bool,
}

// Placement keys: fixed ones for the cast, ranges for the rest.
const K_RACCOON: u32 = 1;
const K_BAG: u32 = 2;
const K_LOKI: u32 = 3;
const K_SKADI: u32 = 4;
const K_HEN: u32 = 5;
const K_GRAVY: u32 = 6;
const K_CAT: u32 = 7;
const K_MOM: u32 = 8;
const K_ETHAN: u32 = 9;
const K_DOOR: u32 = 10;
const K_HAT: u32 = 11;
const K_KEY: u32 = 12;
const K_EGG: u32 = 13;
const K_FOLK: u32 = 1_000;
const K_ITEMS: u32 = 1_000_000;
const K_FX: u32 = 2_000_000_000;

fn dir_name(d: Dir) -> &'static str {
    match d {
        Dir::N => "up",
        Dir::S => "down",
        Dir::E => "right",
        Dir::W => "left",
    }
}

fn side(d: Dir) -> &'static str {
    if d == Dir::W { "left" } else { "right" }
}

impl Default for Show {
    fn default() -> Self {
        Self::new()
    }
}

impl Show {
    pub fn new() -> Show {
        Show {
            effects: Vec::new(),
            rng: ChaCha8Rng::seed_from_u64(0x5_0a11),
            next_key: 0,
            time: 0.0,
            effects_on: true,
        }
    }

    pub fn clear(&mut self) {
        self.effects.clear();
    }

    pub fn update(&mut self, dt: f32) {
        self.time += dt;
        for e in &mut self.effects {
            e.age += dt;
            e.pos += e.vel * dt;
            e.vel *= 0.94;
        }
        self.effects.retain(|e| e.age < e.life);
    }

    fn spawn(&mut self, anim: &'static str, pos: Vec2, vel: Vec2, life: f32) {
        if !self.effects_on {
            return;
        }
        self.next_key = self.next_key.wrapping_add(1) % 1_000_000;
        self.effects.push(Effect {
            anim,
            pos,
            vel,
            age: 0.0,
            life,
            key: K_FX + self.next_key,
        });
    }

    fn burst(&mut self, anim: &'static str, pos: Vec2, n: usize, speed: f32, life: f32) {
        for _ in 0..n {
            let a: f32 = self.rng.random_range(0.0..std::f32::consts::TAU);
            let s = self.rng.random_range(speed * 0.4..speed);
            self.spawn(anim, pos, Vec2::new(a.cos(), a.sin() * 0.6) * s, life);
        }
    }

    pub fn on_event(&mut self, e: &SimEvent) {
        match e {
            SimEvent::Given { pos, .. } => self.burst("sparkle", *pos - Vec2::new(0.0, 0.8), 6, 2.0, 0.7),
            SimEvent::Loved { pos } => {
                self.burst("heart", *pos - Vec2::new(0.0, 1.2), 4, 1.5, 1.0);
                self.burst("star", *pos - Vec2::new(0.0, 1.0), 4, 2.5, 0.8);
            }
            SimEvent::Refused { pos, .. } => self.spawn("reject", *pos - Vec2::new(0.0, 1.6), Vec2::new(0.0, -0.4), 1.0),
            SimEvent::Scream { pos } | SimEvent::Flee { pos } => self.spawn("emote_fright", *pos - Vec2::new(0.0, 1.8), Vec2::ZERO, 1.0),
            SimEvent::Tear { pos } => self.burst("puff", *pos - Vec2::new(0.0, 0.5), 2, 1.0, 0.5),
            SimEvent::Patched { pos, .. } => self.burst("sparkle", *pos - Vec2::new(0.0, 0.6), 4, 1.2, 0.6),
            SimEvent::Splash { pos } => self.spawn("splash", *pos, Vec2::ZERO, 0.5),
            SimEvent::Leaves { pos } => self.burst("feather", *pos - Vec2::new(0.0, 0.3), 10, 3.0, 1.1),
            SimEvent::Berries { pos } => self.burst("sparkle", *pos, 3, 1.0, 0.5),
            SimEvent::EggBroken { pos } => self.spawn("splat", *pos, Vec2::ZERO, 0.8),
            SimEvent::Cry { pos } => self.spawn("emote_sweat", *pos - Vec2::new(0.0, 1.6), Vec2::ZERO, 1.6),
            SimEvent::Hug { pos } | SimEvent::Jump { pos, .. } => self.burst("heart", *pos - Vec2::new(0.0, 1.4), 3, 1.0, 1.0),
            SimEvent::Tug { pos } => self.spawn("emote_angry", *pos - Vec2::new(0.0, 1.8), Vec2::ZERO, 2.0),
            SimEvent::TugWon { pos, .. } => self.burst("star", *pos - Vec2::new(0.0, 1.2), 3, 1.2, 0.7),
            SimEvent::Squawk { pos } => self.burst("feather", *pos - Vec2::new(0.0, 0.4), 6, 2.0, 0.9),
            SimEvent::Fall { pos, .. } | SimEvent::Slip { pos } => {
                self.burst("dust", *pos, 4, 1.4, 0.5);
                self.spawn("star", *pos - Vec2::new(0.0, 0.8), Vec2::new(0.0, -0.3), 0.9);
            }
            SimEvent::CandyDropped { pos, .. } | SimEvent::Snatch { pos } => self.burst("sparkle", *pos - Vec2::new(0.0, 0.4), 3, 1.5, 0.5),
            SimEvent::Bark { pos } => self.spawn("emote_alert", *pos - Vec2::new(0.0, 1.0), Vec2::ZERO, 0.8),
            SimEvent::Flash { pos } => self.burst("sparkle", *pos - Vec2::new(0.0, 1.2), 8, 3.0, 0.3),
            SimEvent::EggLaid { pos } | SimEvent::EggHome { pos } => self.spawn("check", *pos - Vec2::new(0.0, 0.8), Vec2::new(0.0, -0.5), 0.8),
            SimEvent::Discovery { .. } => {}
            _ => {}
        }
    }

    /// The state of every decal slot of the town's scenery.
    pub fn decals(g: &Game, art: &WorldArt) -> Vec<u32> {
        art.slots.iter().map(|s| scenery::decal_state(g, s)).collect()
    }

    /// The flicker of candles and ripple of flags: about five steps a
    /// second.
    pub fn flicker(&self) -> u32 {
        (self.time * 5.0) as u32
    }

    pub fn sprites(&self, g: &Game, alpha: f32) -> Vec<Sprite> {
        let mut v = Vec::new();
        let lerp = |a: Vec2, b: Vec2| a.lerp(b, alpha);
        let mut put = |key: u32, sheet: SheetId, anim: String, frame: usize, pos: Vec2, layer: Layer, z_bias: i32| {
            v.push(Sprite {
                key,
                sheet,
                anim,
                frame,
                pos,
                layer,
                z_bias,
            });
        };
        let tick = (self.time * 6.0) as usize;
        // The raccoon and its bag.
        let r = &g.raccoon;
        let rp = lerp(r.prev, r.pos);
        let stride = (r.stride * if r.running { 1.4 } else { 2.2 }) as usize;
        let (anim, frame) = match r.act {
            Act::Fallen { .. } => ("fallen".to_string(), tick / 3),
            Act::Eating { .. } => ("eat".to_string(), tick / 2),
            Act::Patching { .. } => ("patch".to_string(), tick / 2),
            Act::Dancing { .. } => ("dance".to_string(), tick / 2),
            Act::Taking { .. } => ("take".to_string(), tick / 2),
            Act::AtDoor => ("knock".to_string(), if g.visit.as_ref().is_some_and(|v| v.phase == crate::sim::doors::Phase::Knocking) { tick } else { 0 }),
            Act::Free if r.swimming => ("swim".to_string(), tick / 3),
            Act::Free if r.vel.length() > 0.3 => (format!("{}_{}", if r.running { "run" } else { "walk" }, dir_name(r.facing)), stride),
            Act::Free => (format!("idle_{}", dir_name(r.facing)), tick / 6),
        };
        put(K_RACCOON, SheetId::Raccoon, anim, frame, rp, Layer::Cast, 4);
        if !r.swimming && r.bag.total() > 0 {
            let (off, bias) = match r.facing {
                Dir::E => (Vec2::new(-0.28, 0.05), -2),
                Dir::W => (Vec2::new(0.28, 0.05), -2),
                Dir::N => (Vec2::new(0.42, -0.05), -3),
                Dir::S => (Vec2::new(0.45, 0.06), 6),
            };
            put(K_BAG, SheetId::Bag, "bag".into(), bag_size(r.bag.total()) as usize, rp + off, Layer::Cast, bias);
        }
        // Loki and Skaði.
        for (i, k) in g.kids.iter().enumerate() {
            let p = lerp(k.prev, k.pos);
            let sheet = if i == 0 { SheetId::Loki } else { SheetId::Skadi };
            let moving = k.vel.length() > 0.3;
            let step = (k.stride * 2.4) as usize;
            let (anim, frame) = match k.state {
                KidState::Crying { .. } => ("cry".to_string(), tick / 2),
                KidState::Comfort { .. } => ("hug".to_string(), tick / 3),
                KidState::HoldHen { .. } | KidState::RescueHen { .. } if !moving => ("hold".to_string(), 0),
                KidState::Tug { .. } => {
                    let other = g.kids[1 - i].pos;
                    (format!("tug_{}", if other.x > k.pos.x { "right" } else { "left" }), tick / 2)
                }
                KidState::Jump { .. } => ("jump".to_string(), tick / 2),
                KidState::LeafDive { t, .. } if t > 0.0 => ("sprawl".to_string(), 0),
                KidState::Sulk { .. } => ("sit".to_string(), 0),
                _ if moving => (format!("walk_{}", dir_name(k.facing)), step),
                _ => (format!("stand_{}", dir_name(k.facing)), 0),
            };
            put(if i == 0 { K_LOKI } else { K_SKADI }, sheet, anim, frame, p, Layer::Cast, 2);
            let head = if i == 0 { 0.95 } else { 1.2 };
            if k.tricorn {
                put(K_HAT, SheetId::Stuff, "tricorn".into(), 0, p - Vec2::new(0.0, head), Layer::Cast, 30);
            }
            if k.key {
                put(K_KEY, SheetId::Stuff, "key".into(), 0, p - Vec2::new(-0.05, 0.55), Layer::Cast, 30);
            }
            if k.carrying_egg() {
                put(K_EGG, SheetId::Stuff, "egg".into(), 0, p - Vec2::new(0.0, 0.25) + k.facing.vec() * 0.2, Layer::Cast, 20);
            }
        }
        // Gullinkombi.
        {
            let h = &g.hen;
            let p = lerp(h.prev, h.pos);
            let d = dir_name(h.facing);
            let step = (h.stride * 4.0) as usize;
            let (anim, frame, lift) = if h.held_by.is_some() {
                ("held".to_string(), 0, 0.4)
            } else {
                match h.state {
                    hen::State::Peck { .. } | hen::State::Eat { .. } => (format!("peck_{}", if d == "up" { "down" } else { d }), tick / 2, 0.0),
                    hen::State::Lay { .. } | hen::State::Poop { .. } => ("fluff".to_string(), 0, 0.0),
                    hen::State::Sit { .. } => (format!("idle_{d}"), tick / 8, 0.0),
                    hen::State::Flee { .. } => ("startled".to_string(), 0, 0.0),
                    _ if h.vel.length() > 0.2 => (format!("walk_{d}"), step, 0.0),
                    _ => (format!("idle_{d}"), tick / 8, 0.0),
                }
            };
            put(K_HEN, SheetId::Hen, anim, frame, p - Vec2::new(0.0, lift), Layer::Cast, if h.held_by.is_some() { 25 } else { 1 });
        }
        // Gravy.
        {
            let d = &g.gravy;
            let p = lerp(d.prev, d.pos);
            let s = side(d.facing);
            let (anim, frame) = match d.state {
                gravy::State::Lie => (format!("lie_{s}"), tick / 8),
                gravy::State::Rubbed { .. } => (format!("rubbed_{s}"), tick / 2),
                gravy::State::Eat { .. } => (format!("eat_{s}"), tick / 2),
                gravy::State::Barf { t } => (format!("barf_{s}"), usize::from(t < 0.5)),
                _ => (format!("walk_{s}"), (d.stride * 1.6) as usize),
            };
            put(K_GRAVY, SheetId::Gravy, anim, frame, p, Layer::Body, 0);
        }
        // The black cat (not while she is up a tree).
        if !g.cat.hidden() {
            let c = &g.cat;
            let p = lerp(c.prev, c.pos);
            let s = side(c.facing);
            let (anim, frame) = match c.state {
                cat::State::Sit { .. } => (format!("cat_sit_{s}"), tick / 10),
                cat::State::Hiss { .. } => (format!("cat_hiss_{s}"), 0),
                _ => (format!("cat_walk_{s}"), (c.stride * 3.0) as usize),
            };
            put(K_CAT, SheetId::Pets, anim, frame, p, Layer::Body, 0);
        }
        // Their mother on the porch, and Ethan Allen on his tower.
        let town = &g.town;
        let visiting = g.visit.as_ref().filter(|v| v.open()).map(|v| v.b);
        {
            let home = &town.buildings[town.home];
            let near = g.raccoon.pos.distance(home.door.step()) < 5.0 || g.kids.iter().any(|k| k.pos.distance(home.door.step()) < 4.0);
            let (anim, frame) = if visiting == Some(town.home) {
                ("laugh", 0)
            } else if near {
                ("offer", tick / 4)
            } else {
                ("stand", 0)
            };
            put(K_MOM, SheetId::Mom, anim.into(), frame, home.door.at + home.door.facing.vec() * 0.45, Layer::Body, 0);
        }
        if let Some(tower) = town.buildings.iter().find(|b| b.kind == BuildingKind::Tower) {
            let shouting = visiting == Some(tower.id);
            put(
                K_ETHAN,
                SheetId::Ethan,
                if shouting { "shout" } else { "stand" }.into(),
                if shouting { tick / 2 } else { 0 },
                tower.rect.center() + Vec2::new(0.0, 0.6),
                Layer::Over,
                0,
            );
        }
        // Whoever answers a door.
        if let Some(b) = visiting {
            let bd = &town.buildings[b];
            if !matches!(bd.kind, BuildingKind::Home | BuildingKind::Tower) {
                let look = people::ADULTS.start + (bd.seed % people::ADULTS.len() as u64) as usize;
                put(K_DOOR, SheetId::Folk(look as u8), "walk_down".into(), 1, bd.door.at + Vec2::new(0.0, 0.1), Layer::Body, 0);
            }
        }
        // Everybody else.
        for w in &g.folk {
            let p = lerp(w.prev, w.pos);
            let moving = w.vel.length() > 0.2;
            if w.kind == Kind::Dog {
                let s = side(w.facing);
                let (anim, frame) = if matches!(w.goal, folk::Goal::Chase { .. }) && !moving {
                    (format!("dog_bark_{s}"), 0)
                } else {
                    (format!("dog_walk_{s}"), (w.stride * 3.0) as usize)
                };
                put(K_FOLK + w.id, SheetId::Pets, anim, frame, p, Layer::Body, 0);
                continue;
            }
            let look = look_of(w);
            let frame = if moving { [0usize, 1, 2, 1][((w.stride * 2.4) as usize) % 4] } else { 1 };
            put(K_FOLK + w.id, SheetId::Folk(look as u8), format!("walk_{}", dir_name(w.facing)), frame, p, Layer::Body, 0);
        }
        // Things on the ground.
        for it in &g.items {
            if it.carried.is_some() {
                continue;
            }
            let anim = match it.kind {
                ItemKind::Candy(c) => format!("candy_{}", c as usize),
                ItemKind::Mat(m) => format!("mat_{}", m as usize),
                ItemKind::Egg => "egg".into(),
                ItemKind::Poop => "poop".into(),
                ItemKind::Barf => "barf".into(),
            };
            let layer = if it.z > 0.05 { Layer::Body } else { Layer::Decal };
            put(K_ITEMS + it.id % 900_000_000, SheetId::Stuff, anim, 0, it.pos - Vec2::new(0.0, it.z), layer, 0);
        }
        for e in &self.effects {
            let frames = 4;
            let f = ((e.age / e.life) * frames as f32) as usize;
            put(e.key, SheetId::Fx, e.anim.into(), f, e.pos, Layer::Over, 0);
        }
        v
    }

    /// Speech bubbles over whoever is talking.
    pub fn bubbles(g: &Game, alpha: f32) -> Vec<Bubble> {
        g.speech
            .iter()
            .filter_map(|s| {
                let (pos, head, colour) = match s.who {
                    Who::Raccoon => (g.raccoon.prev.lerp(g.raccoon.pos, alpha), 1.45, (230, 230, 240)),
                    Who::Loki => (g.kids[0].prev.lerp(g.kids[0].pos, alpha), 1.25, (150, 230, 110)),
                    Who::Skadi => (g.kids[1].prev.lerp(g.kids[1].pos, alpha), 1.55, (130, 190, 255)),
                    Who::Hen => (g.hen.pos, 0.9, (255, 240, 200)),
                    Who::Gravy => (g.gravy.pos, 1.3, (220, 170, 120)),
                    Who::Cat => (g.cat.pos, 0.8, (210, 200, 255)),
                    Who::Door(b) => {
                        let bd = g.town.buildings.get(b as usize)?;
                        let colour = match bd.kind {
                            BuildingKind::Home => (255, 160, 110),
                            BuildingKind::Tower => (140, 220, 140),
                            _ => (255, 214, 140),
                        };
                        let at = if bd.kind == BuildingKind::Tower { bd.rect.center() - Vec2::new(0.0, 1.4) } else { bd.door.at };
                        (at, 1.5, colour)
                    }
                    Who::Folk(id) => {
                        let w = g.folk.iter().find(|w| w.id == id)?;
                        let head = if matches!(w.kind, Kind::Kid | Kind::Dog) { 1.1 } else { 1.7 };
                        (w.prev.lerp(w.pos, alpha), head, if w.kind == Kind::Kid { (255, 190, 230) } else { (220, 220, 200) })
                    }
                };
                Some(Bubble {
                    at: pos - Vec2::new(0.0, head),
                    text: s.text.clone(),
                    colour,
                })
            })
            .collect()
    }
}

/// Which look in the folk sheet a walker wears.
pub fn look_of(w: &folk::Walker) -> usize {
    match w.kind {
        Kind::Kid => {
            let c = folk::Costume::ALL.iter().position(|c| Some(*c) == w.costume).unwrap_or(0);
            c + 10 * (w.look as usize % 2)
        }
        Kind::Teen => people::LOOK_TEEN[w.look as usize % 2],
        Kind::Jogger => people::LOOK_JOGGER,
        Kind::Tourist => people::LOOK_TOURIST[w.look as usize % 2],
        Kind::Professor => people::LOOK_PROFESSOR,
        Kind::Legislator => people::LOOK_LEGISLATOR,
        Kind::Farmer => people::LOOK_FARMER,
        Kind::DogWalker => people::LOOK_DOGWALKER,
        _ => people::ADULTS.start + (w.look as usize % people::ADULTS.len()),
    }
}
