//! The status lines, the menus, the journal, the help page and the
//! capability report.

use super::control::PAUSE_MENU;
use super::{App, Screen};
use crate::render::FrameModel;
use crate::render::text::{Anchor, Line, Overlay, Style, width};
use crate::sim::Game;
use crate::sim::candy::Candy;
use crate::sim::discover::Id;
use crate::sim::raccoon::BELLY_FULL;
use crate::term::caps::Capabilities;

const GOLD: &str = "\x1b[1;38;2;255;190;90m";
const CREAM: &str = "\x1b[38;2;244;236;214m";
const DIM: &str = "\x1b[38;2;150;140;170m";
const RED: &str = "\x1b[38;2;255;105;120m";
const GREEN: &str = "\x1b[38;2;156;222;120m";
const AMBER: &str = "\x1b[38;2;255;179;71m";
const PURPLE: &str = "\x1b[38;2;200;150;255m";
const RESET: &str = "\x1b[0m";

fn spread(left: &str, right: &str, cols: usize) -> String {
    let gap = cols.saturating_sub(width(left) + width(right)).max(1);
    format!("{left}{}{right}", " ".repeat(gap))
}

/// A bar of `n` cells, `frac` full.
fn bar(frac: f32, n: usize, on: &str) -> String {
    let full = ((frac.clamp(0.0, 1.0) * n as f32).round() as usize).min(n);
    format!("{on}{}{DIM}{}{RESET}", "▮".repeat(full), "▯".repeat(n - full))
}

fn colour_for(frac: f32) -> &'static str {
    if frac > 0.6 {
        GREEN
    } else if frac > 0.3 {
        AMBER
    } else {
        RED
    }
}

fn status_top(g: &Game, found: usize, cols: usize) -> String {
    let r = &g.raccoon;
    let loki = &g.kids[crate::sim::kids::LOKI];
    let pass = r.passing();
    let left = format!(
        " {GOLD}LOOPHOLE{RESET}  {DIM}bag{RESET} {CREAM}{}{RESET} {DIM}({} pts){RESET}  {DIM}happy{RESET} {}  {DIM}breath{RESET} {}  {DIM}belly{RESET} {}  {DIM}sheet{RESET} {}{:>3}%{RESET}  {DIM}mend{RESET} {CREAM}{}{RESET}  {DIM}loki{RESET} {CREAM}{}{RESET} {DIM}sugar{RESET} {}",
        r.bag.total(),
        r.bag.value(),
        bar(r.happy, 8, colour_for(r.happy)),
        bar(r.stamina, 4, if r.winded { RED } else { GREEN }),
        bar(r.belly / BELLY_FULL, 4, AMBER),
        colour_for(pass),
        (pass * 100.0).round() as u32,
        r.mats.total(),
        loki.pail,
        bar(loki.sugar, 4, if loki.sugar_rush() { RED } else { PURPLE }),
    );
    let right = format!(
        "{PURPLE}{}{RESET}  {DIM}night {}{RESET}  {DIM}found{RESET} {GOLD}{found}{RESET}{DIM}/{}{RESET} ",
        g.clock_text(),
        g.night,
        Id::ALL.len()
    );
    spread(&left, &right, cols)
}

