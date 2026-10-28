//! Keys, mouse, pen, touch and trackpad gestures.
//!
//! Walking needs to know which arrows are held. Where the terminal reports
//! key releases (the kitty keyboard protocol, which kitty and Silkie both
//! have) a key is held from its press to its release; elsewhere a press
//! (and the terminal's repeats) holds it for a moment.

use super::{App, Screen};
use crate::term::input::{KeyCode, KeyEvent, KeyKind, MouseButton, MouseEvent, MouseKind};
use crate::term::osc777::Frame;
use glam::Vec2;
use std::time::{Duration, Instant};

pub const PAUSE_MENU: [&str; 5] = ["Keep trick-or-treating", "The journal", "How to play", "Quit to title", "Quit"];

/// Without release reports, a key press holds the direction this long
/// (long enough to bridge the gap before the terminal's key repeat).
const LEGACY_HOLD: Duration = Duration::from_millis(560);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Arrow {
    Up,
    Down,
    Left,
    Right,
}

fn arrow(code: KeyCode) -> Option<Arrow> {
    match code {
        KeyCode::Up | KeyCode::Char('w') | KeyCode::Char('W') => Some(Arrow::Up),
        KeyCode::Down | KeyCode::Char('s') | KeyCode::Char('S') => Some(Arrow::Down),
        KeyCode::Left | KeyCode::Char('a') | KeyCode::Char('A') => Some(Arrow::Left),
        KeyCode::Right | KeyCode::Char('d') | KeyCode::Char('D') => Some(Arrow::Right),
        _ => None,
    }
}

/// Which arrows are held, and since when.
#[derive(Clone, Debug, Default)]
pub struct Held {
    keys: std::collections::BTreeMap<Arrow, Instant>,
    /// Shift was down with the last arrow.
    pub shift: bool,
    /// Whether the terminal reports releases (then presses never expire).
    pub releases: bool,
}

impl Held {
    pub fn new(releases: bool) -> Held {
        Held {
            releases,
            ..Default::default()
        }
    }

    pub fn press(&mut self, a: Arrow, shift: bool, now: Instant) {
        self.keys.insert(a, now);
        self.shift = shift;
    }

    pub fn release(&mut self, a: Arrow) {
        self.keys.remove(&a);
    }

    pub fn clear(&mut self) {
        self.keys.clear();
        self.shift = false;
    }

    /// The direction the held arrows point (zero for none).
    pub fn dir(&mut self, now: Instant) -> Vec2 {
        if !self.releases {
            self.keys.retain(|_, t| now.duration_since(*t) < LEGACY_HOLD);
        }
        let mut v = Vec2::ZERO;
        for a in self.keys.keys() {
            v += match a {
                Arrow::Up => Vec2::new(0.0, -1.0),
                Arrow::Down => Vec2::new(0.0, 1.0),
                Arrow::Left => Vec2::new(-1.0, 0.0),
                Arrow::Right => Vec2::new(1.0, 0.0),
            };
        }
        v.normalize_or_zero()
    }
}

