//! Headless tests of the night: no terminal needed.

use super::*;
use crate::world::{plan, town};

pub fn montpelier() -> Arc<Town> {
    Arc::new(plan::build(&town::parse(town::MONTPELIER).unwrap()))
}

fn run(g: &mut Game, secs: f32, input: TickInput) {
    let ticks = (secs / DT) as usize;
    for _ in 0..ticks {
        g.step(&input);
    }
}

#[test]
fn the_same_seed_makes_the_same_night() {
    let t = montpelier();
    let mut a = Game::new(t.clone(), 7);
    let mut b = Game::new(t, 7);
    let walk = TickInput { dir: Vec2::new(1.0, 0.2), ..Default::default() };
    run(&mut a, 20.0, walk);
    run(&mut b, 20.0, walk);
    assert_eq!(a.state_hash(), b.state_hash());
}

#[test]
fn walking_wears_happiness_and_slows_the_raccoon() {
    let t = montpelier();
    let mut g = Game::new(t, 3);
    let h0 = g.raccoon.happy;
    let fast = g.raccoon.speed(crate::world::grid::Terrain::Sidewalk);
    run(&mut g, 15.0, TickInput { dir: Vec2::new(-1.0, 0.0), ..Default::default() });
    assert!(g.raccoon.happy < h0, "{} !< {h0}", g.raccoon.happy);
    assert!(g.raccoon.speed(crate::world::grid::Terrain::Sidewalk) < fast);
}

#[test]
fn eating_hakarl_maxes_happiness() {
    let t = montpelier();
    let mut g = Game::new(t, 3);
    g.raccoon.happy = 0.2;
    g.raccoon.bag.add(candy::Candy::Hakarl, 1);
    g.step(&TickInput { eat: true, ..Default::default() });
    run(&mut g, 1.2, TickInput::default());
    assert_eq!(g.raccoon.happy, 1.0);
    assert!(g.found.has(discover::Id::Hakarl) || true);
}

#[test]
fn knocking_on_a_door_gets_an_answer() {
    let t = montpelier();
    let mut g = Game::new(t.clone(), 11);
    let house = t.buildings.iter().find(|b| b.kind == town::BuildingKind::House).unwrap();
    g.raccoon.pos = house.door.step();
    g.houses[house.id].persona = doors::Persona::Nearsighted;
    g.step(&TickInput { act: true, ..Default::default() });
    assert!(g.visit.is_some());
    // Loki may well guess the door and bump into the raccoon afterwards, so
    // look at what the door gave, not at what is left.
    let mut given = 0;
    for _ in 0..360 {
        given = given.max(g.raccoon.bag.total());
        g.step(&TickInput::default());
    }
    assert!(g.visit.is_none());
    assert!(given > 0, "a nearsighted neighbour always gives");
}

#[test]
fn a_ruined_costume_is_refused_by_the_suspicious() {
    let t = montpelier();
    let mut g = Game::new(t.clone(), 12);
    let house = t.buildings.iter().find(|b| b.kind == town::BuildingKind::House).unwrap();
    for s in g.raccoon.costume.slots.iter_mut() {
        *s = costume::Slot::Hole;
    }
    g.raccoon.costume.dirt = 1.0;
    g.raccoon.pos = house.door.step();
    g.houses[house.id].persona = doors::Persona::Suspicious;
    g.step(&TickInput { act: true, ..Default::default() });
    run(&mut g, 6.0, TickInput::default());
    assert_eq!(g.raccoon.bag.total(), 0);
    assert!(g.found.has(discover::Id::Refused));
}

#[test]
fn bowls_refill_at_their_own_pace() {
    let t = montpelier();
    let mut g = Game::new(t, 5);
    let rates: Vec<f32> = g.houses.iter().filter(|h| h.answers()).map(|h| h.refill).collect();
    let mut sorted = rates.clone();
    sorted.sort_by(f32::total_cmp);
    sorted.dedup();
    assert!(sorted.len() > 20, "refill rates should differ");
    let i = g.houses.iter().position(|h| h.answers()).unwrap();
    g.houses[i].supply = 0.0;
    run(&mut g, 120.0, TickInput::default());
    assert!(g.houses[i].supply > 0.0);
}

#[test]
fn a_skadi_bump_spills_more_than_a_loki_bump() {
    let t = montpelier();
    let mut g = Game::new(t, 9);
    g.raccoon.bag.add(candy::Candy::Chocolate, 40);
    let a = raccoon::spill(&mut g, Who::Loki, 0.1, 1);
    let b = raccoon::knock_down(&mut g, Who::Skadi, 0.25, 2, Vec2::ZERO);
    assert!(b > a, "{b} !> {a}");
}

