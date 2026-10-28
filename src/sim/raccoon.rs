//! The raccoon: walking and running, happiness and belly, what it does to
//! its sheet, the things it walks into, and the things it does on purpose
//! (knocking, eating, patching, taking).

use super::candy::{Bag, Candy, Counts, Mat};
use super::costume::{Costume, Wear};
use super::discover::Id;
use super::items::{self, ItemKind};
use super::{DT, Game, SimEvent, TickInput, Who, facing_of, lines, move_body, steer};
use crate::world::Dir;
use crate::world::grid::Terrain;
use crate::world::town::FeatureKind;
use glam::Vec2;
use rand::RngExt;

pub const RADIUS: f32 = 0.3;
pub const WALK: f32 = 3.4;
pub const RUN: f32 = 6.0;
/// A belly holds this much before it will take no more.
pub const BELLY_FULL: f32 = 12.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Act {
    Free,
    /// Rapping on a door (the visit takes over from here).
    AtDoor,
    Eating { t: f32, candy: Candy },
    Patching { t: f32, mat: Mat },
    /// Flat on the ground, seeing stars.
    Fallen { t: f32 },
    /// A happy little dance (hákarl, or a favourite).
    Dancing { t: f32 },
    /// Reaching into a bowl, up a pole, into a tree.
    Taking { t: f32 },
}

#[derive(Clone, Debug)]
pub struct Raccoon {
    pub pos: Vec2,
    pub prev: Vec2,
    pub vel: Vec2,
    pub facing: Dir,
    pub act: Act,
    pub costume: Costume,
    pub bag: Bag,
    pub mats: Counts<Mat>,
    /// Units of candy in the belly.
    pub belly: f32,
    /// 0 to 1. Walking wears it down, eating brings it back, and a sad
    /// raccoon is a slow raccoon.
    pub happy: f32,
    /// Seconds of sugar rush left.
    pub rush: f32,
    pub running: bool,
    pub swimming: bool,
    /// Where a click asked it to go.
    pub path: Vec<Vec2>,
    /// Distance walked, for the walk cycle.
    pub stride: f32,
    /// No new bumps for a moment after one.
    pub shaken: f32,
    /// Seconds since it last pressed the action key at a bowl (for taking
    /// the whole bowl).
    pub bowl_grab: Option<(usize, f32)>,
    pub maple_eaten: u32,
    /// A black cat crossed its path: the next door is less friendly.
    pub bad_luck: u8,
    pub cows_seen: std::collections::BTreeSet<usize>,
}

impl Raccoon {
    pub fn new(at: Vec2) -> Raccoon {
        Raccoon {
            pos: at,
            prev: at,
            vel: Vec2::ZERO,
            facing: Dir::S,
            act: Act::Free,
            costume: Costume::default(),
            bag: Bag::default(),
            mats: Counts::default(),
            belly: 2.0,
            happy: 0.8,
            rush: 0.0,
            running: false,
            swimming: false,
            path: Vec::new(),
            stride: 0.0,
            shaken: 0.0,
            bowl_grab: None,
            maple_eaten: 0,
            bad_luck: 0,
            cows_seen: Default::default(),
        }
    }

    /// The belly's size for the art: 0 (lean) to 4 (round as a pumpkin).
    pub fn belly_level(&self) -> u8 {
        ((self.belly / BELLY_FULL) * 4.0).round().clamp(0.0, 4.0) as u8
    }

    /// How well it passes for a child at a door, before the person there
    /// has their say.
    pub fn passing(&self) -> f32 {
        let mut p = self.costume.passing();
        // A round belly hitches the sheet up and shows the legs.
        p -= (self.belly_level() as f32 - 1.0).max(0.0) * 0.04;
        p.clamp(0.0, 1.0)
    }

    pub fn speed(&self, terrain: Terrain) -> f32 {
        let base = if self.running { RUN } else { WALK };
        let mood = 0.4 + 0.6 * self.happy;
        let rush = if self.rush > 0.0 { 1.25 } else { 1.0 };
        let stuffed = if self.belly > BELLY_FULL * 0.85 { 0.85 } else { 1.0 };
        let haul = 1.0 / (1.0 + self.bag.total() as f32 / 600.0);
        base * mood * rush * stuffed * haul * terrain.pace()
    }