impl App {
    pub fn on_key(&mut self, k: KeyEvent) {
        let now = Instant::now();
        if let Some(a) = arrow(k.code) {
            if !k.mods.ctrl() && matches!(self.screen, Screen::Play) && !self.map_open {
                if k.kind == KeyKind::Release {
                    self.held.release(a);
                } else {
                    self.held.press(a, k.mods.shift(), now);
                    self.note_key();
                }
                return;
            }
        }
        if k.kind == KeyKind::Release {
            return;
        }
        let ch = match k.code {
            KeyCode::Char(c) => Some(c.to_ascii_lowercase()),
            _ => None,
        };
        if k.mods.ctrl() && ch == Some('c') {
            self.quit = true;
            return;
        }
        if k.mods.ctrl() && ch == Some('z') {
            if let Err(e) = self.suspend() {
                tracing::error!("suspend: {e}");
            }
            return;
        }
        if k.code == KeyCode::F(2) || ch == Some('`') || ch == Some('~') {
            self.show_diag = !self.show_diag;
            return;
        }
        match &mut self.screen {
            Screen::Title => match (k.code, ch) {
                (KeyCode::Enter, _) | (_, Some(' ')) => self.start_game(),
                (KeyCode::Escape, _) | (_, Some('q')) => self.quit = true,
                (_, Some('c')) => self.screen = Screen::Report,
                (_, Some('j')) => self.screen = Screen::Journal { back: Box::new(Screen::Title), page: 0 },
                _ => {}
            },
            Screen::Report => self.go_title(),
            Screen::Journal { back, page } => match (k.code, ch) {
                (KeyCode::Right, _) | (KeyCode::Down, _) | (_, Some('n')) => *page += 1,
                (KeyCode::Left, _) | (KeyCode::Up, _) | (_, Some('b')) => *page = page.saturating_sub(1),
                _ => {
                    let back = std::mem::replace(back, Box::new(Screen::Title));
                    self.screen = *back;
                    if matches!(self.screen, Screen::Play) {
                        self.set_low_latency(true);
                    }
                }
            },
            Screen::Help => {
                self.screen = Screen::Play;
                self.set_low_latency(true);
            }
            Screen::Play => {
                if self.map_open {
                    if matches!(ch, Some('m')) || matches!(k.code, KeyCode::Escape | KeyCode::Tab | KeyCode::Enter) || ch == Some(' ') {
                        self.map_open = false;
                    }
                    return;
                }
                match (k.code, ch) {
                    (_, Some(' ')) | (KeyCode::Enter, _) => {
                        self.pending.act = true;
                        self.note_key();
                    }
                    (_, Some('e')) => {
                        self.pending.eat = true;
                        self.note_key();
                    }
                    (_, Some('f')) => {
                        self.pending.patch = true;
                        self.note_key();
                    }
                    (_, Some('r')) => {
                        self.run_toggle = !self.run_toggle;
                    }
                    (_, Some('m')) | (KeyCode::Tab, _) => self.map_open = true,
                    (_, Some('j')) => {
                        self.held.clear();
                        self.screen = Screen::Journal { back: Box::new(Screen::Play), page: 0 };
                        self.set_low_latency(false);
                    }
                    (_, Some('?')) | (_, Some('h')) | (KeyCode::F(1), _) => {
                        self.held.clear();
                        self.screen = Screen::Help;
                        self.set_low_latency(false);
                    }
                    (_, Some('+')) | (_, Some('=')) => self.zoom(1),
                    (_, Some('-')) | (_, Some('_')) => self.zoom(-1),
                    (_, Some('p')) | (KeyCode::Escape, _) => self.pause(),
                    _ => {}
                }
            }
            Screen::Paused { menu } => match (k.code, ch) {
                (KeyCode::Up, _) | (_, Some('w')) => *menu = (*menu + PAUSE_MENU.len() - 1) % PAUSE_MENU.len(),
                (KeyCode::Down, _) | (_, Some('s')) => *menu = (*menu + 1) % PAUSE_MENU.len(),
                (_, Some('p')) | (KeyCode::Escape, _) => self.resume(),
                (KeyCode::Enter, _) | (_, Some(' ')) => {
                    let m = *menu;
                    self.pause_menu(m);
                }
                (_, Some('q')) => self.quit = true,
                _ => {}
            },
        }
    }

    pub fn pause(&mut self) {
        self.screen = Screen::Paused { menu: 0 };
        self.held.clear();
        self.set_low_latency(false);
    }

    fn resume(&mut self) {
        self.screen = Screen::Play;
        self.set_low_latency(true);
    }

    fn pause_menu(&mut self, item: usize) {
        match item {
            0 => self.resume(),
            1 => self.screen = Screen::Journal { back: Box::new(Screen::Paused { menu: 1 }), page: 0 },
            2 => self.screen = Screen::Help,
            3 => self.go_title(),
            _ => self.quit = true,
        }
    }

    fn world_at(&self, px: Option<(i32, i32)>, col: u32, row: u32) -> Option<Vec2> {
        let l = self.renderer.layout?;
        let (x, y) = match px {
            Some((x, y)) => (x as f32, y as f32),
            None => ((col as f32 + 0.5) * l.cell_w as f32, (row as f32 + 0.5) * l.cell_h as f32),
        };
        Some(l.px_to_world(self.cam, x, y))
    }

