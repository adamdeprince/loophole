//! Giving things away. The raccoon picks a piece from the bag and offers it
//! to whoever is in reach, and what happens depends on who and what.
//!
//! Hákarl is the interesting one. Loki, Skaði, the hen and the cat love it.
//! Their mother and Ethan Allen take it gladly, and Calvin Coolidge eats it
//! without a flicker. Anybody else recoils and runs, dropping candy as they
//! go; a householder runs indoors and leaves the bowl tipped over on the
//! step, with nobody watching it. Gravy sniffs it and is suddenly not
//! hungry, and gets out of the way. Carrying hákarl frightens nobody:
//! offering it does, and they will not take it, so it goes back in the bag
//! for next time.
//!
//! Anything else is a present. Loki eats it on the spot (and the sugar shows:
//! chocolate is the quickest way to set him off), Skaði saves it, the
//! townsfolk are touched (and the doors nearby hear about the ghost who gives
//! candy away), a householder gives two back the first time, Gravy wolfs it
//! down (and it comes back up), the hen gobbles it, and the cat is not
//! interested.

use super::candy::Candy;
use super::discover::Id;
use super::doors::{self, Persona};
use super::folk::{Goal, Kind};
use super::items::{self, ItemKind};
use super::kids::{self, LOKI, SKADI};
use super::raccoon::Act;
use super::{Game, SimEvent, Who, gravy, hen, lines};
use glam::Vec2;
use rand::RngExt;

/// How close somebody has to be to be handed something, in tiles.
pub const REACH: f32 = 1.8;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Taker {
    Loki,
    Skadi,
    Folk(u32),
    Gravy,
    Hen,
    Cat,
    /// Whoever answered the door the raccoon is at.
    Door(usize),
}

/// Who the raccoon would hand something to now: whoever answered the door
/// it is at, or else the nearest within reach.
pub fn taker(g: &Game) -> Option<Taker> {
    if let Some(v) = g.visit.as_ref().filter(|v| v.open()) {
        return Some(Taker::Door(v.b));
    }
    let r = g.raccoon.pos;
    let mut best: Option<(Taker, f32)> = None;
    let mut consider = |t: Taker, p: Vec2| {
        let d = p.distance(r);
        if d < REACH && best.is_none_or(|(_, b)| d < b) {
            best = Some((t, d));
        }
    };
    consider(Taker::Loki, g.kids[LOKI].pos);
    consider(Taker::Skadi, g.kids[SKADI].pos);
    for w in g.folk.iter().filter(|w| w.kind != Kind::Dog) {
        consider(Taker::Folk(w.id), w.pos);
    }
    consider(Taker::Gravy, g.gravy.pos);
    if g.hen.held_by.is_none() {
        consider(Taker::Hen, g.hen.pos);
    }
    if !g.cat.hidden() {
        consider(Taker::Cat, g.cat.pos);
    }
    best.map(|(t, _)| t)
}

/// What to call them on the menu and the hint line.
pub fn name(g: &Game, t: Taker) -> String {
    match t {
        Taker::Loki => "Loki".into(),
        Taker::Skadi => "Skaði".into(),
        Taker::Folk(id) => g.folk.iter().find(|w| w.id == id).map_or("somebody", |w| w.kind.label()).into(),
        Taker::Gravy => "Gravy".into(),
        Taker::Hen => "Gullinkombi".into(),
        Taker::Cat => "the black cat".into(),
        Taker::Door(b) => match g.houses[b].persona {
            Persona::Mom => "Mom".into(),
            Persona::Ethan => "Ethan Allen".into(),
            Persona::Frost => "Robert Frost".into(),
            Persona::Coolidge => "Calvin Coolidge".into(),
            _ => "whoever is at the door".into(),
        },
    }
}

