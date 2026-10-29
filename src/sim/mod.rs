//! The deterministic simulation of Halloween night in Montpelier: the
//! raccoon, its costume, bag, belly and happiness; the doors it knocks on;
//! Loki, Skaði and their mother; Gullinkombi the hen; the people out
//! walking and the rival trick-or-treaters; and everything lying about.
//!
//! [`Game::step`] advances it by one tick of 1/60 s. Everything random comes
//! from one ChaCha8 generator, so the same seed and inputs make the same
//! night.

pub mod autopilot;
pub mod candy;
pub mod cat;
pub mod costume;
pub mod discover;
pub mod doors;
pub mod folk;
pub mod give;
pub mod gravy;
pub mod hen;
pub mod items;
pub mod kids;
pub mod lines;
pub mod raccoon;

#[cfg(test)]
mod tests;

use crate::world::plan::Town;
use crate::world::Dir;
use candy::Candy;
use discover::Discoveries;
use doors::{House, Visit};
use folk::Walker;
use glam::Vec2;
use hen::Hen;
use items::Item;
use kids::Kid;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use raccoon::Raccoon;
use std::sync::Arc;

pub const DT: f32 = 1.0 / 60.0;

/// Game seconds per minute of the evening's clock.
pub const SECONDS_PER_MINUTE: f32 = 4.0;
/// Trick-or-treating runs from half past five to half past nine.
pub const EVENING_START: f32 = 17.5 * 60.0;
pub const EVENING_END: f32 = 21.5 * 60.0;

/// Somebody (or something) in the town.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub enum Who {
    Raccoon,
    Loki,
    Skadi,
    Hen,
    /// The big corded dog at State and Main.
    Gravy,
    /// The black cat on State Street.
    Cat,
    /// Somebody at a door (the building's index).
    Door(u32),
    /// Somebody out walking (the walker's id).
    Folk(u32),
}

/// What the player asks for this tick.
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TickInput {
    /// The way the arrow keys point (zero when none are held).
    pub dir: Vec2,
    pub run: bool,
    /// Knock, take, or (with nothing else to do) patch.
    pub act: bool,
    /// Eat the best thing in the bag.
    pub eat: bool,
    /// Eat this, picked from the bag.
    pub eat_this: Option<Candy>,
    /// Give this to whoever is in reach.
    pub give: Option<Candy>,
    pub patch: bool,
    /// Walk to this point (a click or a tap).
    pub goto: Option<Vec2>,
}

/// Something that happened this tick, for the presentation (effects,
/// haptics, the message line) to react to.
#[derive(Clone, Debug, PartialEq)]
pub enum SimEvent {
    Knock { b: usize },
    DoorOpen { b: usize },
    DoorClose { b: usize },
    Given { b: usize, candy: Vec<Candy>, pos: Vec2 },
    /// The raccoon gave somebody a present.
    Gift { pos: Vec2 },
    Refused { b: usize, pos: Vec2 },
    Scream { pos: Vec2 },
    LightOut { b: usize },
    LightOn { b: usize },
    CandyDropped { pos: Vec2, n: u32, by: Who },
    Picked { pos: Vec2, what: String },
    Fall { who: Who, pos: Vec2 },
    Slip { pos: Vec2 },
    Tear { pos: Vec2 },
    Patched { pos: Vec2, mat: candy::Mat },
    Ate { candy: Candy, pos: Vec2 },
    Loved { pos: Vec2 },
    Splash { pos: Vec2 },
    Leaves { pos: Vec2 },
    Berries { pos: Vec2 },
    EggLaid { pos: Vec2 },
    EggBroken { pos: Vec2 },
    EggHome { pos: Vec2 },
    Poop { pos: Vec2 },
    Barf { pos: Vec2 },
    Cry { pos: Vec2 },
    Hug { pos: Vec2 },
    Tug { pos: Vec2 },
    TugWon { who: Who, pos: Vec2 },
    Jump { who: Who, pos: Vec2 },
    Squawk { pos: Vec2 },
    Bark { pos: Vec2 },
    Flash { pos: Vec2 },
    Flee { pos: Vec2 },
    Snatch { pos: Vec2 },
    Discovery { id: discover::Id },
    NewNight { night: u32 },
    Message { text: String },
}

/// A speech bubble.
#[derive(Clone, Debug, PartialEq)]
pub struct Speech {
    pub who: Who,
    pub text: String,
    pub ttl: f32,
    /// Where to show it when the speaker is a door.
    pub at: Vec2,
}

/// The state of a feature that can change: berries picked, a leaf pile
/// scattered, a pumpkin knocked over, the flyers taken, a bowl emptied.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FeatState {
    pub used: bool,
    /// Seconds until it is back to how it was.
    pub regrow: f32,
    /// What is left: flyers on the pole, candy in the bowl, eggs in the
    /// basket, streamers in the tree.
    pub n: u32,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Stats {
    pub last_tick: std::time::Duration,
    pub doors_knocked: u64,
    pub refusals: u64,
    pub eaten: u64,
    pub patched: u64,
    pub distance: f64,
}

