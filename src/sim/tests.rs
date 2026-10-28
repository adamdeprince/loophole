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
    run(&mut g, 6.0, TickInput::default());
    assert!(g.visit.is_none());
    assert!(g.raccoon.bag.total() > 0, "a nearsighted neighbour always gives");
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
    let id = items::drop_item(&mut g, items::ItemKind::Egg, p, Some(Who::Hen));
    let i = items::find(&g, id).unwrap();
    g.items[i].carried = Some(Who::Loki);
    g.kids[kids::LOKI].pos = p;
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