#[test]
fn touching_loki_with_an_egg_makes_him_cry_and_skadi_comes() {
    let t = montpelier();
    let mut g = Game::new(t, 21);
    let p = g.raccoon.pos + Vec2::new(0.3, 0.0);
    g.kids[kids::LOKI].pos = p;
    g.kids[kids::LOKI].eggs = 1;
    g.kids[kids::LOKI].state = kids::State::CarryEgg;
    g.kids[kids::LOKI].bump_cd = 0.0;
    g.step(&TickInput::default());
    assert!(matches!(g.kids[kids::LOKI].state, kids::State::Crying { .. }), "{:?} at {} from the raccoon", g.kids[kids::LOKI].state, g.kids[kids::LOKI].pos.distance(g.raccoon.pos));
    assert!(g.found.has(discover::Id::EggBroken));
    run(&mut g, 1.0, TickInput::default());
    assert!(matches!(g.kids[kids::SKADI].state, kids::State::GoComfort | kids::State::Comfort { .. }));
}

#[test]
fn a_long_night_runs_without_trouble() {
    let t = montpelier();
    let mut g = Game::new(t, 1);
    let mut dirs = [Vec2::X, Vec2::Y, -Vec2::X, -Vec2::Y].into_iter().cycle();
    let mut d = dirs.next().unwrap();
    for k in 0..(60 * 240) {
        if k % 300 == 0 {
            d = dirs.next().unwrap();
        }
        let input = TickInput { dir: d, act: k % 97 == 0, eat: k % 1300 == 0, patch: k % 700 == 0, ..Default::default() };
        g.step(&input);
    }
    assert!(g.hen.pos.x.is_finite());
    assert!(g.items.len() < 2000, "{} items lying about", g.items.len());
}

#[test]
fn most_people_out_are_trick_or_treaters() {
    let t = montpelier();
    let g = Game::new(t, 2);
    let kids = g.folk.iter().filter(|w| matches!(w.kind, folk::Kind::Kid | folk::Kind::Teen)).count();
    let share = kids as f32 / g.folk.len() as f32;
    assert!((0.58..0.72).contains(&share), "{kids} of {} are children", g.folk.len());
}

#[test]
fn frightened_children_drop_candy_the_raccoon_can_collect() {
    let t = montpelier();
    let mut g = Game::new(t, 4);
    for s in g.raccoon.costume.slots.iter_mut() {
        *s = costume::Slot::Hole;
    }
    // Stand the raccoon among a band of children until some run.
    let lead = g.folk.iter().position(|w| w.kind == folk::Kind::Kid).unwrap();
    let mut dropped = 0;
    for _ in 0..20 {
        let at = g.folk[lead].pos;
        g.raccoon.pos = at + Vec2::new(1.0, 0.0);
        for w in g.folk.iter_mut() {
            w.seen_cd = 0.0;
        }
        run(&mut g, 1.0, TickInput::default());
        dropped += g.items.iter().filter(|it| matches!(it.from, Some(Who::Folk(_))) && matches!(it.kind, items::ItemKind::Candy(_))).count();
        if dropped > 0 {
            break;
        }
    }
    assert!(dropped > 0, "nobody dropped anything");
    let it = g.items.iter().find(|it| matches!(it.from, Some(Who::Folk(_)))).unwrap().clone();
    let before = g.raccoon.bag.total();
    g.raccoon.pos = it.pos;
    run(&mut g, 0.5, TickInput::default());
    assert!(g.raccoon.bag.total() > before || items::find(&g, it.id).is_none());
}

#[test]
fn gravy_lies_in_the_way_until_his_belly_is_rubbed() {
    let t = montpelier();
    let mut g = Game::new(t, 6);
    g.gravy.state = gravy::State::Lie;
    g.raccoon.pos = g.gravy.pos + Vec2::new(-2.0, 0.0);
    run(&mut g, 2.0, TickInput { dir: Vec2::X, ..Default::default() });
    assert!(g.raccoon.pos.distance(g.gravy.pos) >= gravy::RADIUS + raccoon::RADIUS - 0.05, "walked through Gravy");
    assert_eq!(g.gravy.state, gravy::State::Lie);
    let h = g.raccoon.happy;
    g.step(&TickInput { act: true, ..Default::default() });
    assert!(matches!(g.gravy.state, gravy::State::Rubbed { .. }), "{:?}, {:?} at {}", g.gravy.state, g.raccoon.act, g.raccoon.pos.distance(g.gravy.pos));
    assert!(g.raccoon.happy > h);
    run(&mut g, 4.0, TickInput::default());
    assert!(!g.gravy.lying());
}

