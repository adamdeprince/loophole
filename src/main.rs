//! Silkie Loophole: free candy, if you're in costume.

use clap::{Parser, Subcommand};
use silkie_loophole::app::{self, Options, show::Show};
use silkie_loophole::gfx::scenery;
use silkie_loophole::render::snapshot::snapshot;
use silkie_loophole::render::{FrameModel, TownRef};
use silkie_loophole::sim::autopilot::Autopilot;
use silkie_loophole::sim::{DT, Game, SimEvent};
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Parser, Debug)]
#[command(version, about = "A raccoon in a bedsheet trick-or-treats its way round Montpelier, Vermont.")]
struct Cli {
    /// Random seed (the same seed and inputs make the same night).
    #[arg(long)]
    seed: Option<u64>,
    /// Show the diagnostics panel from the start (toggle with F2 or `).
    #[arg(long)]
    diagnostics: bool,
    /// Use only what plain kitty offers, even when the terminal has more.
    #[arg(long)]
    kitty_only: bool,
    /// Draw scenery and sprites as bitmaps even where vector images exist.
    #[arg(long)]
    no_vector: bool,
    /// Do not ask for the terminal's low-latency mode.
    #[arg(long)]
    no_latency: bool,
    /// Pace frames by timer even where presentation reports exist.
    #[arg(long)]
    no_frame_feedback: bool,
    /// No haptic feedback.
    #[arg(long)]
    no_haptics: bool,
    /// No HDR glow on porch lights, pumpkins and the gold dome.
    #[arg(long)]
    no_hdr: bool,
    /// Never send images through temporary files (as over SSH).
    #[arg(long)]
    no_tempfile: bool,
    /// Print what the terminal supports and exit.
    #[arg(long)]
    report: bool,
    /// Let the raccoon trick-or-treat by itself.
    #[arg(long)]
    autoplay: bool,
    /// Play in another town (a town file; see assets/town/README.md).
    #[arg(long)]
    town: Option<PathBuf>,
    /// Write a log here (the terminal itself is busy with the game).
    #[arg(long)]
    log: Option<PathBuf>,
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Lets the raccoon trick-or-treat by itself, without a terminal, and
    /// prints how the night went.
    Headless {
        #[arg(long, default_value_t = 1)]
        seed: u64,
        #[arg(long, default_value_t = 600)]
        seconds: u32,
    },
    /// Lets the raccoon play for a while and writes a picture of the moment,
    /// as the terminal would show it.
    Snapshot {
        #[arg(long, default_value_t = 20.0)]
        at: f32,
        #[arg(long, default_value_t = 48)]
        tile_px: u32,
        #[arg(long, default_value_t = 1600)]
        width: u32,
        #[arg(long, default_value_t = 900)]
        height: u32,
        #[arg(long, default_value_t = 1)]
        seed: u64,
        /// Centre the picture here (tiles) instead of on the raccoon.
        #[arg(long, num_args = 2)]
        centre: Option<Vec<f32>>,
        out: PathBuf,
    },
    /// Checks a town file and says what it makes.
    Town { file: Option<PathBuf> },
}

