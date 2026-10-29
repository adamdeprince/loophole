//! Doors: who lives behind each one, how much candy they have left (and
//! how fast they restock), what they remember, and what happens when a
//! raccoon in a sheet knocks.

use super::candy::{Candy, Mat};
use super::discover::Id;
use super::raccoon::Act;
use super::{DT, Game, SimEvent, Who, lines};
use crate::world::plan::Town;
use crate::world::town::BuildingKind;
use glam::Vec2;
use rand::RngExt;
use rand_chacha::ChaCha8Rng;

/// The person who answers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Persona {
    Generous,
    Normal,
    Suspicious,
    /// Can't see past the end of the porch. Everybody is a lovely child.
    Nearsighted,
    Grumpy,
    Dentist,
    /// Gives out full-size bars.
    FullSize,
    /// Only maple. It is Vermont.
    Maple,
    /// Cheddar instead of candy.
    Cheese,
    /// Loki and Skaði's mother, dressed as a jug of maple syrup, with a
    /// platter of hákarl.
    Mom,
    /// A shop, a church, the fire station: whoever is minding the door.
    Keeper,
    /// Ethan Allen, on top of the Hubbard Park tower. Do not ask.
    Ethan,
    /// Robert Frost, in the library door, with a book and a verse.
    Frost,
    /// Calvin Coolidge, in the State House door, saying nothing.
    Coolidge,
    /// Nobody answers (an office after hours, the tower).
    Nobody,
}

impl Persona {
    /// How well a costume must pass for this person to hand over candy.
    fn threshold(self) -> f32 {
        match self {
            Persona::Generous => 0.38,
            Persona::Normal => 0.5,
            Persona::Suspicious => 0.68,
            Persona::Nearsighted => 0.05,
            Persona::Grumpy => 0.55,
            Persona::Dentist => 0.45,
            Persona::FullSize => 0.55,
            Persona::Maple | Persona::Cheese => 0.48,
            Persona::Mom => 0.0,
            Persona::Keeper => 0.36,
            Persona::Ethan => 0.0,
            // A poet sees the child under the sheet.
            Persona::Frost => 0.2,
            Persona::Coolidge => 0.3,
            Persona::Nobody => 2.0,
        }
    }

    /// Pieces handed out before the bowl is empty, and pieces back in it per
    /// minute: some people restock quickly, some barely at all.
    fn stock(self) -> (f32, f32) {
        match self {
            Persona::Generous => (40.0, 1.6),
            Persona::Normal => (24.0, 0.9),
            Persona::Suspicious => (16.0, 0.5),
            Persona::Nearsighted => (30.0, 0.7),
            Persona::Grumpy => (12.0, 0.25),
            Persona::Dentist => (30.0, 1.0),
            Persona::FullSize => (10.0, 0.3),
            Persona::Maple => (28.0, 1.2),
            Persona::Cheese => (22.0, 0.8),
            Persona::Mom => (80.0, 4.0),
            Persona::Keeper => (60.0, 2.2),
            Persona::Ethan => (6.0, 0.4),
            Persona::Frost => (40.0, 1.5),
            Persona::Coolidge => (30.0, 1.0),
            Persona::Nobody => (0.0, 0.0),
        }
    }
}

#[derive(Clone, Debug)]
pub struct House {
    pub persona: Persona,
    pub supply: f32,
    pub max: f32,
    /// Pieces per second back in the bowl.
    pub refill: f32,
    /// The porch light: on while there is candy to give.
    pub light: bool,
    /// How the raccoon looked last time it came, and how many times.
    pub last_seen: Option<u32>,
    pub visits: u32,
    /// Wariness from word going round that there is a raccoon about.
    pub wary: f32,
    /// Somebody tried a moment ago: the door does not open again at once.
    pub cooldown: f32,
    /// The raccoon has given them a present tonight.
    pub touched: bool,
}

impl House {
    pub fn new_night(&mut self) {
        self.touched = false;
        self.supply = self.max;
        self.light = self.max >= 1.0;
        self.last_seen = None;
        self.visits = 0;
        self.wary = 0.0;
    }

    pub fn answers(&self) -> bool {
        self.persona != Persona::Nobody
    }
}

