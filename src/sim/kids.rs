//! Loki and Skaði, who think the raccoon is another kid and tag along.
//!
//! Loki and Skaði think the raccoon is another kid, and go trick-or-treating
//! with it.
//!
//! Skaði (fourteen, cat ears) watches where the raccoon is heading, guesses
//! which door, and gets there first. Loki (six, in a dinosaur hoodie) watches
//! his sister, guesses a beat later, and runs. Both keep to the sidewalks and
//! stop at the curb before they cross. Shake them off by changing course and
//! they notice, and work out the new door quickly; worth trying, because
//! with them along every child takes one, from the same bowl.
//!
//! Between doors Loki is random: he dashes about, crashes into people, dives
//! into leaf piles, moves things, pulls tails, and sprints the hen's eggs
//! home and back, on his own (he is six: he knows the way). Skaði means
//! what she does: she comforts her brother, hands back what the raccoon
//! dropped, patches its sheet, gets the hen out of the road, and has opinions
//! about taking more than one.
//!
//! Loki eats out of his pail as he goes, and the sugar shows: the higher his
//! blood sugar, the more he dashes about and the faster he goes, the more
//! often he crashes into somebody (his sister, the hen, the townsfolk, the
//! raccoon), and the wider and sillier the loop he runs before he comes
//! back for the candy he knocked loose. It wears off.
//!
//! Children wade where they have to (cartoon physics, as in Silkie Sprint),
//! though they would much rather use the bridge.

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

/// Looking both ways at the curb, in seconds.
pub const CROSS_PAUSE: f32 = 0.7;
/// What crossing a road costs a child, against a sidewalk's one: they go
/// round by the sidewalk unless that is a long way round.
const ROAD_COST: u32 = 8;
/// Children this far from the raccoon (stuck, or left on the far side of
/// town) for this long catch up out of sight.
const LOST_AT: f32 = 40.0;
const LOST_FOR: f32 = 15.0;

/// How much of Loki's blood sugar (0 calm, 1 bouncing off the walls) wears
/// off a second: a full rush takes about five minutes to wear off. What
/// each piece puts in is [`sugar_of`].
const SUGAR_FADE: f32 = 0.0033;
/// Loki on a sugar rush; past `SUGAR_CUTOFF` Skaði stops his snacking, if
/// she is there to see it. At `SUGAR_CRASH` he crashes: he stops, cries
/// without knowing why, and comes down to `SUGAR_AFTER`.
pub const SUGAR_RUSH: f32 = 0.65;
const SUGAR_CUTOFF: f32 = 0.85;
pub const SUGAR_CRASH: f32 = 1.0;
const SUGAR_AFTER: f32 = 0.1;

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
    /// Loki: his now. Finders keepers.
    Keep,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum State {
    Follow,
    Wander { to: Vec2, t: f32 },
    /// Running flat out (Loki, mostly): this is how bumps happen.
    Dash { to: Vec2, t: f32 },
    FetchEgg { item: u32 },
    /// Loki taking the eggs in his pail home, on his own, at a sprint.
    CarryEgg,
    /// Off to a door to be there first: Skaði guessing where the raccoon is
    /// going, Loki guessing where Skaði is.
    GoDoor { b: usize },
    /// "Huh? Oh, that one!" A moment, then off to the door it really is.
    Realize { t: f32, b: usize },
    /// Loki, having knocked candy out of somebody, running round in a loop
    /// (of radius `r`, wider the more sugar he has had) before coming back
    /// for it.
    Circle { t: f32, centre: Vec2, a: f32, r: f32 },
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
    /// Loki, his teeth stuck together with caramel: going nowhere.
    Chew { t: f32 },
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
    /// Eggs in Loki's pail, on their way home.
    pub eggs: u32,
    /// Seconds spent far from the raccoon.
    pub lost: f32,
    pub call_cd: f32,
    /// Waiting at the curb, and then on the way across.
    pub cross_wait: f32,
    pub crossing: bool,
    /// Seconds of trying to move and getting nowhere from `stuck_at`.
    pub stuck: f32,
    pub stuck_at: Vec2,
    /// Blood sugar (Loki's is the one that matters): 0 calm to 1.
    pub sugar: f32,
    /// Seconds until the next piece out of the pail.
    pub snack_t: f32,
    /// Seconds more Skaði looks the other way, bribed with something good.
    pub bribed: f32,
    /// Seconds more Loki is busy with a lollipop, and too content to dash
    /// about.
    pub licking: f32,
    /// Seconds Loki is allowed for the run back from taking an egg home
    /// (until he is back with his sister).
    pub back_t: f32,
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
            eggs: 0,
            lost: 0.0,
            call_cd: 0.0,
            cross_wait: 0.0,
            crossing: false,
            stuck: 0.0,
            stuck_at: at,
            sugar: 0.15,
            snack_t: 20.0,
            bribed: 0.0,
            licking: 0.0,
            back_t: 0.0,
        }
    }

    pub fn holding_hen(&self) -> bool {
        matches!(self.state, State::HoldHen { .. } | State::Tug { .. } | State::RescueHen { .. })
    }

    pub fn carrying_egg(&self) -> bool {
        self.eggs > 0
    }

    /// Off taking an egg home, or on the way back from it.
    pub fn on_errand(&self) -> bool {
        self.state == State::CarryEgg || self.back_t > 0.0
    }

    fn walk_speed(&self) -> f32 {
        if self.who == Who::Loki { 3.2 } else { 3.4 }
    }

    /// Running (or skipping) to catch up: quick, but slower than a raccoon
    /// flat out.
    fn run_speed(&self) -> f32 {
        if self.who == Who::Loki { 6.0 + 1.0 * self.sugar } else { 6.2 }
    }

    pub fn sugar_rush(&self) -> bool {
        self.sugar >= SUGAR_RUSH
    }
}