/// What the action key would do here.
fn hint(g: &Game) -> String {
    let r = &g.raccoon;
    if g.visit.is_some() {
        let give = crate::sim::give::taker(g).map(|t| format!("{AMBER}G{RESET} {CREAM}give to {}{RESET}  ", crate::sim::give::name(g, t)));
        return format!("{}{DIM}walk away to leave{RESET}", give.unwrap_or_default());
    }
    if g.gravy.lying() && g.gravy.pos.distance(r.pos) < 1.6 {
        return format!("{AMBER}Space{RESET} {CREAM}rub Gravy's belly{RESET}");
    }
    if let Some(b) = g.town.door_near(r.pos, 1.0) {
        let lit = g.houses[b.id].light;
        return format!("{AMBER}Space{RESET} {CREAM}knock{RESET} {DIM}({}){RESET}", if lit { "porch light on" } else { "porch light off" });
    }
    let parts = [
        format!("{AMBER}E{RESET} {DIM}eat{RESET}"),
        match crate::sim::give::taker(g) {
            Some(t) => format!("{AMBER}G{RESET} {CREAM}give to {}{RESET}", crate::sim::give::name(g, t)),
            None => format!("{AMBER}G{RESET} {DIM}give{RESET}"),
        },
        format!("{AMBER}F{RESET} {DIM}patch{RESET}"),
        format!("{AMBER}Shift{RESET} {DIM}run{RESET}"),
        format!("{AMBER}M{RESET} {DIM}map{RESET}"),
        format!("{AMBER}J{RESET} {DIM}journal{RESET}"),
        format!("{AMBER}?{RESET} {DIM}help{RESET}"),
    ];
    parts.join("  ")
}

fn status_bottom(g: &Game, cols: usize) -> String {
    let left = match &g.message {
        Some((m, _)) => format!(" {CREAM}{m}{RESET}"),
        None => format!(" {DIM}{}{RESET}", g.town.describe(g.raccoon.pos)),
    };
    spread(&left, &format!("{} ", hint(g)), cols)
}

pub fn decorate(app: &App, m: &mut FrameModel) {
    let cols = app.renderer.layout.map_or(80, |l| l.cols as usize);
    let found = app.save.found.len();
    match &app.screen {
        Screen::Title => {
            m.top = spread(&format!(" {GOLD}SILKIE LOOPHOLE{RESET}  {DIM}free candy, if you're in costume{RESET}"), &format!("{DIM}journal{RESET} {GOLD}{found}{RESET}{DIM}/{}{RESET} ", Id::ALL.len()), cols);
            m.bottom = format!(" {AMBER}Space{RESET} {CREAM}start{RESET}   {AMBER}J{RESET} {DIM}journal{RESET}   {AMBER}C{RESET} {DIM}what this terminal can do{RESET}   {AMBER}Q{RESET} {DIM}quit{RESET}");
            let mut lines = vec![
                Line::new("Montpelier, Vermont. Halloween night.", Style::Heading),
                Line::new("", Style::Normal),
                Line::new("A raccoon has worked out that people give free candy", Style::Normal),
                Line::new("to anybody in a costume. It has found a bedsheet.", Style::Normal),
                Line::new("", Style::Normal),
                Line::new("A work of fiction and gentle parody. The shops are made up;", Style::Dim),
                Line::new("any likeness to a real business or person is coincidence.", Style::Dim),
                Line::new("Not affiliated with Montpelier or the State of Vermont.", Style::Dim),
                Line::new("", Style::Normal),
            ];
            for (name, on) in app.caps.summary() {
                if on {
                    lines.push(Line::new(format!("✓ {name}"), Style::Good));
                }
            }
            let mut o = Overlay::new(Anchor::Low, lines);
            o.min_width = 54;
            m.overlays.push(o);
        }
        Screen::Report => {
            let lines: Vec<Line> = capability_report(&app.caps).into_iter().map(|s| Line::new(s, Style::Normal)).collect();
            m.overlays.push(Overlay::new(Anchor::Center, lines));
            m.bottom = format!(" {DIM}any key: back{RESET}");
        }
        Screen::Journal { page, .. } => {
            m.overlays.push(journal(app, *page));
            m.bottom = format!(" {AMBER}←/→{RESET} {DIM}pages{RESET}   {DIM}any other key: back{RESET}");
        }
        Screen::Help => {
            m.overlays.push(help());
            m.bottom = format!(" {DIM}any key: back to the night{RESET}");
        }
        _ => {}
    }
    if let Some(g) = &app.game {
        if !matches!(app.screen, Screen::Title | Screen::Report) {
            m.top = status_top(g, found, cols);
            if !matches!(app.screen, Screen::Journal { .. } | Screen::Help) {
                m.bottom = status_bottom(g, cols);
            }
        }
        if app.map_open {
            m.bottom = format!(" {CREAM}{}{RESET}   {DIM}M or Esc: close the map{RESET}", g.town.describe(g.raccoon.pos));
        }
    }
    if let (Screen::Pick { give, sel }, Some(g)) = (&app.screen, &app.game) {
        m.overlays.push(pick_menu(g, *give, *sel));
        m.bottom = format!(
            " {AMBER}↑/↓{RESET} {DIM}choose{RESET}   {AMBER}Enter{RESET} {DIM}{}{RESET}   {AMBER}1-9{RESET} {DIM}pick{RESET}   {DIM}any other key: never mind{RESET}",
            if *give { "give" } else { "eat" }
        );
    }
    if let Screen::Paused { menu } = &app.screen {
        let mut lines = vec![Line::new("Paused", Style::Heading), Line::new("", Style::Normal)];
        for (i, item) in PAUSE_MENU.iter().enumerate() {
            lines.push(Line::new(*item, if i == *menu { Style::Selected } else { Style::Normal }));
        }
        let mut o = Overlay::new(Anchor::Center, lines);
        o.min_width = 30;
        m.overlays.push(o);
    }
}