pub fn houses(town: &Town, rng: &mut ChaCha8Rng) -> Vec<House> {
    town.buildings
        .iter()
        .map(|b| {
            let persona = match b.kind {
                BuildingKind::House => match rng.random_range(0..100) {
                    0..=21 => Persona::Generous,
                    22..=51 => Persona::Normal,
                    52..=63 => Persona::Suspicious,
                    64..=71 => Persona::Nearsighted,
                    72..=78 => Persona::Grumpy,
                    79..=82 => Persona::Dentist,
                    83..=86 => Persona::FullSize,
                    87..=94 => Persona::Maple,
                    _ => Persona::Cheese,
                },
                BuildingKind::Home => Persona::Mom,
                BuildingKind::Office => Persona::Nobody,
                BuildingKind::Tower => Persona::Ethan,
                BuildingKind::Library => Persona::Frost,
                BuildingKind::StateHouse => Persona::Coolidge,
                _ => Persona::Keeper,
            };
            let (max, per_minute) = persona.stock();
            // No two households restock at quite the same pace.
            let pace = rng.random_range(0.7..1.3);
            House {
                persona,
                supply: max,
                max,
                refill: per_minute * pace / 60.0,
                light: max >= 1.0,
                last_seen: None,
                visits: 0,
                wary: 0.0,
                cooldown: 0.0,
                touched: false,
            }
        })
        .collect()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Knocking,
    Waiting,
    Open,
    Answer,
    Closing,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Verdict {
    Give { candy: Vec<Candy>, mats: Vec<Mat>, line: String },
    Refuse { line: String, scream: bool },
    Empty { line: String },
    Nobody { line: String },
}

#[derive(Clone, Debug)]
pub struct Visit {
    pub b: usize,
    pub phase: Phase,
    pub t: f32,
    pub verdict: Option<Verdict>,
}

impl Visit {
    /// The door is open (and somebody is in it).
    pub fn open(&self) -> bool {
        matches!(self.phase, Phase::Open | Phase::Answer)
    }
}

pub fn random_treat(rng: &mut ChaCha8Rng) -> Candy {
    match rng.random_range(0..100) {
        0..=17 => Candy::Chocolate,
        18..=29 => Candy::CandyCorn,
        30..=41 => Candy::Lollipop,
        42..=51 => Candy::PeanutCup,
        52..=60 => Candy::GummyWorms,
        61..=68 => Candy::Caramel,
        69..=80 => Candy::MapleLeaf,
        81..=85 => Candy::MapleCotton,
        86..=88 => Candy::SyrupJug,
        89..=91 => Candy::PopcornBall,
        92..=94 => Candy::CiderDonut,
        _ => Candy::Cheddar,
    }
}

/// The raccoon knocks on building `b`'s door.
pub fn knock(g: &mut Game, b: usize) {
    if g.visit.is_some() {
        return;
    }
    if g.houses[b].cooldown > 0.0 {
        g.say_message("Give them a moment, they only just shut the door.");
        return;
    }
    let town = g.town.clone();
    let bd = &town.buildings[b];
    g.raccoon.act = Act::AtDoor;
    g.raccoon.facing = bd.door.facing.opposite();
    g.raccoon.vel = Vec2::ZERO;
    g.raccoon.path.clear();
    g.stats.doors_knocked += 1;
    g.visit = Some(Visit {
        b,
        phase: Phase::Knocking,
        t: 0.6,
        verdict: None,
    });
    g.events.push(SimEvent::Knock { b });
}

/// The raccoon walks off before the door is done with it.
pub fn walk_away(g: &mut Game) {
    if let Some(v) = g.visit.take() {
        if v.open() {
            g.events.push(SimEvent::DoorClose { b: v.b });
            g.say(Who::Door(v.b as u32), "...okay then. Bye?");
        }
        g.houses[v.b].cooldown = 2.0;
    }
    g.raccoon.act = Act::Free;
}

pub fn step(g: &mut Game) {
    // Restocking and forgetting.
    for (i, h) in g.houses.iter_mut().enumerate() {
        h.cooldown = (h.cooldown - DT).max(0.0);
        h.wary = (h.wary - DT * 0.004).max(0.0);
        if h.supply < h.max {
            h.supply = (h.supply + h.refill * DT).min(h.max);
        }
        let lit = h.supply >= 1.0;
        if lit != h.light && h.answers() {
            h.light = lit;
            g.events.push(if lit { SimEvent::LightOn { b: i } } else { SimEvent::LightOut { b: i } });
        }
    }
    let Some(mut v) = g.visit.take() else { return };
    v.t -= DT;
    let b = v.b;
    let who = Who::Door(b as u32);
    if v.t > 0.0 {
        g.visit = Some(v);
        return;
    }
    match v.phase {
        Phase::Knocking => {
            v.phase = Phase::Waiting;
            v.t = if g.houses[b].answers() { g.rng.random_range(0.5..1.3) } else { 1.4 };
            g.visit = Some(v);
        }
        Phase::Waiting => {
            let h = &g.houses[b];
            if !h.answers() {
                let lines = if g.town.buildings[b].kind == BuildingKind::Office { lines::OFFICE } else { lines::NOBODY };
                let line = lines::pick(&mut g.rng, lines);
                g.say(who, line);
                g.raccoon.act = Act::Free;
                g.houses[b].cooldown = 3.0;
                return;
            }
            v.phase = Phase::Open;
            v.t = 1.1;
            g.events.push(SimEvent::DoorOpen { b });
            let greet = match h.persona {
                Persona::Mom => lines::pick(&mut g.rng, lines::MOM),
                Persona::Coolidge => lines::pick(&mut g.rng, lines::COOLIDGE),
                _ => lines::pick(&mut g.rng, lines::GREET),
            };
            g.say(who, greet);
            let tot = lines::pick(&mut g.rng, lines::TRICK_OR_TREAT);
            g.say(Who::Raccoon, tot);
            g.visit = Some(v);
        }
        Phase::Open => {
            let verdict = judge(g, b);
            deliver(g, b, &verdict);
            v.verdict = Some(verdict);
            v.phase = Phase::Answer;
            v.t = 2.0;
            g.visit = Some(v);
        }
        Phase::Answer => {
            v.phase = Phase::Closing;
            v.t = 0.5;
            g.events.push(SimEvent::DoorClose { b });
            g.visit = Some(v);
        }
        Phase::Closing => {
            g.raccoon.act = Act::Free;
            g.houses[b].cooldown = 2.5;
        }
    }
}

/// Whether Loki and Skaði are close by (they make any ghost more
/// convincing, and they get candy too).
fn kids_along(g: &Game, at: Vec2) -> [bool; 2] {
    [g.kids[0].pos.distance(at) < 4.5, g.kids[1].pos.distance(at) < 4.5]
}

fn judge(g: &mut Game, b: usize) -> Verdict {
    let town = g.town.clone();
    let bd = &town.buildings[b];
    let step = bd.door.step();
    let h = g.houses[b].clone();
    if h.supply < 1.0 {
        g.discover(Id::LightsOut);
        return Verdict::Empty {
            line: lines::pick(&mut g.rng, lines::EMPTY).to_string(),
        };
    }
    let fp = g.raccoon.costume.fingerprint();
    let along = kids_along(g, step);
    let mut p = g.raccoon.passing();
    if along[0] || along[1] {
        p += 0.12;
    }
    p -= h.wary;
    if g.raccoon.bad_luck > 0 {
        // The black cat's doing.
        g.raccoon.bad_luck = 0;
        p -= 0.3;
    }
    let total = g.raccoon.bag.total();
    let greedy = total > 40;
    if greedy {
        p -= ((total - 40) as f32 / 400.0).min(0.25);
    }
    let smelly = g.raccoon.costume.smell > 0.3;
    if smelly {
        p -= 0.25;
    }
    let mut again = false;
    if h.visits > 0 {
        match h.last_seen {
            Some(last) if super::costume::Costume::difference(last, fp) < 3 => {
                again = true;
                p -= 0.45;
            }
            Some(_) => g.discover(Id::NewGhost),
            None => {}
        }
    }
    let margin = p - h.persona.threshold();
    let pass = margin >= 0.0 || (margin > -0.12 && g.rng.random_bool(0.5));
    {
        let hm = &mut g.houses[b];
        hm.visits += 1;
        hm.last_seen = Some(fp);
    }
    if !pass {
        g.stats.refusals += 1;
        g.discover(Id::Refused);
        if h.wary > 0.1 {
            g.discover(Id::WordSpreads);
        }
        if again {
            g.discover(Id::SecondVisit);
        }
        // Sometimes a sad enough raccoon gets one anyway.
        if g.raccoon.happy < 0.3 && g.rng.random_bool(0.2) {
            g.discover(Id::PityCandy);
            return Verdict::Give {
                candy: vec![random_treat(&mut g.rng)],
                mats: Vec::new(),
                line: lines::pick(&mut g.rng, lines::PITY).to_string(),
            };
        }
        let scream = margin < -0.32
            && matches!(h.persona, Persona::Normal | Persona::Suspicious | Persona::Generous | Persona::Maple)
            && g.rng.random_bool(0.5);
        let line = if scream {
            lines::pick(&mut g.rng, lines::SCREAM)
        } else if again {
            lines::pick(&mut g.rng, lines::AGAIN)
        } else if smelly {
            lines::pick(&mut g.rng, lines::REFUSE_SMELL)
        } else if greedy && g.rng.random_bool(0.5) {
            lines::pick(&mut g.rng, lines::REFUSE_GREEDY)
        } else {
            lines::pick(&mut g.rng, lines::REFUSE)
        };
        return Verdict::Refuse {
            line: line.to_string(),
            scream,
        };
    }
    let mut candy = Vec::new();
    let mut mats = Vec::new();
    let rng = &mut g.rng;
    let line: String = match h.persona {
        Persona::Generous => {
            for _ in 0..rng.random_range(2..4) {
                candy.push(random_treat(rng));
            }
            lines::pick(rng, lines::GIVE).into()
        }
        Persona::Normal => {
            for _ in 0..rng.random_range(1..3) {
                candy.push(random_treat(rng));
            }
            lines::pick(rng, lines::GIVE).into()
        }
        Persona::Suspicious => {
            candy.push(random_treat(rng));
            lines::pick(rng, lines::SUSPICIOUS_GIVE).into()
        }
        Persona::Nearsighted => {
            candy.push(random_treat(rng));
            candy.push(random_treat(rng));
            lines::pick(rng, lines::NEARSIGHTED).into()
        }
        Persona::Grumpy => {
            candy.push([Candy::Raisins, Candy::Pennies, Candy::Apple][rng.random_range(0..3)]);
            lines::pick(rng, lines::GRUMPY).into()
        }
        Persona::Dentist => {
            candy.push(Candy::Toothbrush);
            lines::pick(rng, lines::DENTIST).into()
        }
        Persona::FullSize => {
            candy.push(Candy::FullSize);
            lines::pick(rng, lines::FULL_SIZE).into()
        }
        Persona::Maple => {
            candy.push([Candy::MapleLeaf, Candy::SyrupJug, Candy::MapleCotton][rng.random_range(0..3)]);
            if rng.random_bool(0.4) {
                candy.push(Candy::MapleLeaf);
            }
            lines::pick(rng, lines::GIVE_MAPLE).into()
        }
        Persona::Cheese => {
            candy.push(Candy::Cheddar);
            lines::pick(rng, lines::GIVE_CHEESE).into()
        }
        Persona::Ethan => {
            candy.extend([Candy::FullSize, Candy::SyrupJug]);
            mats.push(Mat::Sprig);
            bd.say.clone().unwrap_or_else(|| "Huzzah!".into())
        }
        Persona::Frost => {
            mats.push(Mat::Bookmark);
            candy.push(random_treat(rng));
            lines::pick(rng, lines::FROST).into()
        }
        Persona::Coolidge => {
            // Maple, and cheddar: his father started the cheese factory in
            // Plymouth Notch.
            candy.extend([Candy::MapleLeaf, Candy::Cheddar]);
            lines::pick(rng, lines::COOLIDGE).into()
        }
        Persona::Mom => {
            candy.push(Candy::Hakarl);
            candy.push(Candy::Hakarl);
            bd.say.clone().unwrap_or_else(|| lines::pick(rng, lines::MOM).into())
        }
        Persona::Keeper => {
            match bd.gives.as_deref() {
                Some("chocolate") => candy.extend([Candy::Chocolate, Candy::Chocolate]),
                Some("bookmark") => {
                    mats.push(Mat::Bookmark);
                    candy.push(random_treat(rng));
                }
                Some("pamphlet") => mats.push(Mat::Pamphlet),
                Some("sticker") => {
                    mats.push(Mat::Sticker);
                    candy.push(random_treat(rng));
                }
                Some("tape") => mats.extend([Mat::Tape, Mat::Tape]),
                Some("popcorn") => candy.push(Candy::PopcornBall),
                Some("carob") => candy.extend([Candy::Carob, Candy::Carob]),
                Some("maple") => candy.extend([Candy::MapleLeaf, Candy::SyrupJug]),
                Some("hakarl") => candy.extend([Candy::Hakarl, Candy::Hakarl]),
                _ => {
                    for _ in 0..rng.random_range(1..3) {
                        candy.push(random_treat(rng));
                    }
                }
            }
            bd.say.clone().unwrap_or_else(|| lines::pick(rng, lines::GIVE).into())
        }
        Persona::Nobody => String::new(),
    };
    // Frost keeps to his verse and Coolidge to his silence.
    let chatty = !matches!(h.persona, Persona::Frost | Persona::Coolidge);
    let line = if chatty && h.visits > 0 && !again && h.last_seen.is_some() && g.rng.random_bool(0.5) {
        lines::pick(&mut g.rng, lines::NEW_GHOST).to_string()
    } else {
        line
    };
    Verdict::Give { candy, mats, line }
}

fn deliver(g: &mut Game, b: usize, v: &Verdict) {
    let town = g.town.clone();
    let bd = &town.buildings[b];
    let who = Who::Door(b as u32);
    let pos = bd.door.step();
    match v {
        Verdict::Give { candy, mats, line } => {
            // With the children along, what was for one is shared out: one
            // each. Shake them off and the raccoon gets it all.
            let along = kids_along(g, pos);
            let kids = along.iter().filter(|&&k| k).count();
            let mut candy = candy.clone();
            let share = kids.min(candy.len().saturating_sub(1));
            if share > 0 && g.houses[b].persona != Persona::Ethan {
                candy.truncate(candy.len() - share);
                // Coolidge shares it out too, but he does not say so.
                if g.houses[b].persona == Persona::Coolidge {
                    g.say(who, line.clone());
                } else {
                    g.say(who, format!("{line} One each!"));
                }
            } else {
                g.say(who, line.clone());
            }
            let candy = &candy;
            for c in candy {
                g.raccoon.bag.add(*c, 1);
            }
            for m in mats {
                g.raccoon.mats.add(*m, 1);
            }
            let given = candy.len() as f32;
            g.houses[b].supply = (g.houses[b].supply - given.max(1.0)).max(0.0);
            g.raccoon.cheer(0.04);
            g.events.push(SimEvent::Given { b, candy: candy.clone(), pos });
            g.discover(Id::FirstCandy);
            if candy.contains(&Candy::FullSize) {
                g.discover(Id::FullSizeBar);
            }
            if candy.contains(&Candy::Hakarl) {
                g.discover(Id::Hakarl);
            }
            if candy.contains(&Candy::Toothbrush) {
                g.discover(Id::Toothbrush);
            }
            if mats.contains(&Mat::Tape) {
                g.discover(Id::TapeFromHardware);
            }
            if g.houses[b].persona == Persona::Nearsighted {
                g.discover(Id::Nearsighted);
            }
            match bd.kind {
                BuildingKind::StateHouse => g.discover(Id::Republic),
                BuildingKind::Library => g.discover(Id::Library),
                BuildingKind::FireHouse => g.discover(Id::FireStation),
                BuildingKind::College => g.discover(Id::College),
                BuildingKind::Tower => g.discover(Id::EthanAllen),
                _ => {}
            }
            let mut names: Vec<&str> = candy.iter().map(|c| c.name()).collect();
            names.extend(mats.iter().map(|m| m.name()));
            if !names.is_empty() {
                g.say_message(format!("Got {}.", names.join(", ")));
            }
            // The children along with it get some too.
            let along = kids_along(g, pos);
            if g.houses[b].persona == Persona::Ethan {
                ethan_gifts(g, along);
                return;
            }
            if g.houses[b].persona == Persona::Coolidge {
                coolidge(g, b, along);
            }
            if g.houses[b].persona == Persona::Frost && along[1] && g.rng.random_bool(0.5) {
                g.say(Who::Skadi, "Wait. Was that ROBERT FROST?");
                if along[0] {
                    g.say(Who::Loki, "Who?");
                }
            }
            for (i, here) in along.into_iter().enumerate() {
                if here {
                    let c = if g.houses[b].persona == Persona::Mom { Candy::Hakarl } else { random_treat(&mut g.rng) };
                    super::kids::receive(g, i, c);
                    g.houses[b].supply = (g.houses[b].supply - 1.0).max(0.0);
                }
            }
        }
        Verdict::Refuse { line, scream } => {
            g.say(who, line.clone());
            g.raccoon.happy = (g.raccoon.happy - 0.03).max(0.0);
            g.events.push(SimEvent::Refused { b, pos });
            if *scream {
                g.events.push(SimEvent::Scream { pos });
                g.discover(Id::Screamed);
                alarm(g, pos, 0.18);
            } else {
                alarm(g, pos, 0.06);
            }
        }
        Verdict::Empty { line } | Verdict::Nobody { line } => g.say(who, line.clone()),
    }
}

/// Calvin Coolidge says nothing; the plaque by the door says the rest. Skaði
/// bets she can get more than two words out of him.
fn coolidge(g: &mut Game, b: usize, along: [bool; 2]) {
    if let Some(plaque) = g.town.buildings[b].say.clone() {
        g.say_message(plaque);
    }
    g.discover(Id::SilentCal);
    if along[1] && g.rng.random_bool(0.5) {
        let who = Who::Door(b as u32);
        g.say(Who::Skadi, "Mr. President! I bet I can get more than two words out of you.");
        g.say(who, "You lose.");
        if along[0] {
            g.say(Who::Loki, "He talked! He TALKED!");
        }
    }
}

/// Ethan Allen has something for the children, too.
fn ethan_gifts(g: &mut Game, along: [bool; 2]) {
    let who = Who::Door(g.town.buildings.iter().position(|b| b.kind == BuildingKind::Tower).unwrap_or(0) as u32);
    if along[0] && !g.kids[0].tricorn {
        g.kids[0].tricorn = true;
        g.kids[0].state = super::kids::State::Jump { t: 2.5 };
        g.say(who, "For the small dinosaur: a commission in the Green Mountain Boys! And a hat.");
        g.say(Who::Loki, "I'm a GREEN MOUNTAIN DINOSAUR!");
        let pos = g.kids[0].pos;
        g.events.push(SimEvent::Jump { who: Who::Loki, pos });
    }
    if along[1] && !g.kids[1].key {
        g.kids[1].key = true;
        g.kids[1].state = super::kids::State::Jump { t: 2.5 };
        g.say(Who::Skadi, "Is this... the actual key to Fort Ticonderoga?");
        let pos = g.kids[1].pos;
        g.events.push(SimEvent::Jump { who: Who::Skadi, pos });
        g.say_message("Ethan Allen: \"The very one! Well. A very one. Took the fort without a shot, kept the key.\"");
    }
    if (along[0] || along[1]) && g.kids[0].tricorn && g.kids[1].key {
        g.discover(Id::EthanGifts);
    }
}

/// A piece out of a household's bowl: what they would have handed out.
pub fn bowl_piece(persona: Persona, gives: Option<&str>, rng: &mut ChaCha8Rng) -> Candy {
    match (persona, gives) {
        (Persona::FullSize, _) => Candy::FullSize,
        (Persona::Maple, _) | (_, Some("maple")) => [Candy::MapleLeaf, Candy::SyrupJug, Candy::MapleCotton][rng.random_range(0..3)],
        (Persona::Cheese, _) => Candy::Cheddar,
        (Persona::Dentist, _) => Candy::Toothbrush,
        (Persona::Grumpy, _) => [Candy::Raisins, Candy::Pennies, Candy::Apple][rng.random_range(0..3)],
        (_, Some("chocolate")) => Candy::Chocolate,
        (_, Some("popcorn")) => Candy::PopcornBall,
        (_, Some("carob")) => Candy::Carob,
        _ => random_treat(rng),
    }
}

/// Word goes round that there is a raccoon about: doors near `at` get
/// warier for a while (or, for a negative `amount`, that there is a ghost
/// giving candy away: less wary).
pub fn alarm(g: &mut Game, at: Vec2, amount: f32) {
    let town = g.town.clone();
    for (h, b) in g.houses.iter_mut().zip(town.buildings.iter()) {
        let d = b.door.step().distance(at);
        if d < 16.0 {
            h.wary = (h.wary + amount * (1.0 - d / 16.0)).clamp(0.0, 0.45);
        }
    }
}

/// A rival trick-or-treater at the door: takes from the supply.
pub fn rival_takes(g: &mut Game, b: usize) -> u32 {
    let h = &mut g.houses[b];
    if !h.answers() || h.supply < 1.0 || h.cooldown > 0.0 {
        return 0;
    }
    let n: f32 = if h.persona == Persona::Generous { 2.0 } else { 1.0 };
    let n = n.min(h.supply.floor());
    h.supply -= n;
    h.cooldown = 1.5;
    n as u32
}