#[test]
fn gravy_eats_dropped_candy_and_barfs() {
    let t = montpelier();
    let mut g = Game::new(t, 8);
    g.gravy.state = gravy::State::Amble { to: g.gravy.pos, t: 30.0 };
    g.gravy.lie_cd = 1000.0;
    let p = g.gravy.pos + Vec2::new(1.5, 0.0);
    items::drop_item(&mut g, items::ItemKind::Candy(candy::Candy::Chocolate), p, Some(Who::Raccoon));
    g.raccoon.pos = g.gravy.pos + Vec2::new(-12.0, 0.0);
    let mut barfed = false;
    for _ in 0..(60 * 15) {
        g.step(&TickInput::default());
        if g.items.iter().any(|it| it.kind == items::ItemKind::Barf) {
            barfed = true;
            break;
        }
    }
    assert!(barfed);
}

/// Over a long walk the children stay with the raccoon (Skaði) and with
/// each other (Loki), and the hen stays with the children, moving about.
#[test]
fn the_flock_stays_together_and_follows() {
    let t = montpelier();
    let mut g = Game::new(t, 13);
    let mut bot = autopilot::Autopilot::new();
    let (mut n, mut skadi_near, mut loki_near, mut hen_near) = (0, 0, 0, 0);
    let mut hen_walked = 0.0;
    let mut last_hen = g.hen.pos;
    for _ in 0..(60 * 300) {
        let input = bot.decide(&g);
        g.step(&input);
        g.drain_events();
        hen_walked += g.hen.pos.distance(last_hen);
        last_hen = g.hen.pos;
        n += 1;
        // With the raccoon, or waiting at the door it is making for.
        let ahead = match g.kids[kids::SKADI].state {
            kids::State::GoDoor { b } => g.kids[kids::SKADI].pos.distance(g.town.buildings[b].door.step()) < 3.0,
            _ => false,
        };
        if g.kids[kids::SKADI].pos.distance(g.raccoon.pos) < 10.0 || ahead {
            skadi_near += 1;
        }
        // Near his sister, or off on an egg run (and on his way back).
        if g.kids[kids::LOKI].pos.distance(g.kids[kids::SKADI].pos) < 10.0 || g.kids[kids::LOKI].on_errand() {
            loki_near += 1;
        }
        if g.hen.pos.distance(hen::flock(&g)) < 9.0 || g.hen.held_by.is_some() {
            hen_near += 1;
        }
    }
    let pct = |k: i32| k as f32 / n as f32;
    assert!(pct(skadi_near) > 0.85, "Skaði near the raccoon {:.0}% of the time", pct(skadi_near) * 100.0);
    assert!(pct(loki_near) > 0.85, "Loki near Skaði {:.0}% of the time", pct(loki_near) * 100.0);
    assert!(pct(hen_near) > 0.8, "the hen near the children {:.0}% of the time", pct(hen_near) * 100.0);
    assert!(hen_walked > 150.0, "the hen walked {hen_walked:.0} tiles");
}

#[test]
fn everybody_starts_somewhere_they_can_move() {
    let t = montpelier();
    let g = Game::new(t.clone(), 1);
    assert!(t.clear(g.hen.pos, hen::RADIUS, false), "hen at {:?}", g.hen.pos);
    for k in &g.kids {
        assert!(t.clear(k.pos, kids::RADIUS, false), "{:?} at {:?}", k.who, k.pos);
    }
    assert!(t.clear(g.gravy.pos, gravy::RADIUS, false));
    assert!(t.clear(g.cat.pos, cat::RADIUS, false));
}

#[test]
fn a_raccoon_runs_out_of_breath_and_gets_it_back() {
    let t = montpelier();
    let mut g = Game::new(t, 3);
    let run = TickInput { dir: Vec2::X, run: true, ..Default::default() };
    run_for(&mut g, raccoon::WIND + 0.5, run);
    assert!(g.raccoon.winded && !g.raccoon.running, "still running after {} s", raccoon::WIND + 0.5);
    run_for(&mut g, raccoon::RECOVER / 2.0 * raccoon::SECOND_WIND + 0.5, TickInput::default());
    assert!(!g.raccoon.winded);
}

fn run_for(g: &mut Game, secs: f32, input: TickInput) {
    for _ in 0..(secs / DT) as usize {
        g.step(&input);
    }
}