    pub fn busy(&self) -> bool {
        !matches!(self.act, Act::Free)
    }
}

pub fn step(g: &mut Game, input: &TickInput) {
    g.raccoon.prev = g.raccoon.pos;
    let r = &mut g.raccoon;
    r.shaken = (r.shaken - DT).max(0.0);
    r.rush = (r.rush - DT).max(0.0);
    // Digestion, drying and airing out.
    r.belly = (r.belly - DT / 22.0).max(0.0);
    r.costume.wet = (r.costume.wet - DT / 80.0).max(0.0);
    r.costume.smell = (r.costume.smell - DT / 150.0).max(0.0);
    if let Some((_, t)) = &mut r.bowl_grab {
        *t += DT;
    }
    match r.act {
        Act::Eating { t, candy } => {
            let t = t - DT;
            if t <= 0.0 {
                finish_eating(g, candy);
            } else {
                g.raccoon.act = Act::Eating { t, candy };
            }
            g.raccoon.vel = Vec2::ZERO;
            return;
        }
        Act::Patching { t, mat } => {
            let t = t - DT;
            if t <= 0.0 {
                g.raccoon.act = Act::Free;
                if g.raccoon.costume.patch(mat).is_some() {
                    g.stats.patched += 1;
                    let pos = g.raccoon.pos;
                    g.events.push(SimEvent::Patched { pos, mat });
                    g.say_message(format!("Patched the sheet with {}.", mat.name()));
                    match mat {
                        Mat::Gum => g.discover(Id::GumPatch),
                        Mat::Flyer => g.discover(Id::FlyerPatch),
                        Mat::Tp => g.discover(Id::TpPatch),
                        _ => {}
                    }
                } else {
                    g.raccoon.mats.add(mat, 1);
                }
            } else {
                g.raccoon.act = Act::Patching { t, mat };
            }
            g.raccoon.vel = Vec2::ZERO;
            return;
        }
        Act::Fallen { t } | Act::Dancing { t } | Act::Taking { t } => {
            let t = t - DT;
            g.raccoon.act = if t <= 0.0 {
                Act::Free
            } else {
                match g.raccoon.act {
                    Act::Fallen { .. } => Act::Fallen { t },
                    Act::Dancing { .. } => Act::Dancing { t },
                    _ => Act::Taking { t },
                }
            };
            g.raccoon.vel *= 0.85;
            let v = g.raccoon.vel * DT;
            let town = g.town.clone();
            move_body(&town, &mut g.raccoon.pos, v, RADIUS, true);
            return;
        }
        Act::AtDoor => {
            // The visit decides when this ends; moving away ends it too.
            g.raccoon.vel = Vec2::ZERO;
            if input.dir.length_squared() > 0.0 {
                super::doors::walk_away(g);
            } else {
                return;
            }
        }
        Act::Free => {}
    }
    if input.eat {
        start_eating(g);
        return;
    }
    if input.patch {
        start_patching(g, true);
        return;
    }
    if input.act {
        act(g);
        if g.raccoon.busy() {
            return;
        }
    }
    let town = g.town.clone();
    let r = &mut g.raccoon;
    if let Some(to) = input.goto {
        r.path = crate::world::nav::path(&town, r.pos, to, true).unwrap_or_default();
        if !r.path.is_empty() {
            r.path.remove(0);
        }
    }
    let mut want = input.dir;
    if want.length_squared() > 0.0 {
        r.path.clear();
    } else if let Some(&next) = r.path.first() {
        let d = next - r.pos;
        if d.length() < 0.35 {
            r.path.remove(0);
        } else {
            want = d.normalize();
        }
    }
    r.running = input.run && want.length_squared() > 0.0;
    let terrain = town.grid.at(r.pos);
    r.swimming = terrain == Terrain::Water;
    let speed = r.speed(terrain);
    let target = if want.length_squared() > 0.0 { want.normalize() * speed } else { Vec2::ZERO };
    r.vel = steer(r.vel, target, 40.0);
    r.facing = facing_of(r.vel, r.facing);
    let before = r.pos;
    let d = r.vel * DT;
    move_body(&town, &mut r.pos, d, RADIUS, true);
    let moved = r.pos.distance(before);
    r.stride += moved;
    g.stats.distance += moved as f64;
    // Happiness wears down with every step not fuelled by candy.
    let drain = if r.swimming {
        0.011
    } else if r.running {
        0.0075
    } else {
        0.0038
    };
    let was = r.happy;
    r.happy = (r.happy - moved * drain).max(0.0);
    if was > 0.0 && r.happy <= 0.0 {
        g.discover(Id::Miserable);
        g.say(Who::Raccoon, "*sad little chitter*");
    }
    // The hem frays as it drags.
    let fray = moved * if g.raccoon.running { 0.02 } else { 0.007 };
    if fray > 0.0 {
        let roll = g.rng.random();
        if g.raccoon.costume.wear(Wear::Walk, fray, roll).is_some() {
            let pos = g.raccoon.pos;
            g.events.push(SimEvent::Tear { pos });
        }
    }
    if g.raccoon.swimming {
        if g.raccoon.costume.wet < 0.9 {
            g.discover(Id::Swam);
        }
        g.raccoon.costume.wet = 1.0;
    }
    if moved > 0.0 {
        surroundings(g, moved);
    }
    pick_up(g);
}