/// What the pick menu offers, best first: everything edible to eat, and
/// anything at all to give.
pub fn pick_list(g: &Game, give: bool) -> Vec<(Candy, u64)> {
    let mut v: Vec<(Candy, u64)> = g.raccoon.bag.kinds().filter(|(c, _)| give || c.edible()).collect();
    v.sort_by_key(|(c, _)| std::cmp::Reverse(c.value()));
    v
}

fn pick_menu(g: &Game, give: bool, sel: usize) -> Overlay {
    let title = match crate::sim::give::taker(g).filter(|_| give) {
        Some(t) => format!("Give what to {}?", crate::sim::give::name(g, t)),
        None if give => "Give what?".to_string(),
        None => "Eat what?".to_string(),
    };
    let mut lines = vec![Line::new(title, Style::Heading), Line::new("", Style::Normal)];
    for (i, (c, n)) in pick_list(g, give).into_iter().enumerate() {
        let key = if i < 9 { format!("{}", i + 1) } else { " ".to_string() };
        let love = if !give && c.loved() { "  ♥" } else { "" };
        let style = if i == sel { Style::Selected } else { Style::Normal };
        lines.push(Line::new(format!("{key}  {} ×{n}{love}", c.name()), style));
    }
    let mut o = Overlay::new(Anchor::Center, lines);
    o.min_width = 40;
    o
}

fn journal(app: &App, page: usize) -> Overlay {
    let per = 14;
    let pages = Id::ALL.len().div_ceil(per);
    let page = page.min(pages - 1);
    let mut lines = vec![
        Line::new(format!("The journal: {} of {} found", app.save.found.len(), Id::ALL.len()), Style::Heading),
        Line::new(format!("page {} of {pages}", page + 1), Style::Dim),
        Line::new("", Style::Normal),
    ];
    for id in Id::ALL.iter().skip(page * per).take(per) {
        if app.save.found.contains(id) {
            lines.push(Line::new(format!("✓ {}", id.title()), Style::Good));
        } else {
            lines.push(Line::new(format!("? {}", id.hint()), Style::Dim));
        }
    }
    let mut o = Overlay::new(Anchor::Center, lines);
    o.min_width = 56;
    o
}