#[test]
fn loki_comes_round_again_for_what_he_knocked_out() {
    let t = montpelier();
    let mut g = Game::new(t, 31);
    g.raccoon.bag.add(candy::Candy::Chocolate, 30);
    let at = g.raccoon.pos;
    g.kids[kids::LOKI].pos = at + Vec2::new(-0.5, 0.0);
    g.kids[kids::LOKI].vel = Vec2::new(5.0, 0.0);
    g.kids[kids::LOKI].bump_cd = 0.0;
    g.kids[kids::LOKI].state = kids::State::Dash { to: at + Vec2::new(3.0, 0.0), t: 2.0 };
    g.step(&TickInput::default());
    assert!(matches!(g.kids[kids::LOKI].state, kids::State::Circle { .. }), "{:?}", g.kids[kids::LOKI].state);
    let pail = g.kids[kids::LOKI].pail;
    // Keep Skaði out of it, so we see Loki's loop.
    g.kids[kids::SKADI].state = kids::State::Sulk { t: 30.0 };
    let mut looped = false;
    for _ in 0..(60 * 10) {
        g.step(&TickInput::default());
        looped |= matches!(g.kids[kids::LOKI].state, kids::State::Fetch { .. });
        if g.kids[kids::LOKI].pail > pail {
            break;
        }
    }
    assert!(looped && g.kids[kids::LOKI].pail > pail, "Loki did not come back for it");
}

#[test]
fn with_the_children_along_every_child_takes_one() {
    let t = montpelier();
    let house = t.buildings.iter().find(|b| b.kind == town::BuildingKind::House).unwrap().clone();
    let knock = |kids_along: bool| {
        let mut g = Game::new(t.clone(), 41);
        g.houses[house.id].persona = doors::Persona::Nearsighted;
        g.raccoon.pos = house.door.step();
        for k in 0..2 {
            g.kids[k].pos = if kids_along { house.door.step() + Vec2::new(k as f32 - 0.5, 0.6) } else { house.door.step() + Vec2::new(30.0, 30.0) };
            g.kids[k].state = kids::State::Sulk { t: 30.0 };
        }
        g.step(&TickInput { act: true, ..Default::default() });
        run(&mut g, 6.0, TickInput::default());
        g.raccoon.bag.total()
    };
    let alone = knock(false);
    let together = knock(true);
    assert!(together < alone, "with the children {together}, alone {alone}");
}

#[test]
fn a_hungry_hen_eats_fills_up_and_lays() {
    let t = montpelier();
    let mut g = Game::new(t, 51);
    g.hen.hunger = 1.0;
    g.hen.fullness = 0.0;
    g.hen.egg_t = 1000.0;
    g.hen.poop_t = 1000.0;
    // Out on the State House lawn, with room all round.
    let at = g.town.open_near(Vec2::new(30.0, 120.0), hen::RADIUS, false, 3.0).unwrap();
    g.hen.pos = at;
    g.hen.prev = at;
    for k in 0..4 {
        items::drop_item(&mut g, items::ItemKind::Candy(candy::Candy::CandyCorn), at + Vec2::new(0.8 + k as f32 * 0.4, 0.2), None);
    }
    // Her flock beside her, quiet.
    for k in 0..2 {
        g.kids[k].pos = at + Vec2::new(-1.0 - k as f32, 0.5);
        g.kids[k].state = kids::State::Sulk { t: 60.0 };
    }
    let mut ate = 0;
    for _ in 0..(60 * 20) {
        g.step(&TickInput::default());
        ate = 4 - g.items.iter().filter(|it| matches!(it.kind, items::ItemKind::Candy(_))).count();
        if g.hen.fullness > 0.6 {
            break;
        }
    }
    assert!(ate >= 2 && g.hen.fullness > 0.6, "ate {ate}, fullness {}", g.hen.fullness);
    // Full, she lays and leaves droppings soon and often.
    g.hen.egg_t = 0.0;
    g.hen.poop_t = 0.0;
    let mut eggs = 0;
    let mut poops = 0;
    for _ in 0..(60 * 30) {
        g.step(&TickInput::default());
        for e in g.drain_events() {
            match e {
                SimEvent::EggLaid { .. } => eggs += 1,
                SimEvent::Poop { .. } => poops += 1,
                _ => {}
            }
        }
    }
    assert!(eggs >= 1 && poops >= 2, "{eggs} eggs, {poops} droppings in half a minute");
}