/// What the raccoon brushes through or steps in.
fn surroundings(g: &mut Game, moved: f32) {
    let town = g.town.clone();
    let pos = g.raccoon.pos;
    let running = g.raccoon.running;
    for f in town.features_near(pos, RADIUS) {
        let reach = f.kind.reach() * f.size.max(0.6);
        if f.pos.distance(pos) > reach + RADIUS * 0.5 {
            continue;
        }
        match f.kind {
            FeatureKind::Bush => {
                let roll = g.rng.random();
                let k = if running { 0.12 } else { 0.05 };
                if g.raccoon.costume.wear(Wear::Bush, moved * k, roll).is_some() {
                    g.events.push(SimEvent::Tear { pos });
                    g.say_message("The bush snags the sheet.");
                }
                g.raccoon.vel *= 0.93;
            }
            FeatureKind::Raspberry => {
                let roll = g.rng.random();
                let k = if running { 0.4 } else { 0.18 };
                if g.raccoon.costume.wear(Wear::Thorns, moved * k, roll).is_some() {
                    g.events.push(SimEvent::Tear { pos });
                    g.say_message("Ow! Raspberry thorns rip the sheet.");
                }
                g.raccoon.vel *= 0.9;
                let st = &mut g.feats[f.id];
                if !st.used {
                    st.used = true;
                    st.regrow = 150.0;
                    let n = g.rng.random_range(3..7);
                    g.raccoon.bag.add(Candy::Raspberry, n);
                    g.events.push(SimEvent::Berries { pos: f.pos });
                    g.say_message(format!("Picked {n} raspberries. They count!"));
                    g.discover(Id::Raspberries);
                }
            }
            FeatureKind::LeafPile => {
                let st = &mut g.feats[f.id];
                if !st.used {
                    st.used = true;
                    st.regrow = 120.0;
                    g.events.push(SimEvent::Leaves { pos: f.pos });
                    g.discover(Id::LeafPile);
                    // Leaves stick to the sheet, and over holes.
                    let r = &mut g.raccoon;
                    r.costume.leaves = r.costume.leaves.saturating_add(2);
                    let mut stuck = 0;
                    for _ in 0..2 {
                        if r.costume.holes() > 0 && r.costume.patch(Mat::Leaf).is_some() {
                            stuck += 1;
                        }
                    }
                    r.mats.add(Mat::Leaf, 1);
                    g.say_message(if stuck > 0 {
                        "Leaves everywhere! Some stick over the holes."
                    } else {
                        "Leaves everywhere! A few stick to the sheet."
                    });
                    if let Some(owner) = f.owner {
                        if g.rng.random_bool(0.5) {
                            g.say(Who::Door(owner as u32), "HEY! I just raked that!");
                        }
                    }
                }
            }
            FeatureKind::Puddle => {
                let r = &mut g.raccoon;
                let k = if running { 0.12 } else { 0.03 };
                r.costume.dirt = (r.costume.dirt + moved * k).min(1.0);
                if running && g.rng.random_bool((moved * 0.8).min(1.0) as f64) {
                    g.events.push(SimEvent::Splash { pos });
                }
            }
            _ => {}
        }
    }
    // A cow close by counts as seen, too.
    for f in town.features_near(pos, 2.5) {
        if f.kind == FeatureKind::Cow && g.raccoon.cows_seen.insert(f.id) && g.raccoon.cows_seen.len() == 6 {
            g.discover(Id::Cows);
            g.say_message("That's six painted cows. Montpelier really committed to the cows.");
        }
    }
    // Landmarks.
    for b in &town.buildings {
        if b.rect.distance(pos) < 2.0 {
            match b.kind {
                crate::world::town::BuildingKind::Tower => g.discover(Id::Tower),
                crate::world::town::BuildingKind::StateHouse => g.discover(Id::StateHouse),
                _ => {}
            }
        }
    }
}