fn main() {
    let cli = Cli::parse();
    if let Some(path) = &cli.log {
        if let Ok(file) = std::fs::File::create(path) {
            tracing_subscriber::fmt()
                .with_writer(std::sync::Mutex::new(file))
                .with_ansi(false)
                .with_env_filter(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
                .init();
        }
    }
    let result = match cli.command {
        Some(Command::Headless { seed, seconds }) => headless(cli.town.as_deref(), seed, seconds),
        Some(Command::Snapshot { at, tile_px, width, height, seed, centre, out }) => picture(cli.town.as_deref(), at, tile_px, width, height, seed, centre, &out),
        Some(Command::Town { file }) => town_report(file.as_deref().or(cli.town.as_deref())),
        None => app::run(Options {
            seed: cli.seed,
            diagnostics: cli.diagnostics,
            kitty_only: cli.kitty_only,
            no_vector: cli.no_vector,
            no_latency: cli.no_latency,
            no_frame_feedback: cli.no_frame_feedback,
            no_haptics: cli.no_haptics,
            no_hdr: cli.no_hdr,
            no_tempfile: cli.no_tempfile,
            report: cli.report,
            autoplay: cli.autoplay,
            town: cli.town,
        }),
    };
    if let Err(e) = result {
        eprintln!("silkie-loophole: {e:#}");
        std::process::exit(1);
    }
}

fn headless(town: Option<&std::path::Path>, seed: u64, seconds: u32) -> anyhow::Result<()> {
    let t = Arc::new(app::load_town(town)?);
    let mut g = Game::new(t, seed);
    let mut bot = Autopilot::new();
    let mut total = std::time::Duration::ZERO;
    let mut max = std::time::Duration::ZERO;
    let mut ticks = 0u64;
    let mut given = 0;
    let mut refused = 0;
    let mut falls = 0;
    while g.time < seconds as f32 {
        let input = bot.decide(&g);
        g.step(&input);
        for e in g.drain_events() {
            match e {
                SimEvent::Given { .. } => given += 1,
                SimEvent::Refused { .. } => refused += 1,
                SimEvent::Fall { who: silkie_loophole::sim::Who::Raccoon, .. } | SimEvent::Slip { .. } => falls += 1,
                _ => {}
            }
        }
        total += g.stats.last_tick;
        max = max.max(g.stats.last_tick);
        ticks += 1;
    }
    let r = &g.raccoon;
    println!("{} seconds of Halloween in {} (night {}, {})", seconds, g.town.name, g.night, g.clock_text());
    println!("doors knocked {} · given candy {given} · refused {refused} · falls {falls}", g.stats.doors_knocked);
    println!("bag {} pieces ({} points) · eaten {} · happiness {:.0}% · belly {:.1}", r.bag.total(), r.bag.value(), g.stats.eaten, r.happy * 100.0, r.belly);
    println!("sheet passes {:.0}% · {} holes, {} snags, {} patches · walked {:.0} tiles", r.passing() * 100.0, r.costume.holes(), r.costume.snags(), r.costume.patches(), g.stats.distance);
    println!("Loki's pail {} · Skaði's pillowcase {} · eggs home {}", g.kids[0].pail, g.kids[1].pail, g.feats.iter().zip(g.town.features.iter()).find(|(_, f)| f.kind == silkie_loophole::world::town::FeatureKind::EggBasket).map_or(0, |(s, _)| s.n));
    println!("discoveries: {}", g.found.found.iter().map(|d| d.title()).collect::<Vec<_>>().join("; "));
    println!("{} items lying about · tick mean {:.1} µs, max {:.1} µs", g.items.len(), total.as_secs_f64() * 1e6 / ticks.max(1) as f64, max.as_secs_f64() * 1e6);
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn picture(town: Option<&std::path::Path>, at: f32, tile_px: u32, width: u32, height: u32, seed: u64, centre: Option<Vec<f32>>, out: &std::path::Path) -> anyhow::Result<()> {
    let t = Arc::new(app::load_town(town)?);
    let art = Arc::new(scenery::build(&t, false));
    let mut g = Game::new(t.clone(), seed);
    let mut bot = Autopilot::new();
    let mut show = Show::new();
    while g.time < at {
        let input = bot.decide(&g);
        g.step(&input);
        show.update(DT);
        for e in g.drain_events() {
            show.on_event(&e);
        }
    }
    let view = glam::Vec2::new(width as f32, height as f32) / tile_px as f32;
    let c = match &centre {
        Some(v) if v.len() == 2 => glam::Vec2::new(v[0], v[1]),
        _ => g.raccoon.pos,
    };
    let cam = (c - view / 2.0).clamp(glam::Vec2::ZERO, (t.size() - view).max(glam::Vec2::ZERO));
    let model = FrameModel {
        town: Some(TownRef {
            key: t.name.clone(),
            town: t.clone(),
            art: art.clone(),
        }),
        cam,
        decals: Show::decals(&g, &art),
        look: Some(g.raccoon.costume.look(g.raccoon.belly_level())),
        sprites: show.sprites(&g, 1.0),
        ..Default::default()
    };
    let pm = snapshot(&model, width, height, tile_px);
    pm.save_png(out)?;
    println!("{} at {:.1} s ({}): {} sprites, {}", t.name, g.time, g.clock_text(), model.sprites.len(), t.describe(g.raccoon.pos));
    Ok(())
}

fn town_report(file: Option<&std::path::Path>) -> anyhow::Result<()> {
    let t = app::load_town(file)?;
    let houses = t.buildings.iter().filter(|b| b.kind == silkie_loophole::world::town::BuildingKind::House).count();
    println!("{}: {} by {} tiles, {} streets, {} buildings ({houses} houses), {} features", t.name, t.w, t.h, t.streets.len(), t.buildings.len(), t.features.len());
    for b in t.buildings.iter().filter(|b| b.kind != silkie_loophole::world::town::BuildingKind::House) {
        println!("  {:?} {} (door {:?} at {:.1},{:.1})", b.kind, b.name, b.door.facing, b.door.at.x, b.door.at.y);
    }
    Ok(())
}