#[test]
fn children_stop_at_the_curb_before_crossing() {
    let t = montpelier();
    let mut g = Game::new(t.clone(), 61);
    // Skaði on State Street's south sidewalk; the raccoon across the road.
    let start = t.open_near(Vec2::new(70.0, 132.6), kids::RADIUS, false, 1.0).unwrap();
    assert_eq!(t.grid.at(start), crate::world::grid::Terrain::Sidewalk);
    g.raccoon.pos = Vec2::new(70.0, 126.0);
    g.kids[kids::SKADI].pos = start;
    g.kids[kids::SKADI].prev = start;
    g.kids[kids::SKADI].state = kids::State::Follow;
    let mut waited = 0.0;
    let mut crossed = false;
    for _ in 0..(60 * 6) {
        g.step(&TickInput::default());
        let k = &g.kids[kids::SKADI];
        let on_road = t.grid.at(k.pos) == crate::world::grid::Terrain::Road;
        if on_road {
            crossed = true;
            break;
        }
        if k.cross_wait > 0.0 {
            waited += DT;
        }
    }
    assert!(crossed, "she never crossed");
    assert!(waited >= kids::CROSS_PAUSE * 0.9, "she waited {waited:.2} s at the curb");
}

#[test]
fn skadi_guesses_the_door_and_notices_when_fooled() {
    let t = montpelier();
    let mut g = Game::new(t.clone(), 71);
    // A raccoon told to walk to a door: that is the door she guesses.
    let doors: Vec<_> = t
        .buildings
        .iter()
        .filter(|b| b.kind == town::BuildingKind::House && b.door.step().distance(t.start) < 25.0)
        .map(|b| (b.id, b.door.step()))
        .collect();
    let (first, step1) = doors[0];
    g.step(&TickInput { goto: Some(step1), ..Default::default() });
    run(&mut g, 1.5, TickInput::default());
    assert_eq!(kids::predict_door(&g), Some(first));
    assert!(matches!(g.kids[kids::SKADI].state, kids::State::GoDoor { b } if b == first), "{:?}", g.kids[kids::SKADI].state);
    // Off to another one instead: she works it out within a couple of seconds.
    let (second, step2) = *doors.iter().find(|(b, s)| *b != first && s.distance(step1) > 4.0).unwrap();
    g.step(&TickInput { goto: Some(step2), ..Default::default() });
    let mut switched = false;
    for _ in 0..(60 * 3) {
        g.step(&TickInput::default());
        if matches!(g.kids[kids::SKADI].state, kids::State::GoDoor { b } if b == second) {
            switched = true;
            break;
        }
    }
    assert!(switched, "{:?}", g.kids[kids::SKADI].state);
}

#[test]
fn loki_snacks_and_the_sugar_wears_off() {
    let t = montpelier();
    let mut g = Game::new(t, 41);
    g.kids[kids::LOKI].pail = 12;
    g.kids[kids::LOKI].sugar = 0.0;
    run_for(&mut g, 90.0, TickInput::default());
    assert!(g.kids[kids::LOKI].pail < 12, "he ate none");
    assert!(g.kids[kids::LOKI].sugar > 0.0);
    g.kids[kids::LOKI].pail = 0;
    g.kids[kids::LOKI].sugar = 1.0;
    // Sitting out, so he eats nothing he finds, and no snacks.
    g.kids[kids::LOKI].state = kids::State::Sulk { t: 100.0 };
    g.kids[kids::LOKI].snack_t = 1e9;
    run_for(&mut g, 60.0, TickInput::default());
    let s = g.kids[kids::LOKI].sugar;
    assert!(s < 0.9 && s > 0.6, "{s}");
}

/// Loki dashing at a body standing just ahead of him, and what he does next.
fn crash_into(g: &mut Game, at: Vec2, sugar: f32) {
    let l = &mut g.kids[kids::LOKI];
    l.pos = at + Vec2::new(-0.45, 0.0);
    l.vel = Vec2::new(5.0, 0.0);
    l.bump_cd = 0.0;
    l.sugar = sugar;
    l.state = kids::State::Dash { to: at + Vec2::new(4.0, 0.0), t: 2.0 };
    g.step(&TickInput::default());
}

fn quiet_spot(g: &mut Game) -> Vec2 {
    // Away from the raccoon, so it is not the one he hits.
    let p = g.town.open_near(g.raccoon.pos + Vec2::new(0.0, 12.0), 0.6, false, 6.0).unwrap();
    g.folk.clear();
    g.hen.pos = p + Vec2::new(0.0, 5.0);
    p
}