    pub fn on_mouse(&mut self, m: MouseEvent) {
        let pos = self.world_at(m.px, m.col, m.row);
        let press = matches!(m.kind, MouseKind::Press(MouseButton::Left));
        match &self.screen {
            Screen::Title if press => self.start_game(),
            Screen::Paused { .. } if press => self.resume(),
            Screen::Report | Screen::Help if matches!(m.kind, MouseKind::Press(_)) => {
                if matches!(self.screen, Screen::Help) {
                    self.resume();
                } else {
                    self.go_title();
                }
            }
            Screen::Play if self.map_open && press => self.map_open = false,
            Screen::Play => match (m.kind, pos) {
                (MouseKind::Press(MouseButton::Left), Some(p)) => {
                    self.pending.goto = Some(p);
                    self.note_key();
                }
                (MouseKind::Press(MouseButton::Right), _) => self.pending.act = true,
                _ => {}
            },
            _ => {}
        }
    }

    /// A report of the tablet pointer protocol: a pen or a finger on a
    /// touchscreen walks the raccoon there. Positions are 1/256 pixel.
    pub fn on_pointer_report(&mut self, f: &Frame) {
        let kind = f.get_u64("kind").unwrap_or(0);
        if kind != 1 && kind != 2 {
            return;
        }
        if kind == 2 && f.get_u64("src") == Some(1) {
            // Fingers on a touchpad are positions on the pad.
            return;
        }
        if f.get_u64("phase") != Some(1) {
            return;
        }
        let x = f.get_i64("x").unwrap_or(0) as f32 / 256.0;
        let y = f.get_i64("y").unwrap_or(0) as f32 / 256.0;
        let pos = self.renderer.layout.map(|l| l.px_to_world(self.cam, x, y));
        if let Some(t) = f.get_u64("t") {
            self.note_pen(t);
        }
        match (&self.screen, pos) {
            (Screen::Play, Some(p)) => self.pending.goto = Some(p),
            (Screen::Title, _) => self.start_game(),
            (Screen::Paused { .. }, _) => self.resume(),
            _ => {}
        }
    }

    /// A trackpad gesture (Silkie): pinch in for the town map, out to come
    /// back; a smart zoom toggles it.
    pub fn on_gesture(&mut self, f: &Frame) {
        if !matches!(self.screen, Screen::Play) {
            return;
        }
        let kind = f.get_u64("kind").unwrap_or(0);
        let phase = f.get_u64("phase").unwrap_or(0);
        match (kind, phase) {
            (1, 2) | (1, 3) => {
                let total = f.get_i64("total").unwrap_or(1000);
                if total < 820 {
                    self.map_open = true;
                } else if total > 1200 {
                    self.map_open = false;
                }
            }
            (4, 1) => self.map_open = !self.map_open,
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn held_arrows_combine_and_expire_without_releases() {
        let t0 = Instant::now();
        let mut h = Held::default();
        h.press(Arrow::Up, false, t0);
        h.press(Arrow::Right, false, t0);
        let d = h.dir(t0);
        assert!((d - Vec2::new(1.0, -1.0).normalize()).length() < 1e-5);
        assert_eq!(h.dir(t0 + Duration::from_secs(1)), Vec2::ZERO);
        let mut h = Held {
            releases: true,
            ..Default::default()
        };
        h.press(Arrow::Left, true, t0);
        assert_eq!(h.dir(t0 + Duration::from_secs(10)), Vec2::new(-1.0, 0.0));
        h.release(Arrow::Left);
        assert_eq!(h.dir(t0), Vec2::ZERO);
    }
}

#[cfg(test)]
mod keys {
    use super::*;

    #[test]
    fn the_letter_keys_for_screens_are_not_movement() {
        for c in ['j', 'J', 'm', 'e', 'f', 'p', 'r', 'h', '?'] {
            assert!(arrow(KeyCode::Char(c)).is_none(), "{c} walks");
        }
    }
}
