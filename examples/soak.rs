//! A long night of random input, headless: checks nothing panics, nothing
//! piles up, and the tick stays quick. `cargo run --release --example soak -- MINUTES SEED`
use glam::Vec2;
use rand::{RngExt, SeedableRng};
use silkie_loophole::sim::{Game, TickInput};
use silkie_loophole::world::{plan, town};
use std::sync::Arc;
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let minutes: f32 = a.get(1).and_then(|s| s.parse().ok()).unwrap_or(40.0);
    let seed: u64 = a.get(2).and_then(|s| s.parse().ok()).unwrap_or(1);
    let t = Arc::new(plan::build(&town::parse(town::MONTPELIER).unwrap()));
    let mut g = Game::new(t.clone(), seed);
    let mut rng = rand_chacha::ChaCha8Rng::seed_from_u64(seed);
    let mut dir = Vec2::ZERO;
    let mut max = std::time::Duration::ZERO;
    let mut peak_items = 0;
    let ticks = (minutes * 60.0 * 60.0) as u64;
    for k in 0..ticks {
        if k % 40 == 0 {
            dir = match rng.random_range(0..6) {
                0 => Vec2::ZERO,
                _ => Vec2::new(rng.random_range(-1.0..1.0), rng.random_range(-1.0..1.0)).normalize_or_zero(),
            };
        }
        let mut input = TickInput { dir, run: rng.random_bool(0.3), ..Default::default() };
        match rng.random_range(0..400) {
            0..=3 => input.act = true,
            4 => input.eat = true,
            5 => input.patch = true,
            6 => input.goto = Some(Vec2::new(rng.random_range(0.0..t.w as f32), rng.random_range(0.0..t.h as f32))),
            // Something out of the bag, eaten or given to whoever is near.
            7 | 8 => {
                let kinds: Vec<_> = g.raccoon.bag.kinds().map(|(c, _)| c).collect();
                if !kinds.is_empty() {
                    let c = kinds[rng.random_range(0..kinds.len())];
                    if k % 2 == 0 { input.give = Some(c) } else { input.eat_this = Some(c) }
                }
            }
            _ => {}
        }
        // Now and then, somewhere else entirely (the far corners included).
        if k % 20_000 == 0 {
            g.raccoon.pos = Vec2::new(rng.random_range(1.0..t.w as f32 - 1.0), rng.random_range(1.0..t.h as f32 - 1.0));
        }
        g.step(&input);
        g.drain_events();
        max = max.max(g.stats.last_tick);
        peak_items = peak_items.max(g.items.len());
        assert!(g.raccoon.pos.is_finite() && g.hen.pos.is_finite() && g.gravy.pos.is_finite());
    }
    println!(
        "{minutes} min: night {} {} · bag {} · items now {} (peak {peak_items}) · speech {} · found {} · max tick {:.2} ms",
        g.night, g.clock_text(), g.raccoon.bag.total(), g.items.len(), g.speech.len(), g.found.count(), max.as_secs_f64() * 1000.0
    );
}