#[derive(Clone)]
pub struct Game {
    pub town: Arc<Town>,
    pub rng: ChaCha8Rng,
    pub tick: u64,
    /// Seconds played.
    pub time: f32,
    /// The evening clock in minutes past midnight.
    pub clock: f32,
    /// Which Halloween this is (the night starts over at half past nine).
    pub night: u32,
    pub raccoon: Raccoon,
    pub kids: [Kid; 2],
    pub hen: Hen,
    pub gravy: gravy::Gravy,
    pub cat: cat::Cat,
    pub folk: Vec<Walker>,
    pub items: Vec<Item>,
    pub next_item: u32,
    /// One per building (those nobody answers have a house that never
    /// opens).
    pub houses: Vec<House>,
    pub feats: Vec<FeatState>,
    pub visit: Option<Visit>,
    pub speech: Vec<Speech>,
    pub events: Vec<SimEvent>,
    pub found: Discoveries,
    pub stats: Stats,
    /// The message line: the latest thing worth saying.
    pub message: Option<(String, f32)>,
}

impl Game {
    pub fn new(town: Arc<Town>, seed: u64) -> Game {
        let mut rng = ChaCha8Rng::seed_from_u64(seed);
        let houses = doors::houses(&town, &mut rng);
        let feats = items::initial_feats(&town);
        let raccoon = Raccoon::new(town.start);
        let kids = kids::start(&town, &mut rng);
        let hen = Hen::new(&town, &mut rng);
        let gravy = gravy::Gravy::new(&town);
        let cat = cat::Cat::new(&town);
        let folk = folk::populate(&town, &mut rng);
        let mut g = Game {
            town,
            rng,
            tick: 0,
            time: 0.0,
            clock: EVENING_START,
            night: 1,
            raccoon,
            kids,
            hen,
            gravy,
            cat,
            folk,
            items: Vec::new(),
            next_item: 1,
            houses,
            feats,
            visit: None,
            speech: Vec::new(),
            events: Vec::new(),
            found: Discoveries::default(),
            stats: Stats::default(),
            message: None,
        };
        items::scatter_litter(&mut g, true);
        g
    }

    pub fn step(&mut self, input: &TickInput) {
        let started = std::time::Instant::now();
        self.tick += 1;
        self.time += DT;
        self.advance_clock();
        raccoon::step(self, input);
        doors::step(self);
        kids::step(self);
        hen::step(self);
        gravy::step(self);
        cat::step(self);
        folk::step(self);
        items::step(self);
        self.speech.retain_mut(|s| {
            s.ttl -= DT;
            s.ttl > 0.0
        });
        if let Some((_, ttl)) = &mut self.message {
            *ttl -= DT;
            if *ttl <= 0.0 {
                self.message = None;
            }
        }
        self.stats.last_tick = started.elapsed();
    }

    fn advance_clock(&mut self) {
        self.clock += DT / SECONDS_PER_MINUTE;
        if self.clock >= EVENING_END {
            // Another Halloween: bowls refilled, pumpkins relit, leaves
            // re-raked. The raccoon keeps its haul.
            self.clock = EVENING_START;
            self.night += 1;
            for h in &mut self.houses {
                h.new_night();
            }
            for f in &mut self.feats {
                f.used = false;
                f.regrow = 0.0;
            }
            items::reset_feats(self);
            self.events.push(SimEvent::NewNight { night: self.night });
            self.say_message(format!("Halloween comes round again (night {}). The porch lights come back on.", self.night));
        }
    }

    pub fn drain_events(&mut self) -> Vec<SimEvent> {
        std::mem::take(&mut self.events)
    }

    /// The evening clock as "7:42 pm".
    pub fn clock_text(&self) -> String {
        let m = self.clock as u32;
        let (h, min) = (m / 60, m % 60);
        let h12 = if h > 12 { h - 12 } else { h };
        format!("{h12}:{min:02} pm")
    }

    pub fn say(&mut self, who: Who, text: impl Into<String>) {
        let text = text.into();
        let ttl = 1.6 + text.chars().count() as f32 * 0.045;
        let at = match who {
            Who::Door(b) => self.town.buildings.get(b as usize).map_or(Vec2::ZERO, |b| b.door.at),
            _ => Vec2::ZERO,
        };
        // A speaker says one thing at a time.
        self.speech.retain(|s| s.who != who);
        self.speech.push(Speech { who, text, ttl, at });
    }

    pub fn say_message(&mut self, text: impl Into<String>) {
        let text = text.into();
        self.events.push(SimEvent::Message { text: text.clone() });
        self.message = Some((text, 5.0));
    }