/// Offers `c` to whoever is in reach.
pub fn give(g: &mut Game, c: Candy) {
    let Some(t) = taker(g) else {
        g.say_message("Nobody close enough to give anything to.");
        return;
    };
    if !g.raccoon.bag.take(c) {
        return;
    }
    // Whether they kept it; if not it goes back in the bag.
    let kept = if c == Candy::Hakarl { hakarl(g, t) } else { present(g, t, c) };
    if !kept {
        g.raccoon.bag.add(c, 1);
    }
}

/// Where the one being offered something is.
fn pos_of(g: &Game, t: Taker) -> Vec2 {
    match t {
        Taker::Loki => g.kids[LOKI].pos,
        Taker::Skadi => g.kids[SKADI].pos,
        Taker::Folk(id) => g.folk.iter().find(|w| w.id == id).map_or(g.raccoon.pos, |w| w.pos),
        Taker::Gravy => g.gravy.pos,
        Taker::Hen => g.hen.pos,
        Taker::Cat => g.cat.pos,
        Taker::Door(b) => g.town.buildings[b].door.at,
    }
}

fn hakarl(g: &mut Game, t: Taker) -> bool {
    let at = pos_of(g, t);
    match t {
        Taker::Loki | Taker::Skadi => {
            let i = if t == Taker::Loki { LOKI } else { SKADI };
            let who = g.kids[i].who;
            g.kids[i].state = kids::State::Jump { t: 2.2 };
            let line = lines::pick(&mut g.rng, lines::JOY);
            g.say(who, line);
            g.events.push(SimEvent::Jump { who, pos: at });
            g.events.push(SimEvent::Loved { pos: at });
            g.discover(Id::KidsLoveHakarl);
            g.say_message(format!("{} eats the hákarl and jumps for joy.", name(g, t)));
            true
        }
        Taker::Folk(id) => {
            let Some(i) = g.folk.iter().position(|w| w.id == id) else { return false };
            let line = if g.folk[i].kind.is_child() {
                lines::pick(&mut g.rng, lines::RIVAL_HAKARL)
            } else {
                lines::pick(&mut g.rng, lines::ADULT_HAKARL)
            };
            g.say(Who::Folk(id), line);
            let from = g.raccoon.pos;
            g.folk[i].goal = Goal::Flee { from, t: 5.0 };
            super::folk::spill_pail(g, i, from);
            g.events.push(SimEvent::Flee { pos: at });
            g.discover(Id::HakarlRecoil);
            false
        }
        Taker::Door(b) => match g.houses[b].persona {
            Persona::Mom => {
                g.say(Who::Door(b as u32), "Giving me my own hákarl? Oh, sweetie. Have two!");
                g.raccoon.bag.add(Candy::Hakarl, 2);
                true
            }
            Persona::Ethan => {
                g.say(Who::Door(b as u32), "HUZZAH! Provisions fit for a Viking!");
                true
            }
            Persona::Coolidge => {
                g.say(Who::Door(b as u32), "...");
                g.say_message("Calvin Coolidge eats the hákarl. His expression does not change.");
                true
            }
            _ => {
                abandon_bowl(g, b);
                false
            }
        },
        Taker::Gravy => {
            g.say(Who::Gravy, "*sniff* ...*sniff*");
            g.gravy.state = gravy::State::Content { t: 40.0 };
            g.say_message("Gravy sniffs the hákarl and is suddenly not hungry. He gets up and wanders off.");
            g.discover(Id::GravyNotHungry);
            false
        }
        Taker::Hen => {
            hen::fed(g, 0.5, 40.0);
            g.say(Who::Hen, "BAWK!");
            g.say_message("Gullinkombi gulps the hákarl down. She is, after all, an Icelandic hen.");
            true
        }
        Taker::Cat => {
            charm_cat(g);
            g.say_message("The black cat eats the hákarl and looks at you with new respect.");
            true
        }
    }
}