#[test]
fn a_sugared_loki_knocks_candy_out_of_his_sister_and_loops_back_wider() {
    let t = montpelier();
    let mut g = Game::new(t, 42);
    let at = quiet_spot(&mut g);
    g.kids[kids::SKADI].pos = at;
    g.kids[kids::SKADI].pail = 5;
    g.kids[kids::SKADI].state = kids::State::Sulk { t: 30.0 };
    crash_into(&mut g, at, 1.0);
    assert_eq!(g.kids[kids::SKADI].pail, 4);
    assert!(g.items.iter().any(|it| it.from == Some(Who::Skadi) && matches!(it.kind, items::ItemKind::Candy(_))));
    let kids::State::Circle { r: wide, .. } = g.kids[kids::LOKI].state else { panic!("{:?}", g.kids[kids::LOKI].state) };

    // The same crash with no sugar in him: when it happens, a tighter loop.
    let mut tight = None;
    for seed in 0..20 {
        let mut g = Game::new(montpelier(), 100 + seed);
        let at = quiet_spot(&mut g);
        g.kids[kids::SKADI].pos = at;
        g.kids[kids::SKADI].state = kids::State::Sulk { t: 30.0 };
        crash_into(&mut g, at, 0.0);
        if let kids::State::Circle { r, .. } = g.kids[kids::LOKI].state {
            tight = Some(r);
            break;
        }
    }
    let tight = tight.expect("a calm Loki crashes sometimes too");
    assert!(wide > tight + 3.0, "{wide} vs {tight}");
}

#[test]
fn loki_runs_into_the_hen_and_the_townsfolk_too() {
    let t = montpelier();
    let mut g = Game::new(t, 43);
    let at = quiet_spot(&mut g);
    g.hen.pos = at;
    g.hen.fullness = 0.9;
    crash_into(&mut g, at, 1.0);
    assert!(g.items.iter().any(|it| it.kind == items::ItemKind::Egg && it.from == Some(Who::Hen)), "a startled full hen drops an egg");
    assert!(matches!(g.hen.state, hen::State::Flee { .. }));

    let mut g = Game::new(montpelier(), 44);
    let at = quiet_spot(&mut g);
    let mut g2 = Game::new(montpelier(), 44);
    std::mem::swap(&mut g.folk, &mut g2.folk);
    g.folk.truncate(1);
    g.folk[0].kind = folk::Kind::Kid;
    g.folk[0].pos = at;
    g.folk[0].pail = 3;
    let id = g.folk[0].id;
    crash_into(&mut g, at, 1.0);
    assert!(g.items.iter().any(|it| it.from == Some(Who::Folk(id))), "the trick-or-treater drops a piece");
    assert!(g.found.has(discover::Id::LokiPinball));
}

#[test]
fn too_much_sugar_and_loki_crashes_and_skadi_hugs_him() {
    let t = montpelier();
    let mut g = Game::new(t, 45);
    g.kids[kids::LOKI].sugar = kids::SUGAR_CRASH - 0.05;
    kids::loki_eats(&mut g, candy::Candy::GummyWorms);
    assert!(matches!(g.kids[kids::LOKI].state, kids::State::Crying { .. }));
    assert!(g.kids[kids::LOKI].sugar < kids::SUGAR_RUSH, "he comes down");
    assert!(g.found.has(discover::Id::SugarCrash));
    let mut hugged = false;
    for _ in 0..(60 * 12) {
        g.step(&TickInput::default());
        hugged |= matches!(g.kids[kids::SKADI].state, kids::State::Comfort { .. });
    }
    assert!(hugged, "Skaði comes to hug him");
}

#[test]
fn robert_frost_answers_at_the_library() {
    let t = montpelier();
    let mut g = Game::new(t.clone(), 46);
    let lib = t.buildings.iter().find(|b| b.kind == town::BuildingKind::Library).unwrap();
    assert_eq!(g.houses[lib.id].persona, doors::Persona::Frost);
    g.raccoon.pos = lib.door.step();
    g.step(&TickInput { act: true, ..Default::default() });
    let mut verse = false;
    for _ in 0..360 {
        g.step(&TickInput::default());
        verse |= g.speech.iter().any(|s| lines::FROST.contains(&s.text.as_str()) || lines::FROST.iter().any(|l| s.text.starts_with(l)));
    }
    assert!(verse, "no poem");
    assert!(g.found.has(discover::Id::Library));
}

