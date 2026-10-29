//! The game application: start-up and capability discovery, the main loop
//! (fixed-step simulation, paced rendering, input), the camera, screens,
//! and cleanup.

pub mod control;
pub mod diag;
pub mod pacer;
pub mod save;
pub mod screens;
pub mod show;

use crate::gfx::scenery::{self, WorldArt};
use crate::render::layout::{self, Layout};
use crate::render::text;
use crate::render::upload::Uploader;
use crate::render::{FrameModel, MapModel, RenderConfig, Renderer, TownRef};
use crate::sim::autopilot::Autopilot;
use crate::sim::{DT, Game, SimEvent, TickInput};
use crate::term::caps::{Capabilities, Negotiator, Status};
use crate::term::input::{Decoder, InputEvent, Reply};
use crate::term::osc777::{Frame, req};
use crate::term::tty::{self, Msg, Tty};
use crate::world::plan::{self, Town};
use crate::world::town;
use control::Held;
use diag::Metrics;
use glam::Vec2;
use save::Save;
use show::Show;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::time::{Duration, Instant};

#[derive(Clone, Debug, Default)]
pub struct Options {
    pub seed: Option<u64>,
    pub diagnostics: bool,
    pub kitty_only: bool,
    pub no_vector: bool,
    pub no_latency: bool,
    pub no_frame_feedback: bool,
    pub no_haptics: bool,
    pub no_hdr: bool,
    pub no_tempfile: bool,
    pub report: bool,
    pub autoplay: bool,
    /// Another town file instead of the built-in Montpelier.
    pub town: Option<PathBuf>,
}

pub enum Screen {
    Title,
    Play,
    Paused { menu: usize },
    /// Picking something out of the bag, to eat or (`give`) to hand to
    /// whoever is in reach. The night waits.
    Pick { give: bool, sel: usize },
    Journal { back: Box<Screen>, page: usize },
    Help,
    Report,
}

/// What the game has turned on in the terminal beyond the basics.
#[derive(Clone, Debug, Default)]
pub struct Features {
    pub vector: bool,
    pub frame_feedback: bool,
    pub latency: bool,
    pub haptics: bool,
    pub tablet_mode: u32,
    pub display_events: bool,
    pub gestures: bool,
    pub granted: std::collections::BTreeMap<String, u64>,
}

pub struct App {
    pub tty: Tty,
    rx: Receiver<Msg>,
    pub decoder: Decoder,
    pub caps: Capabilities,
    pub features: Features,
    pub renderer: Renderer,
    pub show: Show,
    pub screen: Screen,
    pub game: Option<Game>,
    pub town: Arc<Town>,
    pub art: Arc<WorldArt>,
    pub opts: Options,
    pub save: Save,
    pub held: Held,
    /// One-shot actions waiting for the next tick.
    pub pending: TickInput,
    pub run_toggle: bool,
    pub map_open: bool,
    /// The town point at the field's top left, and where the camera is
    /// easing towards.
    pub cam: Vec2,
    cam_centre: Option<Vec2>,
    pub zoom: i32,
    pub metrics: Metrics,
    pub show_diag: bool,
    pub quit: bool,
    last_tick: Instant,
    last_frame: Instant,
    pub pacer: pacer::Pacer,
    pending_resize: Option<Instant>,
    last_input_at: Instant,
    key_pending: Option<Instant>,
    pen_pending: Option<u64>,
    pen_in_flight: Option<u64>,
    last_haptic: Instant,
    last_report_t: Option<f64>,
    pub latency_on: bool,
    pub cell_px: (u32, u32),
    seed: u64,
    diag_cache: Option<(Instant, crate::render::text::Overlay)>,
    autopilot: Autopilot,
    too_small: Option<(u32, u32)>,
}

const DEFAULT_PERIOD: Duration = Duration::from_micros(16_667);

/// The largest tile, in pixels, drawn as pixels (kitty): keeps the town and
/// the sprites within kitty's image storage quota on the biggest windows.
const MAX_RASTER_TILE: u32 = 96;

/// Loads the town: the file given, or Montpelier.
pub fn load_town(path: Option<&std::path::Path>) -> anyhow::Result<Town> {
    let text = match path {
        Some(p) => std::fs::read_to_string(p).map_err(|e| anyhow::anyhow!("reading {}: {e}", p.display()))?,
        None => town::MONTPELIER.to_string(),
    };
    let spec = town::parse(&text).map_err(|e| anyhow::anyhow!("{}: {e}", path.map_or("the built-in town".into(), |p| p.display().to_string())))?;
    Ok(plan::build(&spec))
}