/// Offered hákarl at the door, the householder runs indoors, and the bowl
/// goes over on the step with everything in it.
fn abandon_bowl(g: &mut Game, b: usize) {
    let line = lines::pick(&mut g.rng, lines::DOOR_HAKARL);
    g.say(Who::Door(b as u32), line);
    let bd = g.town.buildings[b].clone();
    let persona = g.houses[b].persona;
    let n = g.houses[b].supply.floor() as u32;
    let out = bd.door.facing.vec();
    let step = bd.door.step();
    for _ in 0..n {
        let c = doors::bowl_piece(persona, bd.gives.as_deref(), &mut g.rng);
        let a: f32 = g.rng.random_range(-1.3..1.3);
        let dir = Vec2::new(out.x * a.cos() - out.y * a.sin(), out.x * a.sin() + out.y * a.cos());
        let id = g.next_item;
        g.next_item += 1;
        let vel = dir * g.rng.random_range(0.4..2.2);
        g.items.push(items::Item::new(id, ItemKind::Candy(c), step, vel, Some(Who::Door(b as u32))));
    }
    let h = &mut g.houses[b];
    h.supply = 0.0;
    h.light = false;
    h.cooldown = 90.0;
    g.visit = None;
    g.raccoon.act = Act::Free;
    g.events.push(SimEvent::Scream { pos: bd.door.at });
    g.events.push(SimEvent::CandyDropped { pos: step, n, by: Who::Door(b as u32) });
    g.events.push(SimEvent::LightOut { b });
    g.discover(Id::AbandonedBowl);
    g.say_message(if n > 0 {
        format!("They flee indoors. The bowl goes over: {n} pieces on the step, and nobody watching.")
    } else {
        "They flee indoors. Their bowl was empty anyway.".to_string()
    });
}

fn present(g: &mut Game, t: Taker, c: Candy) -> bool {
    let at = pos_of(g, t);
    let kept = match t {
        Taker::Loki => to_loki(g, c, at),
        Taker::Skadi => to_skadi(g, c),
        Taker::Folk(id) => to_folk(g, id, c, at),
        Taker::Door(b) => door_present(g, b),
        Taker::Gravy => to_gravy(g, c),
        Taker::Hen => to_hen(g, c),
        Taker::Cat => to_cat(g, c),
    };
    if kept {
        g.events.push(SimEvent::Gift { pos: at });
    }
    kept
}

/// Chocolate is bad for dogs and birds; raisins are bad for dogs.
fn chocolate(c: Candy) -> bool {
    matches!(c, Candy::Chocolate | Candy::FullSize | Candy::PeanutCup | Candy::Caramel)
}

/// Loki eats it there and then, and what it does depends on what it was:
/// see [`kids::sugar_of`]. Caramel glues his teeth shut for a while; a
/// lollipop keeps him content; cheddar calms him down; fruit and raisins
/// are not candy, and he says so.
fn to_loki(g: &mut Game, c: Candy, at: Vec2) -> bool {
    let line = match c {
        Candy::Toothbrush => {
            g.say(Who::Loki, "Ew. A toothbrush.");
            items::drop_item(g, ItemKind::Candy(c), at, Some(Who::Loki));
            return true;
        }
        Candy::Pennies => {
            g.say(Who::Loki, "MONEY!");
            g.kids[LOKI].pail = g.kids[LOKI].pail.saturating_add(1);
            return true;
        }
        Candy::Apple | Candy::Raisins | Candy::Carob => {
            g.say(Who::Loki, "That's not CANDY.");
            return false;
        }
        Candy::Chocolate | Candy::PeanutCup => "CHOCOLATE! I mean, THANK YOU!",
        Candy::FullSize => "A WHOLE ONE?! FOR ME?!",
        Candy::SyrupJug => "*glug glug glug* AHHH!",
        Candy::MapleCotton | Candy::MapleLeaf => "Maple! *crunch*",
        Candy::CiderDonut => "DONUT!",
        Candy::Caramel => "Thanks! *chew* *chew* Mmf?",
        Candy::Lollipop => "A lollipop! *lick*",
        Candy::GummyWorms => "WORMS! Gullinkombi, look, WORMS!",
        Candy::Cheddar => "Cheese! *munch* ...I feel sleepy.",
        _ => "Thanks! *munch munch*",
    };
    g.say(Who::Loki, line);
    let calming = kids::sugar_of(c) < 0.0 && g.kids[LOKI].sugar > 0.4;
    kids::loki_eats(g, c);
    if kids::sugar_of(c) >= 0.15 {
        g.discover(Id::FedLoki);
    }
    if calming {
        g.discover(Id::CheddarCalms);
    }
    // Worms: off to show the hen.
    if c == Candy::GummyWorms && g.hen.held_by.is_none() && matches!(g.kids[LOKI].state, kids::State::Follow | kids::State::Wander { .. }) {
        g.kids[LOKI].state = kids::State::ChaseHen { t: 8.0 };
    }
    true
}