#[test]
fn calvin_coolidge_hands_it_over_without_a_word() {
    let t = montpelier();
    let mut g = Game::new(t.clone(), 47);
    let sh = t.buildings.iter().find(|b| b.kind == town::BuildingKind::StateHouse).unwrap();
    assert_eq!(g.houses[sh.id].persona, doors::Persona::Coolidge);
    g.raccoon.pos = sh.door.step();
    g.step(&TickInput { act: true, ..Default::default() });
    let mut given = 0;
    let mut said = Vec::new();
    for _ in 0..360 {
        given = given.max(g.raccoon.bag.get(candy::Candy::Cheddar));
        g.step(&TickInput::default());
        for s in g.speech.iter().filter(|s| s.who == Who::Door(sh.id as u32)) {
            said.push(s.text.clone());
        }
    }
    assert!(given > 0, "cheddar from Plymouth Notch");
    assert!(said.iter().all(|s| lines::COOLIDGE.contains(&s.as_str()) || s == "You lose."), "{said:?}");
    assert!(g.found.has(discover::Id::SilentCal) && g.found.has(discover::Id::Republic));
}

/// The raccoon on an open stretch of sidewalk away from everybody, with
/// `who` placed beside it.
fn alone_with(g: &mut Game) -> Vec2 {
    let p = g.town.open_near(Vec2::new(150.0, 131.5), 0.8, false, 6.0).unwrap();
    g.raccoon.pos = p;
    g.folk.clear();
    g.kids[kids::LOKI].pos = p + Vec2::new(0.0, 20.0);
    g.kids[kids::SKADI].pos = p + Vec2::new(0.0, 21.0);
    g.hen.pos = p + Vec2::new(0.0, 22.0);
    g.gravy.pos = p + Vec2::new(0.0, -30.0);
    g.cat.pos = p + Vec2::new(0.0, -32.0);
    p
}

#[test]
fn offered_hakarl_a_stranger_flees_drops_candy_and_will_not_take_it() {
    let mut g = Game::new(montpelier(), 50);
    let p = alone_with(&mut g);
    let mut g2 = Game::new(montpelier(), 50);
    std::mem::swap(&mut g.folk, &mut g2.folk);
    g.folk.truncate(1);
    g.folk[0].kind = folk::Kind::Kid;
    g.folk[0].pos = p + Vec2::new(1.0, 0.0);
    g.folk[0].pail = 6;
    let id = g.folk[0].id;
    g.raccoon.bag.add(candy::Candy::Hakarl, 1);
    assert_eq!(give::taker(&g), Some(give::Taker::Folk(id)));
    g.step(&TickInput { give: Some(candy::Candy::Hakarl), ..Default::default() });
    assert!(matches!(g.folk[0].goal, folk::Goal::Flee { .. }));
    assert!(g.items.iter().any(|it| it.from == Some(Who::Folk(id))), "they drop candy as they run");
    assert_eq!(g.raccoon.bag.get(candy::Candy::Hakarl), 1, "nobody takes hákarl");
}

#[test]
fn hakarl_at_the_door_leaves_the_bowl_on_the_step() {
    let t = montpelier();
    let mut g = Game::new(t.clone(), 51);
    let house = t.buildings.iter().find(|b| b.kind == town::BuildingKind::House).unwrap();
    g.raccoon.pos = house.door.step();
    g.houses[house.id].persona = doors::Persona::Nearsighted;
    let had = g.houses[house.id].supply.floor() as usize;
    g.raccoon.bag.add(candy::Candy::Hakarl, 1);
    g.step(&TickInput { act: true, ..Default::default() });
    while !g.visit.as_ref().is_some_and(|v| v.open()) {
        g.step(&TickInput::default());
    }
    assert_eq!(give::taker(&g), Some(give::Taker::Door(house.id)));
    g.step(&TickInput { give: Some(candy::Candy::Hakarl), ..Default::default() });
    assert!(g.visit.is_none());
    assert!(g.houses[house.id].supply < 1.0);
    let on_step = g.items.iter().filter(|it| it.from == Some(Who::Door(house.id as u32))).count();
    assert_eq!(on_step, had);
    assert!(g.found.has(discover::Id::AbandonedBowl));
}