pub fn start(town: &Town, _rng: &mut ChaCha8Rng) -> [Kid; 2] {
    let s = town.start;
    let at = |o: Vec2| town.open_near(s + o, RADIUS, false, 12.0).unwrap_or(s);
    [Kid::new(Who::Loki, at(Vec2::new(-1.6, 1.2))), Kid::new(Who::Skadi, at(Vec2::new(1.4, 1.3)))]
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

/// What a piece does to Loki's blood sugar. Chocolate and syrup set him off;
/// fruit barely registers; cheddar calms him down; hákarl is fish.
pub fn sugar_of(c: Candy) -> f32 {
    match c {
        Candy::FullSize => 0.4,
        Candy::SyrupJug => 0.35,
        Candy::MapleCotton => 0.25,
        Candy::Chocolate | Candy::CiderDonut => 0.2,
        Candy::PeanutCup => 0.18,
        Candy::Caramel | Candy::MapleLeaf => 0.16,
        Candy::GummyWorms => 0.15,
        Candy::CandyCorn | Candy::Lollipop => 0.12,
        Candy::PopcornBall => 0.08,
        Candy::Carob => 0.04,
        Candy::Raspberry | Candy::Raisins => 0.03,
        Candy::Apple => 0.02,
        Candy::Cheddar => -0.25,
        Candy::Hakarl | Candy::Toothbrush | Candy::Pennies => 0.0,
    }
}

/// Loki eats a piece of candy.
pub fn loki_eats(g: &mut Game, c: Candy) {
    let was = g.kids[LOKI].sugar_rush();
    let k = &mut g.kids[LOKI];
    k.sugar = (k.sugar + sugar_of(c)).clamp(0.0, SUGAR_CRASH);
    match c {
        // Teeth stuck together for a while.
        Candy::Caramel => k.state = State::Chew { t: 5.0 },
        Candy::Lollipop => k.licking = 25.0,
        _ => {}
    }
    if k.sugar >= SUGAR_CRASH {
        sugar_crash(g);
    } else if !was && k.sugar_rush() {
        let line = lines::pick(&mut g.rng, lines::LOKI_SUGAR);
        g.say(Who::Loki, line);
        g.discover(Id::SugarRush);
        g.say_message("Loki is on a sugar rush.");
    }
}

/// Too much: Loki stops dead and cries, and has no idea why. Skaði knows.
fn sugar_crash(g: &mut Game) {
    let pos = g.kids[LOKI].pos;
    if g.kids[LOKI].holding_hen() {
        release_hen(g, pos);
    }
    let k = &mut g.kids[LOKI];
    k.sugar = SUGAR_AFTER;
    k.state = State::Crying { t: 9.0 };
    k.path.clear();
    let line = lines::pick(&mut g.rng, lines::LOKI_CRASH);
    g.say(Who::Loki, line);
    g.events.push(SimEvent::Cry { pos });
    g.discover(Id::SugarCrash);
    g.say_message("Sugar crash. Loki is crying and he doesn't know why.");
    let line = lines::pick(&mut g.rng, lines::SKADI_CRASH);
    skadi_comes_running(g, line);
}

/// Loki snacks out of his pail as he goes, until his sister says enough.
fn snack(g: &mut Game) {
    let k = &mut g.kids[LOKI];
    k.sugar = (k.sugar - SUGAR_FADE * DT).max(0.0);
    k.snack_t -= DT;
    if k.snack_t > 0.0 {
        return;
    }
    // The fuller the pail, the oftener a hand goes in.
    k.snack_t = g.rng.random_range(22.0..45.0) / (1.0 + k.pail as f32 / 20.0);
    if k.pail == 0 || matches!(k.state, State::Crying { .. } | State::Jump { .. }) {
        return;
    }
    // Over the limit, his sister stops him, if she is close enough to see.
    if k.sugar > SUGAR_CUTOFF && g.kids[SKADI].pos.distance(g.kids[LOKI].pos) < 8.0 {
        let line = lines::pick(&mut g.rng, lines::SKADI_ENOUGH);
        g.say(Who::Skadi, line);
        return;
    }
    g.kids[LOKI].pail -= 1;
    let c = super::doors::random_treat(&mut g.rng);
    loki_eats(g, c);
}

/// Skaði sees the raccoon empty a "please take one" bowl.
pub fn skadi_objects_to_greed(g: &mut Game, feature: usize, took: u32) {
    let k = &g.kids[SKADI];
    if k.pos.distance(g.raccoon.pos) > 9.0 || took < 2 {
        return;
    }
    if k.bribed > 0.0 {
        if k.chat_cd <= 0.0 {
            g.say(Who::Skadi, "I didn't see anything.");
            g.kids[SKADI].chat_cd = 5.0;
            g.discover(Id::SkadiBribed);
        }
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
        k.bribed = (k.bribed - DT).max(0.0);
        k.licking = (k.licking - DT).max(0.0);
        k.back_t = (k.back_t - DT).max(0.0);
        k.mom_cd = (k.mom_cd - DT).max(0.0);
        k.chat_cd = (k.chat_cd - DT).max(0.0);
        k.path_age += DT;
        k.call_cd = (k.call_cd - DT).max(0.0);
        k.think -= DT;
        if k.think <= 0.0 {
            k.think = if i == LOKI { g.rng.random_range(0.6..1.6) * (1.0 - 0.5 * k.sugar) } else { 0.5 };
            decide(g, i);
        }
        act(g, i);
        chatter(g, i);
    }
    if g.kids[LOKI].pos.distance(g.kids[SKADI].pos) < 6.0 {
        g.kids[LOKI].back_t = 0.0;
    }
    snack(g);
    contacts(g);
    loki_crashes(g);
    mom(g);
    regroup(g);
}

/// Children who have been far from the raccoon for a while (stuck behind
/// something, or left on the other side of town) turn up again just out
/// of sight behind it, and their hen with them.
fn regroup(g: &mut Game) {
    let r = g.raccoon.pos;
    for i in 0..2 {
        // Loki on an egg run is not lost: he knows where he is going.
        let errand = g.kids[i].on_errand();
        let far = g.kids[i].pos.distance(r) > LOST_AT;
        g.kids[i].lost = if far && !errand { g.kids[i].lost + DT } else { 0.0 };
    }
    if g.kids.iter().all(|k| k.lost < LOST_FOR) {
        return;
    }
    let town = g.town.clone();
    let back = if g.raccoon.vel.length() > 0.1 { -g.raccoon.vel.normalize() } else { -g.raccoon.facing.vec() };
    let behind = r + back * 13.0;
    let Some(spot) = town.open_near(behind, RADIUS, false, 8.0).or_else(|| town.open_near(r, RADIUS, false, 6.0)) else { return };
    for (i, o) in [(SKADI, Vec2::ZERO), (LOKI, Vec2::new(0.7, 0.3))] {
        let at = town.open_near(spot + o, RADIUS, false, 3.0).unwrap_or(spot);
        let k = &mut g.kids[i];
        k.pos = at;
        k.prev = at;
        k.vel = Vec2::ZERO;
        k.path.clear();
        k.lost = 0.0;
        if !matches!(k.state, State::Jump { .. }) {
            k.state = State::Follow;
        }
    }
    if g.hen.held_by.is_none() && g.hen.pos.distance(spot) > 12.0 {
        let at = town.open_near(spot + Vec2::new(-0.8, 0.4), super::hen::RADIUS, false, 3.0).unwrap_or(spot);
        g.hen.pos = at;
        g.hen.prev = at;
        g.hen.path.clear();
    }
}

/// The door the raccoon seems to be making for: the one it is knocking at,
/// the one at the end of the way it was told to walk, or the lit porch it is
/// heading towards (nearer and more straight ahead is likelier; a door it has
/// already tried, less likely).
pub fn predict_door(g: &Game) -> Option<usize> {
    if let Some(v) = &g.visit {
        return Some(v.b);
    }
    let town = &g.town;
    let r = &g.raccoon;
    if let Some(end) = r.path.last() {
        if let Some(b) = town.door_near(*end, 1.6) {
            return Some(b.id);
        }
    }
    if let Some(b) = town.door_near(r.pos, 1.6) {
        if g.houses[b.id].answers() {
            return Some(b.id);
        }
    }
    if r.vel.length() < 0.6 {
        return None;
    }
    let dir = r.vel.normalize();
    town.buildings
        .iter()
        .filter(|b| g.houses[b.id].light && g.houses[b.id].answers())
        .filter_map(|b| {
            let v = b.door.step() - r.pos;
            let d = v.length();
            if d > 14.0 || d < 0.01 {
                return None;
            }
            let ahead = dir.dot(v / d);
            if ahead < 0.55 {
                return None;
            }
            let tried = if g.houses[b.id].visits > 0 { 0.6 } else { 0.0 };
            Some((b.id, ahead * 2.0 - d * 0.12 - tried))
        })
        .max_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(b, _)| b)
}