/// Skaði saves it for later. Something good (a fun-size chocolate or
/// better) buys a minute or so of her not noticing a whole bowl going into
/// the bag; a full-size bar, three.
fn to_skadi(g: &mut Game, c: Candy) -> bool {
    let (line, bribe) = match c {
        Candy::Toothbrush => ("Ha. Thanks, I guess.", 0.0),
        Candy::Pennies => ("...Thanks?", 0.0),
        Candy::FullSize => ("A FULL-SIZE? You're the best. I owe you.", 180.0),
        Candy::Apple => ("Thanks, actually. I like apples.", 0.0),
        Candy::Raisins | Candy::Carob => ("Oh. Thanks.", 0.0),
        _ if c.value() >= 10 => ("Ooh, thanks! I'll save it for later.", 60.0),
        _ => ("Thanks! I'll save it for later.", 0.0),
    };
    g.say(Who::Skadi, line);
    let k = &mut g.kids[SKADI];
    k.pail = k.pail.saturating_add(1);
    k.bribed = k.bribed.max(bribe);
    true
}

/// The townsfolk are touched, and word goes round: the doors nearby are less
/// wary of a ghost who gives candy away. A toothbrush touches nobody.
fn to_folk(g: &mut Game, id: u32, c: Candy, at: Vec2) -> bool {
    let Some(i) = g.folk.iter().position(|w| w.id == id) else { return false };
    let child = g.folk[i].kind.is_child();
    let (line, kindness) = match (c, child) {
        (Candy::Toothbrush, true) => ("...Thanks?", 0.0),
        (Candy::Toothbrush, false) => ("Ah. A fellow dentist.", 0.0),
        (Candy::Raisins | Candy::Apple | Candy::Carob, true) => ("Raisins. Great. Thanks.", 0.05),
        (Candy::FullSize, true) => ("A FULL-SIZE?! You're the best ghost EVER!", 0.3),
        (_, true) => ("Whoa! Thanks, ghost!", 0.15),
        (Candy::SyrupJug | Candy::MapleLeaf | Candy::MapleCotton, false) => ("Maple! Now that's a Vermont ghost.", 0.2),
        (Candy::Cheddar, false) => ("Cabot country! Thank you, ghost.", 0.2),
        (_, false) => ("For me? Well, aren't you sweet.", 0.15),
    };
    g.say(Who::Folk(id), line);
    g.folk[i].pail = g.folk[i].pail.saturating_add(1);
    if kindness > 0.0 {
        super::doors::alarm(g, at, -kindness);
        g.discover(Id::Generous);
    }
    true
}