pub fn run(opts: Options) -> anyhow::Result<()> {
    let town = Arc::new(load_town(opts.town.as_deref())?);
    let mut tty = Tty::open()?;
    tty.enter_screen()?;
    let (tx, rx) = mpsc::channel();
    tty::spawn_input_thread(tx.clone());
    tty::spawn_signal_thread(tx)?;
    let mut decoder = Decoder::new();
    let (caps, stash) = negotiate(&mut tty, &rx, &mut decoder, !opts.no_tempfile)?;
    if !caps.graphics {
        tty.restore();
        anyhow::bail!("this terminal did not answer the kitty graphics protocol query; Silkie Loophole needs kitty, Silkie, or another terminal with that protocol");
    }
    if opts.report {
        tty.restore();
        println!("{}", screens::capability_report(&caps).join("\n"));
        return Ok(());
    }
    let mut app = App::new(tty, rx, decoder, caps, opts, town);
    app.setup_terminal()?;
    for ev in stash {
        app.on_input_event(ev);
    }
    let result = app.main_loop();
    app.shutdown();
    result
}

/// Sends the capability queries and collects the answers. Input that is not
/// a reply (an impatient key press) is kept for the game.
fn negotiate(tty: &mut Tty, rx: &Receiver<Msg>, decoder: &mut Decoder, tempfile: bool) -> anyhow::Result<(Capabilities, Vec<InputEvent>)> {
    let now = Instant::now();
    let mut n = Negotiator::new(now);
    tty.write(&n.start(now, tempfile))?;
    let mut stash = Vec::new();
    loop {
        let (status, extra) = n.poll(Instant::now());
        if let Some(b) = extra {
            tty.write(&b)?;
        }
        let deadline = match status {
            Status::Done => break,
            Status::Waiting(d) => d,
        };
        let wait = deadline.saturating_duration_since(Instant::now());
        match rx.recv_timeout(wait) {
            Ok(Msg::Input(bytes)) => {
                decoder.feed(&bytes);
                for ev in decoder.drain() {
                    match ev {
                        InputEvent::Reply(r) => {
                            if let Some(b) = n.on_reply(&r, Instant::now()) {
                                tty.write(&b)?;
                            }
                        }
                        other => stash.push(other),
                    }
                }
            }
            Ok(Msg::Signal(s)) if s == libc::SIGINT || s == libc::SIGTERM || s == libc::SIGHUP => anyhow::bail!("interrupted during start-up"),
            Ok(Msg::InputClosed) => anyhow::bail!("the terminal closed its input"),
            Ok(Msg::Signal(_)) => {}
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => anyhow::bail!("input thread stopped"),
        }
    }
    Ok((n.caps, stash))
}