/// Candy and scraps are picked up by walking over them; poop is slipped in.
fn pick_up(g: &mut Game) {
    let pos = g.raccoon.pos;
    let mut taken = Vec::new();
    let mut slip = None;
    for (i, it) in g.items.iter().enumerate() {
        if it.carried.is_some() || it.z > 0.3 {
            continue;
        }
        let d = it.pos.distance(pos);
        match it.kind {
            ItemKind::Candy(_) | ItemKind::Mat(_) if d < 0.55 && it.age > 0.4 => taken.push(i),
            ItemKind::Poop | ItemKind::Barf if d < 0.36 && g.raccoon.vel.length() > 1.0 && g.raccoon.shaken <= 0.0 => slip = Some(i),
            _ => {}
        }
    }
    for &i in taken.iter().rev() {
        let it = g.items.remove(i);
        let what = match it.kind {
            ItemKind::Candy(c) => {
                g.raccoon.bag.add(c, 1);
                c.name().to_string()
            }
            ItemKind::Mat(m) => {
                g.raccoon.mats.add(m, 1);
                m.name().to_string()
            }
            _ => continue,
        };
        g.events.push(SimEvent::Picked { pos: it.pos, what });
    }
    if let Some(i) = slip {
        let i = if taken.iter().any(|&t| t < i) { i - taken.iter().filter(|&&t| t < i).count() } else { i };
        let barf = i < g.items.len() && g.items[i].kind == ItemKind::Barf;
        if i < g.items.len() {
            g.items.remove(i);
        }
        let r = &mut g.raccoon;
        r.act = Act::Fallen { t: 1.1 };
        r.shaken = 2.0;
        r.happy = (r.happy - 0.15).max(0.0);
        r.costume.dirt = (r.costume.dirt + 0.2).min(1.0);
        r.costume.smell = 1.0;
        r.vel *= 0.6;
        g.events.push(SimEvent::Slip { pos });
        g.say(Who::Raccoon, "*SPLAT*");
        if barf {
            g.discover(Id::SlippedInBarf);
            g.say_message("Slipped in Gravy's barf. Unhappy, and smelly.");
        } else {
            g.discover(Id::SlippedInPoop);
            g.say_message("Slipped in Gullinkombi's poop. Unhappy, and smelly.");
        }
    }
}

fn start_eating(g: &mut Game) {
    let r = &g.raccoon;
    if r.belly >= BELLY_FULL {
        g.discover(Id::Stuffed);
        g.say(Who::Raccoon, "*burp*");
        g.say_message("Too full to eat another bite. Walk it off.");
        return;
    }
    let Some(c) = g.raccoon.bag.best() else {
        g.say_message(if g.raccoon.bag.is_empty() { "The bag is empty." } else { "Nothing in the bag a raccoon can eat." });
        return;
    };
    g.raccoon.bag.take(c);
    g.raccoon.act = Act::Eating { t: 0.9, candy: c };
}