/// Gravy eats most things, and brings most of them back up. Cheddar he
/// loves more than a belly rub: he rolls over, and then gets out of the
/// way. Chocolate and raisins he leaves alone (they are bad for dogs).
fn to_gravy(g: &mut Game, c: Candy) -> bool {
    if chocolate(c) || matches!(c, Candy::Raisins | Candy::Toothbrush | Candy::Pennies | Candy::Lollipop) {
        g.say(Who::Gravy, "*sniff*");
        g.say_message("Gravy sniffs it and looks away. Good dog: that one's bad for dogs.");
        return false;
    }
    if c == Candy::Cheddar {
        g.say(Who::Gravy, "*CHOMP* *happy groan*");
        g.gravy.state = gravy::State::Rubbed { t: 2.5 };
        g.discover(Id::GravyCheddar);
        g.say_message("Gravy rolls over in pure joy, then gets up and lets you by.");
        return true;
    }
    g.say(Who::Gravy, "*CHOMP*");
    g.gravy.state = gravy::State::Eat { t: 1.0 };
    true
}

/// The hen eats it and follows the raccoon about for a while, under its
/// feet, wanting more, and lays sooner (Loki will want that egg). Corn and
/// worms are what she likes best. No chocolate for birds.
fn to_hen(g: &mut Game, c: Candy) -> bool {
    let (fill, devotion) = match c {
        Candy::CandyCorn => (0.35, 50.0),
        Candy::GummyWorms => (0.3, 45.0),
        Candy::PopcornBall => (0.4, 40.0),
        Candy::Raspberry | Candy::Apple | Candy::Raisins => (0.25, 30.0),
        Candy::Cheddar | Candy::CiderDonut | Candy::Carob => (0.3, 30.0),
        Candy::MapleLeaf | Candy::MapleCotton => (0.2, 25.0),
        _ => {
            g.say(Who::Hen, "Bok.");
            g.say_message("Gullinkombi pecks it once and walks off. Not for hens.");
            return false;
        }
    };
    hen::fed(g, fill, devotion);
    let line = if c == Candy::CandyCorn { "BOK BOK BAGAWK!" } else { "Bok bok!" };
    g.say(Who::Hen, line);
    true
}

/// The cat wants nothing but cheese (and fish). Fed, she is too pleased
/// with herself to cross the raccoon's path for a while.
fn to_cat(g: &mut Game, c: Candy) -> bool {
    if c != Candy::Cheddar {
        g.say_message("The cat sniffs it and walks off. Cats.");
        return false;
    }
    charm_cat(g);
    g.say_message("The black cat eats the cheddar, purrs, and decides you are not worth crossing.");
    true
}

/// A fed cat crosses nobody's path for a few minutes.
fn charm_cat(g: &mut Game) {
    g.cat.state = super::cat::State::Sit { t: 6.0 };
    g.cat.cross_cd = g.cat.cross_cd.max(180.0);
    g.say(Who::Cat, "*prrrrrr*");
}

/// A present for whoever answered the door: the first time, they are so
/// touched they give two back.
fn door_present(g: &mut Game, b: usize) -> bool {
    let who = Who::Door(b as u32);
    let persona = g.houses[b].persona;
    match persona {
        Persona::Mom => g.say(who, "Aww, thank you, sweetie."),
        Persona::Coolidge => g.say(who, "*nods*"),
        Persona::Frost => g.say(who, "\"Good fences make good neighbors.\" So do good ghosts."),
        Persona::Ethan => g.say(who, "A tribute! The Green Mountain Boys thank you."),
        _ if !g.houses[b].touched => {
            g.houses[b].touched = true;
            g.houses[b].wary = 0.0;
            g.say(who, "A ghost giving ME candy? Well, I never! Here, take two.");
            let gives = g.town.buildings[b].gives.clone();
            for _ in 0..2 {
                let piece = doors::bowl_piece(persona, gives.as_deref(), &mut g.rng);
                g.raccoon.bag.add(piece, 1);
            }
            g.houses[b].supply = (g.houses[b].supply - 2.0).max(0.0);
            g.discover(Id::Generous);
        }
        _ => {
            g.say(who, "That's sweet of you, but you keep it.");
            return false;
        }
    }
    true
}