impl App {
    fn new(tty: Tty, rx: Receiver<Msg>, decoder: Decoder, caps: Capabilities, opts: Options, town: Arc<Town>) -> App {
        let silkie = !opts.kitty_only;
        let features = Features {
            vector: silkie && caps.vector && !opts.no_vector,
            frame_feedback: silkie && caps.frame_feedback() && !opts.no_frame_feedback,
            latency: silkie && caps.low_latency() && !opts.no_latency,
            haptics: silkie && caps.haptics() && !opts.no_haptics,
            tablet_mode: if silkie { u32::from(caps.tablet_tools()) | if caps.touchscreen() { 2 } else { 0 } } else { 0 },
            display_events: silkie && caps.display_events(),
            gestures: silkie && caps.gestures(),
            granted: Default::default(),
        };
        let p3 = silkie && caps.wide_gamut_images && caps.display.as_ref().is_some_and(|d| d.gamut == "p3" || d.gamut == "rec2020");
        let hdr = silkie && caps.hdr_images && !opts.no_hdr;
        let renderer = Renderer::new(RenderConfig {
            vector: features.vector,
            uploader: Uploader {
                temp_file: caps.temp_file && !opts.no_tempfile,
                gamut_p3: p3 && !features.vector,
            },
            sync_output: caps.sync_output,
            cell_alpha: silkie && caps.listing.is_some(),
            hdr,
        });
        let art = Arc::new(scenery::build(&town, hdr && features.vector));
        let period = caps
            .display
            .as_ref()
            .filter(|d| d.period_us > 0 && silkie)
            .map(|d| Duration::from_micros(d.period_us))
            .unwrap_or(DEFAULT_PERIOD);
        let seed = opts.seed.unwrap_or_else(|| {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos() as u64)
                .unwrap_or(1)
        });
        let mut decoder = decoder;
        decoder.sgr_pixels = caps.sgr_pixels;
        let cell = caps.cell_px.unwrap_or((10, 20));
        decoder.cell_px = cell;
        let held = Held::new(caps.kitty_keyboard.is_some());
        let mut app = App {
            tty,
            rx,
            decoder,
            pacer: pacer::Pacer::new(period, features.frame_feedback, Instant::now()),
            caps,
            features,
            renderer,
            show: Show::new(),
            screen: Screen::Title,
            game: None,
            town,
            art,
            opts: opts.clone(),
            save: Save::load(),
            held,
            pending: TickInput::default(),
            run_toggle: false,
            map_open: false,
            cam: Vec2::ZERO,
            cam_centre: None,
            zoom: 0,
            metrics: Metrics::default(),
            show_diag: opts.diagnostics,
            quit: false,
            last_tick: Instant::now(),
            last_frame: Instant::now(),
            pending_resize: None,
            last_input_at: Instant::now(),
            key_pending: None,
            pen_pending: None,
            pen_in_flight: None,
            last_haptic: Instant::now(),
            last_report_t: None,
            latency_on: false,
            cell_px: cell,
            seed,
            diag_cache: None,
            autopilot: Autopilot::new(),
            too_small: None,
        };
        app.apply_display_kind();
        app.go_title();
        app.relayout();
        if opts.autoplay {
            app.start_game();
        }
        app
    }

    fn osc(&mut self, f: Frame) {
        if let Ok(b) = f.encode() {
            let _ = self.tty.write(&b);
        }
    }

    /// Turns on input reporting and the Silkie modes the game uses, then
    /// reads each mode back (a user may refuse one).
    fn setup_terminal(&mut self) -> anyhow::Result<()> {
        self.tty.enable_input(self.caps.kitty_keyboard.is_some(), self.caps.sgr_pixels)?;
        for id in Renderer::image_ids() {
            self.tty.note_image(id);
        }
        if self.features.frame_feedback {
            self.set_mode("frame", 1);
        }
        if self.features.display_events {
            self.set_mode("display", 1);
        }
        if self.features.tablet_mode != 0 {
            self.set_mode("tablet-pointer", self.features.tablet_mode);
        }
        if self.features.gestures {
            self.set_mode("gesture", 1);
        }
        Ok(())
    }

    fn set_mode(&mut self, selector: &str, mode: u32) {
        self.osc(req::set_mode(selector, mode));
        self.osc(req::read_mode(selector));
        self.tty.note_osc_mode(selector, mode);
    }

    /// E-ink: updates are slow and leave ghosts, so draw rarely and skip
    /// the particles.
    fn apply_display_kind(&mut self) {
        let eink = self.caps.display.as_ref().is_some_and(|d| d.kind == "eink");
        self.show.effects_on = !eink;
        if eink {
            self.pacer.period = Duration::from_millis(250);
        }
    }

    fn pointer_for_play(&mut self, on: bool) {
        if !on {
            let _ = self.tty.pop_pointer();
            return;
        }
        if self.tty.modes.pointer_pushes > 0 {
            return;
        }
        if self.caps.pointer_image && !self.opts.kitty_only {
            let scale = self.caps.display.as_ref().map_or(1.0, |d| d.scale.max(1.0));
            let px = (26.0 * scale).round() as u32;
            let (bytes, (hx, hy)) = self.renderer.pointer_image(px);
            let _ = self.tty.write(&bytes);
            let _ = self.tty.push_pointer(&format!("image:{}:{hx}:{hy}", crate::render::IMG_POINTER));
        } else if self.caps.pointer_crosshair {
            let _ = self.tty.push_pointer("crosshair");
        }
    }

    pub fn set_low_latency(&mut self, on: bool) {
        self.pointer_for_play(on);
        if !self.features.latency || self.latency_on == on {
            return;
        }
        self.latency_on = on;
        self.osc(req::set_mode("latency", u32::from(on)));
        self.tty.note_osc_mode("latency", u32::from(on));
    }

    pub fn haptic(&mut self, pattern: &str) {
        if !self.features.haptics || self.last_haptic.elapsed() < Duration::from_millis(80) {
            return;
        }
        self.last_haptic = Instant::now();
        self.osc(req::haptic_play(pattern));
    }

    fn shutdown(&mut self) {
        let t = &self.renderer.totals;
        tracing::info!(
            frames = t.frames,
            bytes = self.tty.stats.bytes,
            uploads = t.uploads,
            upload_bytes = t.upload_bytes,
            chunks_sent = t.chunks_sent,
            vector_edits = t.vector_edits,
            vector_bytes = t.vector_bytes,
            graphics_errors = self.renderer.error_count,
            peak_raster_mb = self.renderer.peak_raster_bytes() / (1024 * 1024),
            last_error = ?self.renderer.last_error,
            vector = self.features.vector,
            frame_feedback = self.features.frame_feedback,
            granted = ?self.features.granted,
            mean_frame_bytes = self.metrics.frame_bytes.mean(),
            "session totals"
        );
        if let Some(g) = &self.game {
            self.save.best_haul = self.save.best_haul.max(g.raccoon.bag.total());
        }
        if let Err(e) = self.save.store() {
            tracing::warn!("saving the journal: {e}");
        }
        let bytes = self.renderer.teardown();
        let _ = self.tty.write(&bytes);
        self.tty.restore();
    }

    pub fn relayout(&mut self) {
        let (cols, rows, xpx, ypx) = tty::size();
        if xpx > 0 && ypx > 0 && cols > 0 && rows > 0 {
            self.cell_px = (xpx / cols, ypx / rows);
        }
        self.decoder.cell_px = self.cell_px;
        let mut l = Layout::new(cols, rows, self.cell_px.0, self.cell_px.1, self.zoom);
        if !self.renderer.cfg.vector {
            // Pixel scenery and sprites grow with the square of the tile size:
            // on a big high-density window they would not fit in kitty's image
            // storage quota. Vector images (Silkie) cost the terminal records,
            // not pixels.
            l = l.map(|l| l.capped(MAX_RASTER_TILE));
        }
        if l != self.renderer.layout {
            let mut clear = Vec::new();
            for r in 0..rows {
                clear.extend_from_slice(format!("\x1b[{};1H\x1b[0m\x1b[2K", r + 1).as_bytes());
            }
            let _ = self.tty.write(&clear);
            tracing::info!(cols, rows, cell_px = ?self.cell_px, layout = ?l, "layout");
            self.renderer.set_layout(l);
            self.renderer.reset_text();
        }
    }

    pub fn zoom(&mut self, by: i32) {
        self.zoom = (self.zoom + by).clamp(-6, 6);
        self.relayout();
    }

    fn new_game(&self, seed: u64) -> Game {
        Game::new(self.town.clone(), seed)
    }

    pub fn start_game(&mut self) {
        self.game = Some(self.new_game(self.seed));
        self.show.clear();
        self.held.clear();
        self.pending = TickInput::default();
        self.cam_centre = None;
        self.map_open = false;
        self.screen = Screen::Play;
        self.autopilot = Autopilot::new();
        self.set_low_latency(true);
    }

    pub fn go_title(&mut self) {
        // The title has the raccoon out on its own, behind the logo.
        self.game = Some(self.new_game(self.seed ^ 0xdead_beef));
        self.autopilot = Autopilot::new();
        self.show.clear();
        self.cam_centre = None;
        self.map_open = false;
        self.screen = Screen::Title;
        self.set_low_latency(false);
    }

    fn main_loop(&mut self) -> anyhow::Result<()> {
        self.last_tick = Instant::now();
        while !self.quit {
            let now = Instant::now();
            let next_tick = self.last_tick + Duration::from_secs_f32(DT);
            let mut deadline = next_tick.min(self.pacer.due_at(now));
            if let Some(r) = self.pending_resize {
                deadline = deadline.min(r + Duration::from_millis(60));
            }
            if self.decoder.has_pending_escape() && self.caps.kitty_keyboard.is_none() {
                deadline = deadline.min(self.last_input_at + Duration::from_millis(30));
            }
            let wait = deadline.saturating_duration_since(now);
            match self.rx.recv_timeout(wait) {
                Ok(m) => self.on_msg(m)?,
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => break,
            }
            while let Ok(m) = self.rx.try_recv() {
                self.on_msg(m)?;
            }
            let now = Instant::now();
            if self.decoder.has_pending_escape() && self.caps.kitty_keyboard.is_none() && now.duration_since(self.last_input_at) >= Duration::from_millis(30) {
                if let Some(ev) = self.decoder.flush_lone_escape() {
                    self.on_input_event(ev);
                }
            }
            if let Some(r) = self.pending_resize {
                if now.duration_since(r) >= Duration::from_millis(60) {
                    self.pending_resize = None;
                    self.relayout();
                }
            }
            self.advance_simulation(now);
            if self.pacer.probe_due(now) {
                self.osc(req::read_mode("frame"));
                self.pacer.probe_sent(now);
            }
            if now >= self.pacer.due_at(now) {
                self.render_frame();
            }
        }
        Ok(())
    }

    fn on_msg(&mut self, m: Msg) -> anyhow::Result<()> {
        match m {
            Msg::Input(bytes) => {
                self.last_input_at = Instant::now();
                self.decoder.feed(&bytes);
                for ev in self.decoder.drain() {
                    self.on_input_event(ev);
                }
            }
            Msg::InputClosed => self.quit = true,
            Msg::Signal(s) => match s {
                libc::SIGWINCH => self.pending_resize = Some(Instant::now()),
                libc::SIGINT | libc::SIGTERM | libc::SIGHUP => self.quit = true,
                libc::SIGTSTP => self.suspend()?,
                _ => {}
            },
        }
        Ok(())
    }

    pub fn suspend(&mut self) -> anyhow::Result<()> {
        if matches!(self.screen, Screen::Play) {
            self.screen = Screen::Paused { menu: 0 };
        }
        let bytes = self.renderer.teardown();
        let _ = self.tty.write(&bytes);
        self.latency_on = false;
        self.tty.suspend()?;
        self.renderer.invalidate();
        self.features.granted.clear();
        self.setup_terminal()?;
        self.relayout();
        self.pacer.reset();
        self.last_tick = Instant::now();
        Ok(())
    }

    pub fn on_input_event(&mut self, ev: InputEvent) {
        match ev {
            InputEvent::Key(k) => self.on_key(k),
            InputEvent::Mouse(m) => self.on_mouse(m),
            InputEvent::Focus(false) => {
                self.held.clear();
                if matches!(self.screen, Screen::Play) {
                    self.pause();
                }
            }
            InputEvent::Focus(true) => {}
            InputEvent::Reply(r) => self.on_reply(r),
        }
    }

    fn on_reply(&mut self, r: Reply) {
        match r {
            Reply::Graphics { id, message, .. } => {
                if message != "OK" {
                    self.renderer.on_graphics_error(id, &message);
                }
            }
            Reply::CellSize { width, height } if width > 0 && height > 0 => {
                self.cell_px = (width, height);
                self.pending_resize = Some(Instant::now());
            }
            Reply::Osc777(f) => self.on_osc(f),
            _ => {}
        }
    }

    fn on_osc(&mut self, f: Frame) {
        if f.version() != Some(1) {
            return;
        }
        match (f.selector.as_str(), f.op()) {
            ("frame", Some("event")) => self.on_frame_report(&f),
            ("display", Some("event")) => {
                let d = crate::term::caps::DisplayInfo::from_frame(&f);
                if d.period_us > 0 {
                    self.pacer.period = Duration::from_micros(d.period_us);
                }
                self.caps.display = Some(d);
                self.apply_display_kind();
            }
            (sel, Some("mode")) => {
                if sel == "frame" {
                    if let Some(rtt) = self.pacer.probe_reply(Instant::now()) {
                        self.metrics.link_rtt.push_dur(rtt);
                    }
                }
                if let Some(m) = f.get_u64("mode") {
                    self.features.granted.insert(sel.to_string(), m);
                    if sel == "frame" && m == 0 {
                        self.features.frame_feedback = false;
                        self.pacer.feedback = false;
                        self.pacer.reset();
                    }
                }
            }
            ("tablet-pointer", Some("event")) => self.on_pointer_report(&f),
            ("gesture", Some("event")) => self.on_gesture(&f),
            _ => {}
        }
    }

    fn on_frame_report(&mut self, f: &Frame) {
        let now = Instant::now();
        self.metrics.reports += 1;
        if let Some(p) = f.get_u64("period").filter(|&p| p > 0) {
            self.pacer.period = Duration::from_micros(p);
            self.metrics.period_us = p;
        }
        let t = f.get_f64("t").unwrap_or(0.0) + f.get_f64("tu").unwrap_or(0.0) / 1000.0;
        if let Some(prev) = self.last_report_t {
            if t > prev && t - prev < self.pacer.period.as_secs_f64() * 1000.0 * 4.0 {
                self.metrics.present_interval.push(t - prev);
                if self.metrics.present_interval.count >= 30 {
                    let mean = self.metrics.present_interval.mean();
                    self.pacer.observe_presentation(Duration::from_secs_f64(mean / 1000.0));
                }
            }
        }
        self.last_report_t = Some(t);
        if let Some(d) = self.pacer.report(now) {
            self.metrics.write_to_report.push_dur(d);
        }
        if let Some(t0) = self.pen_in_flight.take() {
            if t >= t0 as f64 {
                self.metrics.pen_to_present.push(t - t0 as f64);
            }
        }
    }

    fn advance_simulation(&mut self, now: Instant) {
        let step = Duration::from_secs_f32(DT);
        let mut steps = 0;
        while now.duration_since(self.last_tick) >= step {
            self.last_tick += step;
            steps += 1;
            if steps > 8 {
                self.last_tick = now;
                break;
            }
            self.tick();
        }
    }

    fn tick(&mut self) {
        self.show.update(DT);
        let playing = matches!(self.screen, Screen::Play);
        let title = matches!(self.screen, Screen::Title);
        if !(playing || title) || (playing && self.map_open) {
            return;
        }
        let Some(game) = &mut self.game else { return };
        let mut input = if playing {
            let mut i = std::mem::take(&mut self.pending);
            i.dir = self.held.dir(Instant::now());
            i.run = self.held.shift || self.run_toggle;
            i
        } else {
            TickInput::default()
        };
        if title || self.opts.autoplay {
            let t = Instant::now();
            let bot = self.autopilot.decide(game);
            self.metrics.autopilot.push_dur(t.elapsed());
            if input.dir == Vec2::ZERO && input.goto.is_none() {
                input.goto = bot.goto;
                input.act |= bot.act;
                input.eat |= bot.eat;
                input.patch |= bot.patch;
            }
        }
        game.step(&input);
        self.metrics.tick.push_dur(game.stats.last_tick);
        let events = game.drain_events();
        let mut store = false;
        for e in &events {
            self.show.on_event(e);
            if !playing {
                continue;
            }
            match e {
                SimEvent::Given { .. } | SimEvent::Loved { .. } | SimEvent::Hug { .. } => self.haptic("alignment"),
                SimEvent::Knock { .. } | SimEvent::Tear { .. } | SimEvent::Patched { .. } => self.haptic("click"),
                SimEvent::Fall { .. } | SimEvent::Slip { .. } | SimEvent::Refused { .. } | SimEvent::Scream { .. } => self.haptic("generic"),
                SimEvent::Discovery { id } => {
                    self.haptic("complete");
                    if self.save.found(*id) {
                        store = true;
                    }
                }
                _ => {}
            }
        }
        if store {
            if let Some(g) = &self.game {
                self.save.best_haul = self.save.best_haul.max(g.raccoon.bag.total());
            }
            if let Err(e) = self.save.store() {
                tracing::warn!("saving the journal: {e}");
            }
        }
    }

    fn alpha(&self) -> f32 {
        (Instant::now().duration_since(self.last_tick).as_secs_f32() / DT).clamp(0.0, 1.0)
    }

    /// Eases the camera towards the raccoon (a little ahead of it), and
    /// keeps it inside the town.
    fn update_camera(&mut self) {
        let Some(l) = self.renderer.layout else { return };
        let Some(g) = &self.game else { return };
        let dt = self.last_frame.elapsed().as_secs_f32().min(0.1);
        self.last_frame = Instant::now();
        let a = self.alpha();
        let r = &g.raccoon;
        let target = r.prev.lerp(r.pos, a) + r.vel * 0.35 - Vec2::new(0.0, 0.6);
        let c = match self.cam_centre {
            None => target,
            Some(c) => c + (target - c) * (dt * 5.0).min(1.0),
        };
        self.cam_centre = Some(c);
        let view = l.view();
        let size = self.town.size();
        let max = (size - view).max(Vec2::ZERO);
        let cam = (c - view / 2.0).clamp(Vec2::ZERO, max);
        // Centre a town smaller than the window.
        self.cam = Vec2::new(if size.x < view.x { (size.x - view.x) / 2.0 } else { cam.x }, if size.y < view.y { (size.y - view.y) / 2.0 } else { cam.y });
    }

    fn render_frame(&mut self) {
        let started = Instant::now();
        if self.renderer.layout.is_none() {
            self.relayout();
        }
        if self.renderer.layout.is_none() {
            if matches!(self.screen, Screen::Play) {
                self.pause();
            }
            let (cols, rows, _, _) = tty::size();
            if self.too_small != Some((cols, rows)) {
                let mut out = if self.too_small.is_none() { self.renderer.hide() } else { Vec::new() };
                self.too_small = Some((cols, rows));
                out.extend_from_slice(b"\x1b[0m\x1b[2J");
                let message = format!("Silkie Loophole needs a bigger window ({}x{} cells or more)", layout::MIN_COLS, layout::MIN_ROWS);
                let lines = text::wrap(&message, cols as usize);
                let top = (rows as usize).saturating_sub(lines.len()) / 2;
                for (i, line) in lines.iter().enumerate() {
                    let col = (cols as usize).saturating_sub(text::width(line)) / 2;
                    out.extend_from_slice(format!("\x1b[{};{}H{line}", top + i + 1, col + 1).as_bytes());
                }
                let _ = self.tty.write(&out);
            }
            self.pacer.idle(Instant::now());
            return;
        }
        if self.too_small.take().is_some() {
            let _ = self.tty.write(b"\x1b[0m\x1b[2J");
            self.renderer.reset_text();
        }
        self.pacer.expire(started);
        self.update_camera();
        let model = self.frame_model();
        if self.renderer.needs_heavy_upload(&model) {
            let banner = self.renderer.banner("carving the pumpkins…");
            let _ = self.tty.write(&banner);
        }
        let bytes = self.renderer.render(&model);
        let built = started.elapsed();
        if bytes.is_empty() {
            self.pacer.idle(Instant::now());
            return;
        }
        if let Err(e) = self.tty.write(&bytes) {
            tracing::error!("writing a frame: {e}");
            self.quit = true;
            return;
        }
        let written = Instant::now();
        self.metrics.build.push_dur(built);
        self.metrics.write.push_dur(self.tty.stats.last_write);
        self.metrics.frame_bytes.push(bytes.len() as f64);
        self.metrics.frames += 1;
        if let Some(k) = self.key_pending.take() {
            self.metrics.key_to_write.push_dur(written.duration_since(k));
        }
        if let Some(t) = self.pen_pending.take() {
            self.pen_in_flight = Some(t);
        }
        self.pacer.written(written);
        self.metrics.report_timeouts = self.pacer.timeouts;
    }

    pub fn town_ref(&self) -> TownRef {
        TownRef {
            key: self.town.name.clone(),
            town: self.town.clone(),
            art: self.art.clone(),
        }
    }

    fn frame_model(&mut self) -> FrameModel {
        let alpha = if matches!(self.screen, Screen::Play | Screen::Title) { self.alpha() } else { 1.0 };
        let mut m = FrameModel {
            title: matches!(self.screen, Screen::Title),
            ..Default::default()
        };
        if let Some(g) = &self.game {
            m.town = Some(self.town_ref());
            m.cam = self.cam;
            m.decals = Show::decals(g, &self.art);
            m.flicker = if self.renderer.cfg.vector { self.show.flicker() } else { 0 };
            m.look = Some(g.raccoon.costume.look(g.raccoon.belly_level()));
            m.sprites = self.show.sprites(g, alpha);
            if matches!(self.screen, Screen::Play | Screen::Title) && !self.map_open {
                m.bubbles = Show::bubbles(g, alpha);
            }
            if self.map_open {
                let mut markers = vec![(g.raccoon.pos, '@', (255, 255, 255))];
                markers.push((g.kids[0].pos, 'L', (150, 230, 110)));
                markers.push((g.kids[1].pos, 'S', (130, 190, 255)));
                markers.push((g.hen.pos, 'h', (255, 240, 200)));
                m.map = Some(MapModel { markers });
            }
        }
        screens::decorate(self, &mut m);
        if self.show_diag {
            let stale = self.diag_cache.as_ref().is_none_or(|(at, _)| at.elapsed() >= Duration::from_millis(250));
            if stale {
                self.diag_cache = Some((Instant::now(), diag_panel(self)));
            }
            if let Some((_, o)) = &self.diag_cache {
                m.overlays.push(o.clone());
            }
        }
        m
    }

    pub fn note_key(&mut self) {
        self.key_pending.get_or_insert_with(Instant::now);
    }

    pub fn note_pen(&mut self, t: u64) {
        self.pen_pending = Some(t);
    }
}

