//! A raccoon that plays itself: for the title screen, `--autoplay`, and the
//! headless runs. It goes door to door (the nearest lit porch it has not
//! tried), eats when it is unhappy, patches its sheet when it can, and rubs
//! Gravy's belly when he is in the way.

use super::{Game, TickInput, raccoon::Act};
use crate::world::town::BuildingKind;
use glam::Vec2;
use std::collections::BTreeSet;

#[derive(Clone, Debug, Default)]
pub struct Autopilot {
    target: Option<usize>,
    tried: BTreeSet<usize>,
    stuck: f32,
    last_pos: Vec2,
    wait: u32,
}

impl Autopilot {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn decide(&mut self, g: &Game) -> TickInput {
        let mut input = TickInput::default();
        let r = &g.raccoon;
        if r.busy() || g.visit.is_some() {
            return input;
        }
        if self.wait > 0 {
            self.wait -= 1;
            return input;
        }
        if r.happy < 0.35 && !r.bag.is_empty() && r.belly < super::raccoon::BELLY_FULL * 0.8 {
            input.eat = true;
            return input;
        }
        if r.costume.holes() > 0 && !r.mats.is_empty() {
            input.patch = true;
            return input;
        }
        if g.gravy.lying() && g.gravy.pos.distance(r.pos) < 1.6 {
            input.act = true;
            return input;
        }
        let town = &g.town;
        if self.target.is_none() {
            if self.tried.len() > 30 {
                self.tried.clear();
            }
            self.target = town
                .buildings
                .iter()
                .filter(|b| g.houses[b.id].light && g.houses[b.id].answers() && b.kind != BuildingKind::Tower && !self.tried.contains(&b.id))
                .min_by(|a, b| a.door.step().distance(r.pos).total_cmp(&b.door.step().distance(r.pos)))
                .map(|b| b.id);
            if let Some(b) = self.target {
                input.goto = Some(town.buildings[b].door.step());
                self.stuck = 0.0;
                self.last_pos = r.pos;
            }
            return input;
        }
        let b = self.target.expect("checked");
        let step = town.buildings[b].door.step();
        if r.pos.distance(step) < 0.9 {
            input.act = true;
            self.tried.insert(b);
            self.target = None;
            self.wait = 30;
            return input;
        }
        // Walked off its path (bumped, tripped): ask again.
        if r.path.is_empty() {
            input.goto = Some(step);
        }
        self.stuck += super::DT;
        if self.stuck > 2.0 {
            if r.pos.distance(self.last_pos) < 0.5 {
                self.tried.insert(b);
                self.target = None;
            }
            self.stuck = 0.0;
            self.last_pos = r.pos;
        }
        input
    }

    /// Whether the raccoon is at rest (for tests).
    pub fn idle(g: &Game) -> bool {
        g.raccoon.act == Act::Free && g.visit.is_none()
    }
}