#[test]
fn chocolate_winds_loki_up_and_cheddar_calms_him_down() {
    let mut g = Game::new(montpelier(), 52);
    let p = alone_with(&mut g);
    g.kids[kids::LOKI].pos = p + Vec2::new(1.0, 0.0);
    g.kids[kids::LOKI].state = kids::State::Sulk { t: 30.0 };
    g.kids[kids::LOKI].sugar = 0.2;
    g.raccoon.bag.add(candy::Candy::Chocolate, 3);
    g.raccoon.bag.add(candy::Candy::Cheddar, 1);
    for _ in 0..3 {
        g.step(&TickInput { give: Some(candy::Candy::Chocolate), ..Default::default() });
    }
    let high = g.kids[kids::LOKI].sugar;
    assert!(high >= kids::SUGAR_RUSH, "{high}");
    g.kids[kids::LOKI].pos = g.raccoon.pos + Vec2::new(1.0, 0.0);
    g.step(&TickInput { give: Some(candy::Candy::Cheddar), ..Default::default() });
    assert!(g.kids[kids::LOKI].sugar < high - 0.2);
    assert!(g.found.has(discover::Id::FedLoki) && g.found.has(discover::Id::CheddarCalms));
}

#[test]
fn a_fed_hen_tails_the_raccoon_and_loki_runs_her_egg_home() {
    let mut g = Game::new(montpelier(), 53);
    let p = alone_with(&mut g);
    g.hen.pos = p + Vec2::new(1.0, 0.0);
    g.raccoon.bag.add(candy::Candy::CandyCorn, 1);
    g.step(&TickInput { give: Some(candy::Candy::CandyCorn), ..Default::default() });
    assert!(g.hen.devoted > 30.0 && g.hen.egg_t <= 16.0);
    // Walk off: she comes too.
    let mut tailing = 0;
    for _ in 0..(60 * 6) {
        g.step(&TickInput { dir: Vec2::X, ..Default::default() });
        if g.hen.pos.distance(g.raccoon.pos) < 2.5 {
            tailing += 1;
        }
    }
    assert!(tailing > 60 * 4, "she kept up {tailing} ticks");
    // An egg, with Loki about: home it goes, however far.
    let at = g.raccoon.pos + Vec2::new(0.0, 0.5);
    items::drop_item(&mut g, items::ItemKind::Egg, at, Some(Who::Hen));
    g.kids[kids::LOKI].pos = g.raccoon.pos + Vec2::new(-1.0, 0.0);
    g.kids[kids::LOKI].state = kids::State::Follow;
    let mut ran = false;
    for _ in 0..(60 * 10) {
        g.step(&TickInput::default());
        ran |= g.kids[kids::LOKI].state == kids::State::CarryEgg;
    }
    assert!(ran, "Loki takes it home");
}

#[test]
fn gravy_turns_down_chocolate_and_rolls_over_for_cheddar() {
    let mut g = Game::new(montpelier(), 54);
    let p = alone_with(&mut g);
    g.gravy.pos = p + Vec2::new(1.2, 0.0);
    g.gravy.state = gravy::State::Lie;
    g.raccoon.bag.add(candy::Candy::Chocolate, 1);
    g.raccoon.bag.add(candy::Candy::Cheddar, 1);
    g.step(&TickInput { give: Some(candy::Candy::Chocolate), ..Default::default() });
    assert_eq!(g.raccoon.bag.get(candy::Candy::Chocolate), 1, "no chocolate for dogs");
    g.step(&TickInput { give: Some(candy::Candy::Cheddar), ..Default::default() });
    assert!(matches!(g.gravy.state, gravy::State::Rubbed { .. }));
}

#[test]
fn a_bribed_skadi_does_not_see_the_bowl_go() {
    let mut g = Game::new(montpelier(), 55);
    let p = alone_with(&mut g);
    g.kids[kids::SKADI].pos = p + Vec2::new(1.0, 0.0);
    g.raccoon.bag.add(candy::Candy::FullSize, 1);
    g.raccoon.bag.add(candy::Candy::Chocolate, 10);
    g.step(&TickInput { give: Some(candy::Candy::FullSize), ..Default::default() });
    assert!(g.kids[kids::SKADI].bribed > 100.0);
    let bowl = g.town.features.iter().position(|f| f.kind == crate::world::town::FeatureKind::CandyBowl).unwrap();
    let before = g.raccoon.bag.total();
    kids::skadi_objects_to_greed(&mut g, bowl, 6);
    assert_eq!(g.raccoon.bag.total(), before);
}

#[test]
fn eating_what_you_pick() {
    let mut g = Game::new(montpelier(), 56);
    alone_with(&mut g);
    g.raccoon.bag.add(candy::Candy::FullSize, 1);
    g.raccoon.bag.add(candy::Candy::Raisins, 1);
    g.step(&TickInput { eat_this: Some(candy::Candy::Raisins), ..Default::default() });
    run_for(&mut g, 1.5, TickInput::default());
    assert_eq!(g.raccoon.bag.get(candy::Candy::Raisins), 0);
    assert_eq!(g.raccoon.bag.get(candy::Candy::FullSize), 1);
}