    pub fn discover(&mut self, id: discover::Id) {
        if self.found.mark(id) {
            self.events.push(SimEvent::Discovery { id });
        }
    }

    /// Where somebody is, for speech bubbles and distances.
    pub fn pos_of(&self, who: Who) -> Option<Vec2> {
        match who {
            Who::Raccoon => Some(self.raccoon.pos),
            Who::Loki => Some(self.kids[0].pos),
            Who::Skadi => Some(self.kids[1].pos),
            Who::Hen => Some(self.hen.pos),
            Who::Gravy => Some(self.gravy.pos),
            Who::Cat => Some(self.cat.pos),
            Who::Door(b) => self.town.buildings.get(b as usize).map(|b| b.door.at),
            Who::Folk(id) => self.folk.iter().find(|w| w.id == id).map(|w| w.pos),
        }
    }

    /// A rough hash of the state, for checking that runs agree.
    pub fn state_hash(&self) -> u64 {
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        let mut mix = |v: u64| {
            h ^= v;
            h = h.wrapping_mul(0x100_0000_01b3);
        };
        mix(self.tick);
        mix(self.raccoon.pos.x.to_bits() as u64);
        mix(self.raccoon.pos.y.to_bits() as u64);
        mix(self.raccoon.bag.total());
        mix(self.raccoon.happy.to_bits() as u64);
        for k in &self.kids {
            mix(k.pos.x.to_bits() as u64 ^ (k.pos.y.to_bits() as u64) << 32);
        }
        mix(self.hen.pos.x.to_bits() as u64 ^ (self.hen.pos.y.to_bits() as u64) << 32);
        mix(self.gravy.pos.x.to_bits() as u64 ^ (self.gravy.pos.y.to_bits() as u64) << 32);
        for w in &self.folk {
            mix(w.pos.x.to_bits() as u64 ^ (w.pos.y.to_bits() as u64) << 32);
        }
        mix(self.items.len() as u64);
        h
    }
}

/// Moves a round body of radius `r` by `d`, sliding along walls. Returns
/// whether it was stopped (fully or partly).
pub fn move_body(town: &Town, pos: &mut Vec2, d: Vec2, r: f32, swim: bool) -> bool {
    let len = d.length();
    if len <= 0.0 {
        return false;
    }
    let steps = (len / 0.2).ceil().max(1.0) as u32;
    let sd = d / steps as f32;
    let mut blocked = false;
    for _ in 0..steps {
        let full = *pos + sd;
        if town.clear(full, r, swim) {
            *pos = full;
            continue;
        }
        blocked = true;
        let xo = *pos + Vec2::new(sd.x, 0.0);
        let yo = *pos + Vec2::new(0.0, sd.y);
        if sd.x != 0.0 && town.clear(xo, r, swim) {
            *pos = xo;
        } else if sd.y != 0.0 && town.clear(yo, r, swim) {
            *pos = yo;
        } else {
            // Head on into something round (a trunk, a pole, a cow): slide
            // along it, the way that keeps going forward.
            let mut moved = false;
            let round = town
                .features_near(*pos, r + 1.0)
                .filter(|f| f.kind.solid_radius() > 0.0)
                .min_by(|a, b| a.pos.distance(*pos).total_cmp(&b.pos.distance(*pos)))
                .map(|f| f.pos);
            if let Some(c) = round {
                let n = (*pos - c).normalize_or_zero();
                // Always round the same way for the same approach, so that
                // a walker heading straight at a trunk circles it instead
                // of dithering from side to side.
                let cross = n.x * sd.y - n.y * sd.x;
                let t = if cross >= 0.0 { Vec2::new(-n.y, n.x) } else { Vec2::new(n.y, -n.x) };
                let slide = t * sd.length() + n * 0.01;
                if town.clear(*pos + slide, r, swim) {
                    *pos += slide;
                    continue;
                }
            }
            for deg in [35.0f32, -35.0, 70.0, -70.0, 90.0, -90.0] {
                let (s, c) = deg.to_radians().sin_cos();
                let side = Vec2::new(sd.x * c - sd.y * s, sd.x * s + sd.y * c);
                if town.clear(*pos + side, r, swim) {
                    *pos += side;
                    moved = true;
                    break;
                }
            }
            if !moved {
                break;
            }
        }
    }
    blocked
}

/// Facing from a velocity, keeping the old one when nearly still.
pub fn facing_of(v: Vec2, old: Dir) -> Dir {
    if v.length_squared() < 0.04 { old } else { Dir::of(v) }
}

/// Steers velocity `v` towards `want` with at most `accel` change per
/// second.
pub fn steer(v: Vec2, want: Vec2, accel: f32) -> Vec2 {
    let dv = want - v;
    let max = accel * DT;
    if dv.length() <= max { want } else { v + dv.normalize() * max }
}