/// Where a child waits at a door: beside the step, Skaði on one side and
/// Loki on the other, so the raccoon has room to knock.
fn door_spot(g: &Game, b: usize, i: usize) -> Vec2 {
    let d = g.town.buildings[b].door;
    let side = Vec2::new(-d.facing.vec().y, d.facing.vec().x);
    d.step() + side * if i == SKADI { 1.0 } else { -1.0 } + d.facing.vec() * 0.4
}

fn decide(g: &mut Game, i: usize) {
    let me = g.kids[i].clone();
    let interruptible = matches!(me.state, State::Follow | State::Wander { .. } | State::GoDoor { .. });
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
        // The hen sitting in the road.
        if g.hen.held_by.is_none()
            && g.town.grid.at(g.hen.pos) == Terrain::Road
            && matches!(g.hen.state, super::hen::State::Sit { .. } | super::hen::State::Peck { .. })
            && g.hen.pos.distance(me.pos) < 8.0
        {
            g.kids[i].state = State::ChaseHen { t: 8.0 };
            return;
        }
        // Which door next? Get there first.
        let guess = predict_door(g);
        match (me.state, guess) {
            (State::GoDoor { b }, Some(nb)) if nb != b => {
                // Fooled! A moment to notice, quicker if it is already
                // knocking somewhere else.
                let t = if g.visit.is_some() { 0.35 } else { g.rng.random_range(0.5..1.1) };
                g.kids[i].state = State::Realize { t, b: nb };
                if g.kids[i].call_cd <= 0.0 {
                    g.kids[i].call_cd = 6.0;
                    let line = lines::pick(&mut g.rng, &["Huh? Oh, that one!", "Wait, where's it going?", "Not that house? Okay!", "Hey, wait for us!"]);
                    g.say(Who::Skadi, line);
                }
            }
            (State::GoDoor { b }, None) => {
                // Still making for it while the raccoon is close by; lost
                // interest once it has wandered off.
                if g.town.buildings[b].door.step().distance(rpos) > 7.0 && g.visit.is_none() {
                    g.kids[i].state = State::Follow;
                }
            }
            (State::GoDoor { .. }, Some(_)) => {}
            (_, Some(b)) => g.kids[i].state = State::GoDoor { b },
            _ => {
                if g.rng.random_bool(0.008) && g.hen.held_by.is_none() && g.hen.pos.distance(me.pos) < 6.0 {
                    g.kids[i].state = State::ChaseHen { t: 10.0 };
                } else {
                    g.kids[i].state = State::Follow;
                }
            }
        }
        return;
    }
    // Loki: an egg goes straight home, at a sprint, and then he runs back.
    if me.eggs > 0 {
        g.kids[i].state = State::CarryEgg;
        g.say(Who::Loki, "I'm taking the egg home! Be right back!");
        return;
    }
    // Where is Skaði going? There, at a run.
    let sister = g.kids[SKADI].state;
    if let State::GoDoor { b } | State::Realize { b, .. } = sister {
        if me.state != (State::GoDoor { b }) {
            g.kids[i].state = State::GoDoor { b };
            if g.kids[i].call_cd <= 0.0 && g.rng.random_bool(0.3) {
                g.kids[i].call_cd = 8.0;
                let line = lines::pick(&mut g.rng, &["Race you!", "I'm first!", "That house! That house!", "Wait for me!"]);
                g.say(Who::Loki, line);
            }
        }
        return;
    }
    if let State::GoDoor { .. } = me.state {
        g.kids[i].state = State::Follow;
    }
    if let Some(item) = items::nearest(g, me.pos, 10.0, |it| it.kind == ItemKind::Egg) {
        g.kids[i].state = State::FetchEgg { item };
        return;
    }
    // Busy with a lollipop: content to tag along.
    if me.licking > 0.0 {
        g.kids[i].state = State::Follow;
        return;
    }
    // Sugar makes for more dashing, and less looking where he is going.
    let hyper = 0.3 * me.sugar;
    let roll: f32 = g.rng.random::<f32>() - hyper;
    if roll < 0.2 {
        // Dash past (or through) the raccoon, or, sugared up, whoever is
        // nearest.
        let a: f32 = g.rng.random_range(0.0..std::f32::consts::TAU);
        let to = rpos + Vec2::new(a.cos(), a.sin()) * g.rng.random_range(2.0..5.0);
        let through = g.rng.random_bool(0.35);
        let target = if g.rng.random_bool(me.sugar as f64) { somebody_near(g, me.pos) } else { rpos };
        let to = if through || target != rpos { target + (target - me.pos).normalize_or_zero() * 2.5 } else { to };
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
    // A spot inside a wall or a hedge of trunks: the nearest open one, and
    // reaching that counts as getting there.
    let to = town.open_near(to, RADIUS, true, 4.0).unwrap_or(to);
    let k = &mut g.kids[i];
    let left = k.pos.distance(to);
    let mut aim = to;
    if !nav::line_clear(&town, k.pos, to, false) {
        let stale = k.path_to.is_none_or(|p| p.distance(to) > 2.0) || k.path_age > 2.0 || k.path.is_empty();
        if stale {
            // On foot by the sidewalks if there is a way; wading if that is
            // the only one.
            k.path = nav::path_on(&town, k.pos, to, false, ROAD_COST)
                .or_else(|| nav::path_on(&town, k.pos, to, true, ROAD_COST))
                .unwrap_or_default();
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
    // At the curb: stop and look both ways before stepping into the road.
    // Once they have looked, they go, and keep going until they are across.
    let on_road = town.grid.at(k.pos) == Terrain::Road;
    let entering = !on_road && d.length() > 0.05 && town.grid.at(k.pos + d.normalize() * 0.45) == Terrain::Road;
    if !on_road && !entering {
        k.crossing = false;
        k.cross_wait = 0.0;
    }
    if entering && !k.crossing {
        k.cross_wait += DT;
        k.vel = Vec2::ZERO;
        k.facing = facing_of(d, k.facing);
        if k.cross_wait >= CROSS_PAUSE {
            k.crossing = true;
        }
        return left;
    }
    let want = if d.length() > 0.08 { d.normalize() * speed.min(d.length() / DT) } else { Vec2::ZERO };
    k.vel = steer(k.vel, want, 30.0);
    k.facing = facing_of(k.vel, k.facing);
    let before = k.pos;
    let dv = k.vel * DT;
    move_body(&town, &mut k.pos, dv, RADIUS, true);
    let moved = k.pos.distance(before);
    k.stride += moved;
    // Getting nowhere (wedged between a pole and a wall, say, or sliding to
    // and fro in a pocket): squeeze on past, the way a child does, and plan
    // again from there.
    if k.pos.distance(k.stuck_at) > 0.8 {
        k.stuck_at = k.pos;
        k.stuck = 0.0;
    } else if dv.length() > 0.02 {
        k.stuck += DT;
    }
    if k.stuck > 1.2 {
        k.stuck = 0.0;
        k.path.clear();
        k.path_to = None;
        let ahead = (aim - k.pos).normalize_or_zero();
        let side = Vec2::new(-ahead.y, ahead.x);
        let past = [ahead * 0.9, ahead * 0.7 + side * 0.5, ahead * 0.7 - side * 0.5, side * 0.8, -side * 0.8]
            .into_iter()
            .map(|o| k.pos + o)
            .find(|&p| town.clear(p, RADIUS, true) && nav::walls_clear(&town, k.pos, p, true));
        if let Some(p) = past.or_else(|| town.open_near(k.pos, RADIUS + 0.15, true, 1.5)) {
            k.pos = p;
            k.stuck_at = p;
        }
    }
    left
}

fn stand(g: &mut Game, i: usize) {
    let k = &mut g.kids[i];
    k.vel = steer(k.vel, Vec2::ZERO, 30.0);
    let town = g.town.clone();
    let dv = k.vel * DT;
    move_body(&town, &mut k.pos, dv, RADIUS, true);
}

fn act(g: &mut Game, i: usize) {
    let state = g.kids[i].state;
    let rpos = g.raccoon.pos;
    let me = g.kids[i].pos;
    match state {
        State::Follow => {
            // Skaði beside the raccoon on the left; Loki beside his sister.
            let back = -g.raccoon.facing.vec();
            let side = Vec2::new(-back.y, back.x);
            let (spot, far) = if i == SKADI {
                (rpos + back * 1.2 - side * 1.1, me.distance(rpos) > 6.0)
            } else {
                let sister = g.kids[SKADI].pos;
                (sister + back * 0.6 - side * 0.9, me.distance(sister) > 3.5)
            };
            if me.distance(spot) > 0.9 {
                go(g, i, spot, far);
            } else {
                stand(g, i);
                g.kids[i].facing = g.raccoon.facing;
            }
        }
        State::GoDoor { b } => {
            let spot = door_spot(g, b, i);
            // Skaði hurries when the raccoon is nearer the door than she
            // is; Loki always runs.
            let step = g.town.buildings[b].door.step();
            let hurry = i == LOKI || rpos.distance(step) < me.distance(step) + 1.0;
            if me.distance(spot) > 0.5 {
                go(g, i, spot, hurry);
            } else {
                stand(g, i);
                g.kids[i].facing = g.town.buildings[b].door.facing.opposite();
            }
        }
        State::Circle { t, centre, a, r } => {
            let t = t - DT;
            // Round at a run; a sugared Loki's loop wobbles too.
            let a = a + g.kids[i].run_speed() * 0.8 / r * DT;
            let wobble = 1.0 + 0.3 * g.kids[i].sugar * (3.0 * a).sin();
            let target = centre + Vec2::new(a.cos(), a.sin()) * r * wobble;
            go(g, i, target, true);
            if t > 0.0 {
                g.kids[i].state = State::Circle { t, centre, a, r };
            } else if let Some(item) = items::nearest(g, centre, 5.0 + r, |it| it.carried.is_none() && matches!(it.kind, ItemKind::Candy(_))) {
                g.kids[i].state = State::Fetch { item, purpose: Purpose::Keep };
            } else {
                g.kids[i].state = State::Follow;
            }
        }
        State::Realize { t, b } => {
            stand(g, i);
            let t = t - DT;
            g.kids[i].state = if t <= 0.0 { State::GoDoor { b } } else { State::Realize { t, b } };
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
                    g.items.remove(idx);
                    g.kids[i].eggs += 1;
                    g.kids[i].state = State::Follow;
                    g.say(Who::Loki, "An egg! It goes in my pail. For Mom!");
                }
            }
            _ => g.kids[i].state = State::Follow,
        },
        State::CarryEgg => {
            let home = g.town.buildings[g.town.home].door.step();
            if g.kids[i].eggs == 0 {
                g.kids[i].state = State::Follow;
                return;
            }
            if go(g, i, home, true) < 0.9 {
                let n = std::mem::take(&mut g.kids[i].eggs);
                g.kids[i].state = State::Follow;
                if let Some(b) = g.town.features.iter().find(|f| f.kind == FeatureKind::EggBasket) {
                    g.feats[b.id].n = g.feats[b.id].n.saturating_add(n);
                }
                g.events.push(SimEvent::EggHome { pos: home });
                g.discover(Id::EggDelivered);
                g.say(Who::Door(g.town.home as u32), "Another egg! Good job, Loki! Have some hákarl.");
                receive(g, LOKI, Candy::Hakarl);
                g.kids[i].mom_cd = 60.0;
                // Then all the way back, at a run: no catching up out of
                // sight until he has had time to.
                g.kids[i].back_t = home.distance(g.raccoon.pos) / g.kids[i].run_speed() + 8.0;
            }
        }
        State::Chew { t } => {
            stand(g, i);
            let t = t - DT;
            if g.kids[i].chat_cd <= 0.0 {
                g.say(Who::Loki, "Mmf! Mmmf mmf!");
                g.kids[i].chat_cd = 3.0;
            }
            g.kids[i].state = if t <= 0.0 { State::Follow } else { State::Chew { t } };
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
            // Not worth wandering off for: Loki stays near his sister, and
            // Skaði near the raccoon.
            Some(idx) if g.items[idx].pos.distance(if i == LOKI { g.kids[SKADI].pos } else { rpos }) > 12.0 => g.kids[i].state = State::Follow,
            Some(idx) if g.items[idx].carried.is_none() => {
                let at = g.items[idx].pos;
                if go(g, i, at, (i == SKADI && purpose == Purpose::GiveBack) || purpose == Purpose::Keep) < 0.5 {
                    let it = g.items.remove(idx);
                    // Loki eats candy he finds, unless it is a present.
                    // Sugared up, he eats what he came back for on the spot.
                    let eat = match purpose {
                        Purpose::Gift => false,
                        Purpose::Keep => g.rng.random_bool(g.kids[i].sugar as f64),
                        _ => g.rng.random_bool(0.6),
                    };
                    if let (LOKI, ItemKind::Candy(c), true) = (i, it.kind, eat) {
                        g.say(Who::Loki, "Mmm! Finders keepers!");
                        loki_eats(g, c);
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
            let far = me.distance(to) > 5.0;
            let left = go(g, i, to, far);
            let reach = match purpose {
                Purpose::Elsewhere(_) => 0.5,
                Purpose::Keep => f32::INFINITY,
                _ => 1.0,
            };
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
        Purpose::Keep => {
            g.kids[i].pail = g.kids[i].pail.saturating_add(1);
            g.say(Who::Loki, "Finders keepers!");
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
            g.kids[LOKI].eggs = 0;
            g.events.push(SimEvent::EggBroken { pos: loki.pos });
            g.events.push(SimEvent::Cry { pos: loki.pos });
            g.kids[LOKI].state = State::Crying { t: 9.0 };
            g.kids[LOKI].bump_cd = 4.0;
            let line = lines::pick(&mut g.rng, lines::LOKI_CRY);
            g.say(Who::Loki, line);
            g.discover(Id::EggBroken);
            g.say_message("Crack. Loki's egg. Loki is crying.");
            skadi_comes_running(g, "Loki! I'm coming!");
            return;
        }
        // Going for an egg he is careful, for once.
        let fast = loki.vel.length() > 3.6 && !matches!(loki.state, State::FetchEgg { .. });
        if fast && g.raccoon.shaken <= 0.0 && g.raccoon.act != Act::AtDoor {
            let push = (rpos - loki.pos).normalize_or_zero() * 2.0;
            g.raccoon.vel = push;
            g.raccoon.shaken = 1.5;
            let n = raccoon::spill(g, Who::Loki, 0.1, 1);
            g.kids[LOKI].bump_cd = 4.0;
            loop_back(g, rpos);
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

/// Loki, having knocked candy loose at `at`, runs round in a loop before he
/// comes back for it: the more sugar in him, the wider, longer and sillier
/// the loop.
fn loop_back(g: &mut Game, at: Vec2) {
    let sugar = g.kids[LOKI].sugar;
    let away = (g.kids[LOKI].pos - at).normalize_or_zero();
    g.kids[LOKI].state = State::Circle {
        t: g.rng.random_range(2.2..3.4) + 5.0 * sugar,
        centre: at,
        a: away.y.atan2(away.x),
        r: 2.6 + 5.0 * sugar,
    };
}

/// Whoever is nearest Loki to run into: his sister, the hen or one of the
/// townsfolk (not the dogs), else the raccoon.
fn somebody_near(g: &Game, from: Vec2) -> Vec2 {
    let mut best = (g.raccoon.pos, g.raccoon.pos.distance(from));
    let mut consider = |p: Vec2| {
        let d = p.distance(from);
        if d < best.1 {
            best = (p, d);
        }
    };
    consider(g.kids[SKADI].pos);
    if g.hen.held_by.is_none() {
        consider(g.hen.pos);
    }
    for w in g.folk.iter().filter(|w| w.kind != super::folk::Kind::Dog && w.pos.distance(from) < 10.0) {
        consider(w.pos);
    }
    best.0
}

/// Loki going flat out does not look where he is going, and the sugar makes
/// it worse. Whoever he runs into (other than the raccoon, which is
/// `contacts`) drops something, and round he goes to come back for it.
fn loki_crashes(g: &mut Game) {
    let loki = g.kids[LOKI].clone();
    let heedless = matches!(loki.state, State::Dash { .. } | State::GoDoor { .. } | State::Follow | State::Wander { .. });
    if loki.bump_cd > 0.0 || loki.vel.length() < 3.6 || loki.carrying_egg() || !heedless {
        return;
    }
    let close = |p: Vec2| p.distance(loki.pos) < 0.55;
    let skadi = close(g.kids[SKADI].pos) && !g.kids[SKADI].holding_hen();
    let hen = g.hen.held_by.is_none() && close(g.hen.pos);
    let walker = g.folk.iter().position(|w| w.kind != super::folk::Kind::Dog && close(w.pos));
    if !skadi && !hen && walker.is_none() {
        return;
    }
    // Calm, he mostly swerves in time; sugared, he mostly does not.
    g.kids[LOKI].bump_cd = 1.0;
    if !g.rng.random_bool((0.3 + 0.7 * loki.sugar) as f64) {
        return;
    }
    g.kids[LOKI].bump_cd = 3.0;
    let line = lines::pick(&mut g.rng, lines::LOKI_OOPS);
    g.say(Who::Loki, line);
    let away = loki.vel.normalize_or_zero();
    let fling = |g: &mut Game, kind: ItemKind, at: Vec2, from: Who| {
        let a: f32 = g.rng.random_range(-0.8..0.8);
        let dir = Vec2::new(away.x * a.cos() - away.y * a.sin(), away.x * a.sin() + away.y * a.cos());
        let id = g.next_item;
        g.next_item += 1;
        g.items.push(items::Item::new(id, kind, at, dir * g.rng.random_range(1.0..2.4), Some(from)));
        g.events.push(SimEvent::CandyDropped { pos: at, n: 1, by: from });
    };
    let at = if skadi {
        let p = g.kids[SKADI].pos;
        g.kids[SKADI].pail = g.kids[SKADI].pail.saturating_sub(1);
        let c = super::doors::random_treat(&mut g.rng);
        fling(g, ItemKind::Candy(c), p, Who::Skadi);
        let line = lines::pick(&mut g.rng, lines::SKADI_BUMPED);
        g.say(Who::Skadi, line);
        p
    } else if hen {
        // A hen has no candy, but a startled hen does drop something.
        let p = g.hen.pos;
        let what = if g.hen.fullness > 0.6 { ItemKind::Egg } else { ItemKind::Poop };
        fling(g, what, p, Who::Hen);
        if what == ItemKind::Poop {
            g.events.push(SimEvent::Poop { pos: p });
        }
        g.hen.state = super::hen::State::Flee { from: loki.pos, t: 1.5 };
        g.events.push(SimEvent::Squawk { pos: p });
        g.say(Who::Hen, "BWAAAAK!");
        p
    } else {
        let w = walker.unwrap_or_default();
        let (p, id) = (g.folk[w].pos, g.folk[w].id);
        g.folk[w].pail = g.folk[w].pail.saturating_sub(1);
        let c = super::doors::random_treat(&mut g.rng);
        fling(g, ItemKind::Candy(c), p, Who::Folk(id));
        let line = lines::pick(&mut g.rng, lines::FOLK_BUMPED);
        g.say(Who::Folk(id), line);
        p
    };
    g.discover(Id::LokiPinball);
    loop_back(g, at);
}

/// Loki is crying: Skaði drops whatever she is doing (gently, if it is the
/// hen) and runs to give him a hug.
fn skadi_comes_running(g: &mut Game, line: &str) {
    if g.kids[SKADI].holding_hen() {
        let p = g.kids[SKADI].pos + g.kids[SKADI].facing.vec() * 0.5;
        release_hen(g, p);
    }
    if let State::Bring { what, .. } = g.kids[SKADI].state {
        let p = g.kids[SKADI].pos;
        items::drop_item(g, what, p, Some(Who::Raccoon));
    }
    g.kids[SKADI].state = State::GoComfort;
    g.say(Who::Skadi, line);
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