/// The diagnostics panel: renderer and terminal measurements, each labelled
/// with the clock it was measured on.
pub fn diag_panel(app: &App) -> crate::render::text::Overlay {
    use crate::render::text::{Anchor, Line, Overlay, Style};
    let m = &app.metrics;
    let r = &app.renderer;
    let f = &app.features;
    let yes = |b: bool| if b { "on" } else { "off" };
    let mut lines = vec![
        Line::new("Diagnostics  (F2 hides)", Style::Heading),
        Line::new(
            format!(
                "scenery & sprites {} · uploads {} · sync {} · HDR paints {}",
                if r.cfg.vector { "vector f=200" } else { "raster" },
                if r.cfg.uploader.temp_file { "temp file" } else { "inline" },
                yes(r.cfg.sync_output),
                yes(r.cfg.hdr && r.cfg.vector)
            ),
            Style::Normal,
        ),
        Line::new(
            format!(
                "frame reports {} · low latency {} · haptics {} · tablet {} · gestures {}",
                yes(f.frame_feedback),
                yes(app.latency_on),
                yes(f.haptics),
                f.tablet_mode,
                yes(f.gestures)
            ),
            Style::Normal,
        ),
        Line::new(format!("sim tick {:.3} ms (max {:.3}) · autopilot {:.2} ms", m.tick.mean(), m.tick.max(), m.autopilot.mean()), Style::Normal),
        Line::new(format!("frame build {:.2} ms · write {:.2} ms · {:.0} B/frame", m.build.mean(), m.write.mean(), m.frame_bytes.mean()), Style::Normal),
        Line::new(
            format!(
                "last: {} B {} placed {} deleted {} clipped · {} uploads ({} B) · {} chunks sent, {} let go",
                r.last.bytes, r.last.placements, r.last.deletions, r.last.clipped, r.last.uploads, r.last.upload_bytes, r.last.chunks_sent, r.last.chunks_dropped
            ),
            Style::Dim,
        ),
        Line::new(format!("vector: {} edits {} B (last frame) · {} chunks held", r.last.vector_edits, r.last.vector_bytes, r.chunks_loaded()), Style::Dim),
        Line::new(
            format!(
                "totals: {} frames · {:.2} MB written · uploads {:.2} MB · {} chunks sent · {} on screen",
                r.totals.frames,
                app.tty.stats.bytes as f64 / 1e6,
                r.totals.upload_bytes as f64 / 1e6,
                r.totals.chunks_sent,
                r.placement_count()
            ),
            Style::Dim,
        ),
    ];
    if f.frame_feedback {
        lines.push(Line::new(
            format!(
                "presented every {:.2} ms (terminal clock) · link round trip {:.1} ms (our clock)",
                m.present_interval.mean(),
                app.pacer.rtt().map_or(0.0, |d| d.as_secs_f64() * 1000.0),
            ),
            Style::Normal,
        ));
    } else {
        lines.push(Line::new(format!("paced by timer every {:.2} ms (no presentation reports)", app.pacer.period.as_secs_f64() * 1000.0), Style::Normal));
    }
    let mut lat = format!("key to frame written {:.2} ms (our clock)", m.key_to_write.mean());
    if !m.pen_to_present.is_empty() {
        lat.push_str(&format!(" · tap to presented {:.1} ms (terminal clock)", m.pen_to_present.mean()));
    }
    lines.push(Line::new(lat, Style::Normal));
    if let Some(e) = &r.last_error {
        lines.push(Line::new(format!("terminal said ({}x): {e}", r.error_count), Style::Bad));
    }
    let mut o = Overlay::new(Anchor::TopRight, lines);
    o.alpha = 80;
    o
}