fn finish_eating(g: &mut Game, c: Candy) {
    let pos = g.raccoon.pos;
    let r = &mut g.raccoon;
    r.belly = (r.belly + c.filling()).min(BELLY_FULL + 2.0);
    r.happy = (r.happy + c.joy()).min(1.0);
    r.rush = r.rush.max(c.rush());
    if let Some(m) = c.leaves() {
        r.mats.add(m, 1);
    }
    if matches!(c, Candy::MapleLeaf | Candy::SyrupJug | Candy::MapleCotton) {
        r.maple_eaten += 1;
        if r.maple_eaten == 5 {
            g.discover(Id::MapleOverload);
            g.say(Who::Raccoon, "*maple-induced vibrating*");
        }
    }
    g.stats.eaten += 1;
    g.events.push(SimEvent::Ate { candy: c, pos });
    if c == Candy::Hakarl {
        g.raccoon.happy = 1.0;
        g.raccoon.act = Act::Dancing { t: 1.6 };
        g.events.push(SimEvent::Loved { pos });
        g.say(Who::Raccoon, "!!!!!!");
        g.say_message("Hákarl. The raccoon has never been happier.");
    } else if c.loved() {
        g.raccoon.act = Act::Dancing { t: 0.8 };
        g.events.push(SimEvent::Loved { pos });
        g.say_message(format!("Ate {}. Delicious!", c.name()));
    } else {
        g.raccoon.act = Act::Free;
        g.say_message(format!("Ate {}.", c.name()));
    }
    if c == Candy::Toothbrush {
        g.discover(Id::Toothbrush);
    }
    if g.raccoon.belly >= BELLY_FULL {
        g.discover(Id::Stuffed);
        g.say(Who::Raccoon, "*burp*");
    }
}

/// Patching: the best material on the worst tear. `asked` says whether the
/// player pressed the patch key (and wants telling why not).
pub fn start_patching(g: &mut Game, asked: bool) -> bool {
    if g.raccoon.costume.worst().is_none() {
        if asked {
            g.say_message("The sheet has no holes to patch. Yet.");
        }
        return false;
    }
    let best = g
        .raccoon
        .mats
        .kinds()
        .map(|(m, _)| m)
        .max_by(|a, b| a.preference().total_cmp(&b.preference()));
    let Some(m) = best else {
        if asked {
            g.say_message("Nothing to patch with. Look for paper, gum, flyers, leaves...");
        }
        return false;
    };
    g.raccoon.mats.take(m);
    g.raccoon.act = Act::Patching { t: 1.2, mat: m };
    true
}

/// The action key: knock if at a door, take if at a bowl, a pole or a
/// decorated tree, and otherwise patch.
fn act(g: &mut Game) {
    let pos = g.raccoon.pos;
    let town = g.town.clone();
    if super::gravy::rub(g) {
        g.raccoon.act = Act::Taking { t: 1.0 };
        return;
    }
    if let Some(b) = town.door_near(pos, 1.0) {
        super::doors::knock(g, b.id);
        return;
    }
    let near = town
        .features_near(pos, 0.9)
        .filter(|f| matches!(f.kind, FeatureKind::CandyBowl | FeatureKind::FlyerPole | FeatureKind::TpTree))
        .min_by(|a, b| a.pos.distance(pos).total_cmp(&b.pos.distance(pos)))
        .cloned();
    if let Some(f) = near {
        take_from(g, f.id, f.kind, f.owner);
        return;
    }
    start_patching(g, true);
}