fn help() -> Overlay {
    let lines = [
        ("How to play", Style::Heading),
        ("", Style::Normal),
        ("Arrows or WASD walk; hold Shift (or press R) to run.", Style::Normal),
        ("Space at a door knocks. Space by a bowl, a pole or a", Style::Normal),
        ("decorated tree takes something. Space by Gravy: belly rub.", Style::Normal),
        ("E picks something to eat; G something to give whoever is near.", Style::Normal),
        ("F patches the sheet.", Style::Normal),
        ("Click, tap or touch somewhere to walk there.", Style::Normal),
        ("M (or pinch on a Silkie trackpad) shows the town map.", Style::Normal),
        ("+ and - zoom. J is the journal. P pauses.", Style::Normal),
        ("", Style::Normal),
        ("Walking makes a raccoon sad; candy makes it happy; a sad", Style::Dim),
        ("raccoon is a slow raccoon. Bushes, thorns, falls and dogs tear", Style::Dim),
        ("the sheet; a torn, muddy, wet or smelly sheet fools nobody.", Style::Dim),
        ("Patch it with whatever you find. Watch where you step.", Style::Dim),
        ("Loki and Skaði think you are a kid. Let them keep thinking so.", Style::Dim),
        ("Loki snacks as he goes; the more sugar, the more he crashes.", Style::Dim),
        ("Everybody likes something different. Try hákarl on a stranger.", Style::Dim),
    ];
    let mut o = Overlay::new(Anchor::Center, lines.iter().map(|(t, s)| Line::new(*t, *s)).collect());
    o.min_width = 64;
    o
}

fn mark(on: bool) -> &'static str {
    if on { "yes" } else { "no" }
}

/// What the terminal answered, for `--report`, the title screen's C key
/// and the diagnostics panel.
pub fn capability_report(c: &Capabilities) -> Vec<String> {
    let mut v = vec!["Terminal capabilities (as answered by the terminal)".to_string()];
    v.push(format!("graphics protocol: {}   temp-file medium: {}", mark(c.graphics), mark(c.temp_file)));
    v.push(format!(
        "kitty keyboard: {}   synchronized output: {}   SGR pixel mouse: {}",
        c.kitty_keyboard.map_or("no".to_string(), |f| format!("yes (flags {f})")),
        mark(c.sync_output),
        mark(c.sgr_pixels)
    ));
    v.push(format!(
        "cell: {}   window: {}",
        c.cell_px.map_or("?".into(), |(w, h)| format!("{w}x{h} px")),
        c.window_px.map_or("?".into(), |(w, h)| format!("{w}x{h} px"))
    ));
    match &c.listing {
        Some(l) => v.push(format!("OSC 777 extensions: {}", l.iter().map(|(k, ver)| format!("{k}.{ver}")).collect::<Vec<_>>().join(" "))),
        None => v.push("OSC 777 extensions: none listed".into()),
    }
    v.push(format!(
        "vector images: {}   HDR images: {}   wide-gamut images: {}",
        mark(c.vector),
        mark(c.hdr_images),
        mark(c.wide_gamut_images)
    ));
    v.push(format!("pointer shapes: crosshair {}   image pointers {}", mark(c.pointer_crosshair), mark(c.pointer_image)));
    for (name, s) in &c.selectors {
        let fields: Vec<String> = s
            .fields
            .iter()
            .filter(|(k, _)| !matches!(k.as_str(), "v" | "op" | "id"))
            .map(|(k, val)| format!("{k}={val}"))
            .collect();
        v.push(format!("  {name}: {}", fields.join(" ")));
    }
    if let Some(d) = &c.display {
        v.push(format!(
            "display: {} {} {}-bit gamut={} headroom={} scale={} refresh={} ppi={}",
            d.kind,
            d.color,
            d.bits,
            d.gamut,
            d.headroom,
            d.scale,
            if d.period_us > 0 { format!("{:.1} Hz", 1e6 / d.period_us as f64) } else { "?".into() },
            d.ppi
        ));
    }
    for (what, d) in &c.timings {
        v.push(format!("discovery {what}: {:.1} ms", d.as_secs_f64() * 1000.0));
    }
    if c.timed_out {
        v.push("(some queries timed out)".into());
    }
    v
}