fn take_from(g: &mut Game, id: usize, kind: FeatureKind, owner: Option<usize>) {
    let st = g.feats[id];
    match kind {
        FeatureKind::CandyBowl => {
            if st.n == 0 {
                g.say_message("The bowl is empty. \"PLEASE TAKE ONE\", says the sign, to nobody.");
                return;
            }
            let again = matches!(g.raccoon.bowl_grab, Some((b, t)) if b == id && t < 6.0);
            if again {
                // The whole bowl.
                let n = st.n;
                for _ in 0..n {
                    let c = super::doors::random_treat(&mut g.rng);
                    g.raccoon.bag.add(c, 1);
                }
                g.feats[id].n = 0;
                g.feats[id].used = true;
                g.feats[id].regrow = 200.0;
                g.discover(Id::TookWholeBowl);
                g.say_message(format!("Took the whole bowl: {n} pieces. The sign said PLEASE TAKE ONE."));
                g.raccoon.act = Act::Taking { t: 0.8 };
                super::kids::skadi_objects_to_greed(g, id, n);
                let _ = owner;
            } else {
                let c = super::doors::random_treat(&mut g.rng);
                g.raccoon.bag.add(c, 1);
                g.feats[id].n -= 1;
                if g.feats[id].n == 0 {
                    g.feats[id].used = true;
                    g.feats[id].regrow = 200.0;
                }
                g.raccoon.bowl_grab = Some((id, 0.0));
                g.raccoon.act = Act::Taking { t: 0.5 };
                g.say_message(format!("Took {} from the bowl. (It says take one. Press again to take more.)", c.name()));
            }
        }
        FeatureKind::FlyerPole => {
            if st.n == 0 {
                g.say_message("No flyers left on this pole.");
                return;
            }
            g.feats[id].n -= 1;
            g.raccoon.mats.add(Mat::Flyer, 1);
            g.raccoon.act = Act::Taking { t: 0.6 };
            let what = lines::pick(&mut g.rng, &["LOST CAT", "CONTRA DANCE FRIDAY", "FIREWOOD 4 SALE", "GUITAR LESSONS", "SAVE THE RIVER", "MAPLE FEST"]);
            g.say_message(format!("Tore down a flyer (\"{what}\"). Good patching paper."));
        }
        FeatureKind::TpTree => {
            if st.n == 0 {
                g.say_message("Somebody already took all the toilet paper.");
                return;
            }
            g.feats[id].n -= 1;
            g.raccoon.mats.add(Mat::Tp, 1);
            g.raccoon.act = Act::Taking { t: 0.8 };
            g.say_message("Pulled down a streamer of toilet paper.");
            if g.feats[id].n == 0 {
                g.feats[id].used = true;
            }
        }
        _ => {}
    }
}

/// The raccoon goes down (bumped, tripped) and some of the bag goes flying.
/// `frac` of the bag, at least `min`, is scattered.
pub fn knock_down(g: &mut Game, by: Who, frac: f32, min: u64, push: Vec2) -> u64 {
    let r = &mut g.raccoon;
    r.act = Act::Fallen { t: 1.3 };
    r.shaken = 2.5;
    r.vel = push;
    r.path.clear();
    let roll = g.rng.random();
    if g.raccoon.costume.wear(Wear::Fall, 0.5, roll).is_some() {
        let pos = g.raccoon.pos;
        g.events.push(SimEvent::Tear { pos });
    }
    let pos = g.raccoon.pos;
    g.events.push(SimEvent::Fall { who: Who::Raccoon, pos });
    spill(g, by, frac, min)
}

/// Some of the bag spills without the raccoon falling.
pub fn spill(g: &mut Game, by: Who, frac: f32, min: u64) -> u64 {
    let total = g.raccoon.bag.total();
    if total == 0 {
        return 0;
    }
    let n = ((total as f32 * frac).round() as u64).max(min).min(total);
    let pos = g.raccoon.pos;
    for _ in 0..n {
        // Pick a piece in proportion to how many of each there are.
        let mut k = g.rng.random_range(0..g.raccoon.bag.total());
        let mut chosen = None;
        for (c, cnt) in g.raccoon.bag.kinds() {
            if k < cnt {
                chosen = Some(c);
                break;
            }
            k -= cnt;
        }
        let Some(c) = chosen else { break };
        g.raccoon.bag.take(c);
        let a: f32 = g.rng.random_range(0.0..std::f32::consts::TAU);
        let s: f32 = g.rng.random_range(1.2..3.4);
        let it = items::Item::new(g.next_item, ItemKind::Candy(c), pos, Vec2::new(a.cos(), a.sin()) * s, Some(Who::Raccoon));
        g.next_item += 1;
        g.items.push(it);
    }
    g.events.push(SimEvent::CandyDropped { pos, n: n as u32, by });
    n
}
