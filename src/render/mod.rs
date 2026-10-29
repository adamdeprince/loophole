//! The renderer: turns a frame model (the camera, the sprites, the decal
//! states, the costume, the text) into terminal output.
//!
//! It keeps a model of what the terminal holds (images, every placement
//! and where it is, the text of each line and overlay) and writes only the
//! difference each frame.
//!
//! **The town** is cut into square chunks of tiles, two layers each (the
//! ground under everybody, the canopy over them). Only the chunks around the
//! camera are in the terminal at once; chunks well behind are deleted, and
//! sent again if the raccoon comes back. On Silkie a chunk is a vector image
//! (`f=200`) of a few kilobytes of records that the terminal draws; a
//! porch light going out or a pumpkin flickering is a patch of a few
//! records. Elsewhere chunks are rasterized here and sent as pixels, and a
//! decal change sends the chunk again.
//!
//! **Sprites** are placements of sprite sheets, chosen by source rectangle
//! and clipped to the field. On Silkie the sheets themselves are vector
//! images too, drawn by the terminal; elsewhere they are pixels. The
//! raccoon's sheet is repainted (into a second image, then swapped) when its
//! costume changes.
//!
//! Speech bubbles, the status lines and menus are terminal text.

pub mod layout;
pub mod snapshot;
pub mod text;
pub mod upload;

use crate::art::Painted;
use crate::gfx::scenery::{self, DecalKind, Layer as ArtLayer, WorldArt};
use crate::gfx::sprites::SheetSpec;
use crate::gfx::vector::{Edit, Scene};
use crate::sim::costume::Look;
use crate::term::kitty_gfx::{self, EncodeStats, Place, Quiet};
use crate::world::plan::Town;
use glam::Vec2;
use layout::Layout;
use std::collections::{BTreeMap, BTreeSet};
use std::io::Write as _;
use std::sync::Arc;
use text::{Anchor, Line, Overlay, Style};
use tiny_skia::{Pixmap, Transform};
use upload::Uploader;

/// Our image ids start here, well clear of small ids other programs use.
pub const IMG_BASE: u32 = 0x4c00_0000;
pub const IMG_RACCOON: [u32; 2] = [IMG_BASE + 40, IMG_BASE + 41];
pub const IMG_POINTER: u32 = IMG_BASE + 50;
pub const IMG_TITLE: u32 = IMG_BASE + 51;
pub const IMG_MAP: u32 = IMG_BASE + 52;
/// Chunk images: `IMG_CHUNKS + 2 * chunk + layer`.
pub const IMG_CHUNKS: u32 = IMG_BASE + 0x10_0000;

// Stacking. Everything sits below INT32_MIN/2 so that cells with a
// background colour (menus, bubbles) are drawn over the town.
pub const Z_GROUND: i32 = -2_000_000_000;
pub const Z_DECAL: i32 = -1_990_000_000;
pub const Z_SPRITE: i32 = -1_900_000_000;
pub const Z_CANOPY: i32 = -1_800_000_000;
pub const Z_CAST: i32 = -1_760_000_000;
pub const Z_OVER: i32 = -1_700_000_000;
pub const Z_MAP: i32 = -1_650_000_000;
pub const Z_TITLE: i32 = -1_640_000_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SheetId {
    Raccoon,
    Bag,
    Loki,
    Skadi,
    Mom,
    Ethan,
    Frost,
    Coolidge,
    Hen,
    Gravy,
    Pets,
    /// One townsperson's look.
    Folk(u8),
    Stuff,
    Fx,
}

impl SheetId {
    /// Every sheet but the townsfolk's (see [`SheetId::all`]).
    pub const ALL: [SheetId; 13] = [
        SheetId::Raccoon,
        SheetId::Bag,
        SheetId::Loki,
        SheetId::Skadi,
        SheetId::Mom,
        SheetId::Ethan,
        SheetId::Frost,
        SheetId::Coolidge,
        SheetId::Hen,
        SheetId::Gravy,
        SheetId::Pets,
        SheetId::Stuff,
        SheetId::Fx,
    ];

    /// Every sheet there is, the townsfolk's looks included.
    pub fn all() -> Vec<SheetId> {
        let mut v = SheetId::ALL.to_vec();
        v.extend((0..crate::art::people::look_count()).map(|i| SheetId::Folk(i as u8)));
        v
    }

    pub fn image_id(self) -> u32 {
        let n = match self {
            SheetId::Raccoon => 0,
            SheetId::Bag => 1,
            SheetId::Loki => 2,
            SheetId::Skadi => 3,
            SheetId::Mom => 4,
            SheetId::Ethan => 5,
            SheetId::Hen => 6,
            SheetId::Gravy => 7,
            SheetId::Pets => 8,
            SheetId::Stuff => 9,
            SheetId::Fx => 10,
            SheetId::Frost => 11,
            SheetId::Coolidge => 12,
            SheetId::Folk(i) => 100 + i as u32,
        };
        IMG_BASE + 1 + n
    }

    /// The pixels per tile a sheet is painted at. People and animals are
    /// drawn larger than the town's scale (a raccoon is not a fifth of a
    /// house), so that they read at a glance.
    fn res(self, tile_px: u32) -> u32 {
        let k = match self {
            SheetId::Hen => 0.85,
            SheetId::Stuff => 1.15,
            SheetId::Fx => 1.2,
            SheetId::Folk(_) => 1.2,
            _ => 1.35,
        };
        ((tile_px as f32 * k).round() as u32).max(8)
    }
}

/// Paints a sheet at `tile_px` pixels per tile (the raccoon in `look`).
pub fn paint_sheet(id: SheetId, tile_px: u32, look: &Look, vector: bool) -> Painted {
    let res = id.res(tile_px);
    match id {
        SheetId::Raccoon => crate::art::raccoon::sheet(res, look, vector),
        SheetId::Bag => crate::art::raccoon::bag_sheet(res, vector),
        SheetId::Loki => crate::art::family::loki_sheet(res, vector),
        SheetId::Skadi => crate::art::family::skadi_sheet(res, vector),
        SheetId::Mom => crate::art::family::mom_sheet(res, vector),
        SheetId::Ethan => crate::art::family::ethan_sheet(res, vector),
        SheetId::Frost => crate::art::family::frost_sheet(res, vector),
        SheetId::Coolidge => crate::art::family::coolidge_sheet(res, vector),
        SheetId::Hen => crate::art::hen::sheet(res, vector),
        SheetId::Gravy => crate::art::animals::gravy_sheet(res, vector),
        SheetId::Pets => crate::art::animals::pets_sheet(res, vector),
        SheetId::Folk(i) => crate::art::people::look_sheet(i as usize, res, vector),
        SheetId::Stuff => crate::art::stuff::stuff_sheet(res, vector),
        SheetId::Fx => crate::art::fx::fx_sheet(res, vector),
    }
}

/// Paints a sheet by name, as pixels, for previews.
pub fn paint_named(name: &str, px: u32, look: &Look) -> Option<(SheetSpec, Pixmap)> {
    let id = match name.strip_prefix("folk") {
        Some(n) if !n.is_empty() => SheetId::Folk(n.parse().ok()?),
        _ => SheetId::ALL.iter().copied().find(|s| format!("{s:?}").to_lowercase() == name)?,
    };
    let p = paint_sheet(id, px, look, false);
    Some((p.spec, p.pixels?))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Layer {
    /// On the ground, under everybody (items, shadows).
    Decal,
    /// Sorted by how far down the screen their feet are.
    Body,
    /// The player's company (the raccoon, its bag, the children, the hen):
    /// sorted the same way, but never hidden under a tree.
    Cast,
    /// Above the canopy (Ethan on his tower, effects, pop-ups).
    Over,
}

#[derive(Clone, Debug)]
pub struct Sprite {
    /// Stable for as long as the thing exists: the placement id.
    pub key: u32,
    pub sheet: SheetId,
    pub anim: String,
    pub frame: usize,
    /// The anchor point, in tiles.
    pub pos: Vec2,
    pub layer: Layer,
    pub z_bias: i32,
}

/// A speech bubble over somebody's head.
#[derive(Clone, Debug, PartialEq)]
pub struct Bubble {
    pub at: Vec2,
    pub text: String,
    pub colour: (u8, u8, u8),
}

#[derive(Clone)]
pub struct TownRef {
    pub key: String,
    pub town: Arc<Town>,
    /// Its scenery (built once, shared with the presentation, which needs
    /// the decal slots).
    pub art: Arc<WorldArt>,
}

#[derive(Clone, Debug, Default)]
pub struct MapModel {
    /// Markers: where, what letter, what colour.
    pub markers: Vec<(Vec2, char, (u8, u8, u8))>,
}

#[derive(Default)]
pub struct FrameModel {
    pub town: Option<TownRef>,
    /// The town point at the field's top left.
    pub cam: Vec2,
    pub decals: Vec<u32>,
    /// The flicker of candles and the ripple of flags (Silkie only).
    pub flicker: u32,
    pub look: Option<Look>,
    pub sprites: Vec<Sprite>,
    pub bubbles: Vec<Bubble>,
    pub top: String,
    pub bottom: String,
    pub overlays: Vec<Overlay>,
    pub title: bool,
    pub map: Option<MapModel>,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct FrameStats {
    pub bytes: usize,
    pub commands: u64,
    pub placements: u32,
    pub deletions: u32,
    pub uploads: u32,
    pub upload_bytes: usize,
    pub vector_edits: u32,
    pub vector_bytes: usize,
    pub vector_full_redraws: u32,
    pub text_bytes: usize,
    pub clipped: u32,
    pub chunks_sent: u32,
    pub chunks_dropped: u32,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Totals {
    pub frames: u64,
    pub bytes: u64,
    pub placements: u64,
    pub uploads: u64,
    pub upload_bytes: u64,
    pub vector_edits: u64,
    pub vector_bytes: u64,
    pub chunks_sent: u64,
}

#[derive(Clone, Copy, Debug)]
pub struct RenderConfig {
    /// Scenery and sprites as vector images (Silkie).
    pub vector: bool,
    pub uploader: Uploader,
    pub sync_output: bool,
    /// Translucent backgrounds behind text (Silkie cell alpha).
    pub cell_alpha: bool,
    /// HDR paints in the vector scenery (Silkie with HDR).
    pub hdr: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Placed {
    row: u32,
    col: u32,
    place: Place,
}

struct SheetState {
    spec: SheetSpec,
    image: u32,
}

struct World {
    key: String,
    art: Arc<WorldArt>,
    /// Tiles per chunk side, and pixels per tile, the chunks were cut for.
    tiles: u32,
    px: u32,
    cw: u32,
    ch: u32,
    pieces: Vec<Vec<usize>>,
    slots: Vec<Vec<usize>>,
    /// Chunk images in the terminal: (chunk, layer).
    loaded: BTreeSet<(u32, u8)>,
    /// Decal states as last sent.
    sent: Vec<u32>,
    sent_flicker: u32,
}

impl World {
    fn image(chunk: u32, layer: u8) -> u32 {
        IMG_CHUNKS + chunk * 2 + layer as u32
    }

    fn layer_of(l: u8) -> ArtLayer {
        if l == 0 { ArtLayer::Ground } else { ArtLayer::Canopy }
    }
}

/// The most pixels kept in the terminal at once, in bytes of RGBA: under
/// kitty's default image storage quota (320 MB), past which it evicts
/// images and placements of them fail. Town chunks get what the sprite
/// sheets leave.
pub const RASTER_BUDGET: u64 = 256 * 1024 * 1024;

/// A townsperson's sheet nobody has worn for this many frames is deleted.
const SHEET_IDLE_FRAMES: u64 = 900;

/// How many tiles a chunk spans at `px` pixels per tile: about 640 pixels
/// square (big enough for few placements, small enough to send quickly).
pub fn chunk_tiles(px: u32) -> u32 {
    ((640.0 / px.max(1) as f32).round() as u32).clamp(6, 32)
}

pub struct Renderer {
    pub cfg: RenderConfig,
    pub layout: Option<Layout>,
    sheets: BTreeMap<SheetId, SheetState>,
    sheets_for: Option<u32>,
    look: Option<Look>,
    raccoon_slot: usize,
    uploaded: BTreeSet<u32>,
    placed: BTreeMap<(u32, u32), Placed>,
    world: Option<World>,
    map_for: Option<(String, Layout)>,
    title_for: Option<Layout>,
    text_rows: BTreeMap<u32, String>,
    overlay_rects: Vec<(u32, u32, u32, u32)>,
    overlay_text: Vec<String>,
    banner: Option<(u32, u32, u32)>,
    pub last: FrameStats,
    pub totals: Totals,
    pub last_error: Option<String>,
    pub error_count: u64,
    /// Bytes of pixels of each raster image in the terminal.
    held: BTreeMap<u32, u64>,
    /// The frame each sheet was last shown in.
    sheet_used: BTreeMap<SheetId, u64>,
    frame_no: u64,
    peak_held: u64,
}

impl Renderer {
    pub fn new(cfg: RenderConfig) -> Self {
        Renderer {
            cfg,
            layout: None,
            sheets: BTreeMap::new(),
            sheets_for: None,
            look: None,
            raccoon_slot: 0,
            uploaded: BTreeSet::new(),
            placed: BTreeMap::new(),
            world: None,
            map_for: None,
            title_for: None,
            text_rows: BTreeMap::new(),
            overlay_rects: Vec::new(),
            overlay_text: Vec::new(),
            banner: None,
            last: FrameStats::default(),
            totals: Totals::default(),
            last_error: None,
            error_count: 0,
            held: BTreeMap::new(),
            sheet_used: BTreeMap::new(),
            frame_no: 0,
            peak_held: 0,
        }
    }

    /// Bytes of pixels held in the terminal (zero on Silkie, where the
    /// images are vector records).
    pub fn raster_bytes(&self) -> u64 {
        self.held.values().sum()
    }

    /// The most pixels held at once so far.
    pub fn peak_raster_bytes(&self) -> u64 {
        self.peak_held
    }

    fn forget(&mut self, out: &mut Vec<u8>, id: u32) {
        kitty_gfx::delete_image(out, id);
        self.uploaded.remove(&id);
        self.held.remove(&id);
        self.placed.retain(|k, _| k.0 != id);
    }

    pub fn set_layout(&mut self, l: Option<Layout>) {
        self.layout = l;
    }

    /// Forgets everything the terminal held (after a suspend): it is all
    /// sent again.
    pub fn invalidate(&mut self) {
        self.uploaded.clear();
        self.placed.clear();
        self.held.clear();
        self.sheet_used.clear();
        self.sheets.clear();
        self.sheets_for = None;
        self.look = None;
        self.world = None;
        self.map_for = None;
        self.title_for = None;
        self.reset_text();
    }

    /// The terminal reported an error for one of our images (an image
    /// evicted under storage pressure says ENOENT): send it again.
    pub fn on_graphics_error(&mut self, id: u32, message: &str) {
        tracing::warn!("terminal reported a graphics error for image {id:#x}: {message}");
        self.error_count = self.error_count.saturating_add(1);
        self.last_error = Some(format!("{id:#x}: {message}"));
        if message.starts_with("ENOENT") {
            self.uploaded.remove(&id);
            self.placed.retain(|k, _| k.0 != id);
            if id >= IMG_CHUNKS {
                if let Some(w) = &mut self.world {
                    let c = (id - IMG_CHUNKS) / 2;
                    let l = ((id - IMG_CHUNKS) % 2) as u8;
                    w.loaded.remove(&(c, l));
                }
            }
            if IMG_RACCOON.contains(&id) {
                self.look = None;
            }
            self.sheets.retain(|_, s| s.image != id);
            if id == IMG_MAP {
                self.map_for = None;
            }
            if id == IMG_TITLE {
                self.title_for = None;
            }
        }
    }

    /// Every fixed image id the renderer may have created (chunks are
    /// deleted by `teardown`).
    pub fn image_ids() -> Vec<u32> {
        let mut v: Vec<u32> = SheetId::all().iter().map(|s| s.image_id()).collect();
        v.extend(IMG_RACCOON);
        v.extend([IMG_POINTER, IMG_TITLE, IMG_MAP]);
        v
    }

    pub fn pointer_image(&mut self, px: u32) -> (Vec<u8>, (u32, u32)) {
        let mut out = Vec::new();
        let (pm, hot) = crate::art::fx::feather_cursor(px);
        let s = self.cfg.uploader.pixmap(&mut out, IMG_POINTER, &pm);
        let mut st = FrameStats::default();
        self.note_upload(IMG_POINTER, s, &mut st);
        (out, hot)
    }

    fn note_upload(&mut self, id: u32, s: EncodeStats, st: &mut FrameStats) {
        self.uploaded.insert(id);
        self.placed.retain(|k, _| k.0 != id);
        st.uploads += 1;
        st.upload_bytes += s.bytes as usize;
        st.commands += s.commands;
    }

    /// Sends a painted sheet as image `id`: vector records or pixels.
    fn send_painted(&mut self, out: &mut Vec<u8>, id: u32, p: &Painted, st: &mut FrameStats) {
        if let Some(pm) = &p.pixels {
            let s = self.cfg.uploader.pixmap(out, id, pm);
            self.note_upload(id, s, st);
            self.held.insert(id, pm.width() as u64 * pm.height() as u64 * 4);
        } else {
            let mut scene = Scene::new(p.width as f32, p.height as f32);
            for (eid, e) in &p.elements {
                scene.set(*eid, e.clone());
            }
            let s = kitty_gfx::vector_transmit(out, id, p.width as f64, p.height as f64, &scene.records(), Quiet::ErrorsOnly);
            st.vector_bytes += s.bytes as usize;
            self.note_upload(id, s, st);
        }
    }

    fn ensure_sheet(&mut self, out: &mut Vec<u8>, id: SheetId, res: u32, look: &Look, st: &mut FrameStats) {
        if self.sheets_for != Some(res) {
            // A new size: every sheet is painted again. Delete the old ones
            // first, so the terminal never holds both sizes at once (kitty
            // evicts images past its storage quota).
            self.sheets_for = Some(res);
            let old: Vec<u32> = self.sheets.values().map(|s| s.image).chain(IMG_RACCOON).filter(|i| self.uploaded.contains(i)).collect();
            for i in old {
                self.forget(out, i);
            }
            self.sheets.clear();
            self.look = None;
        }
        if id == SheetId::Raccoon {
            if self.look.as_ref() == Some(look) && self.sheets.contains_key(&id) {
                return;
            }
            // Paint into the other image, so the old one stays on screen
            // until the new one is there.
            let old = self.sheets.get(&id).map(|s| s.image);
            self.raccoon_slot = 1 - self.raccoon_slot;
            let img = IMG_RACCOON[self.raccoon_slot];
            let p = paint_sheet(id, res, look, self.cfg.vector);
            self.send_painted(out, img, &p, st);
            self.sheets.insert(id, SheetState { spec: p.spec, image: img });
            self.look = Some(*look);
            if let Some(o) = old.filter(|&o| o != img) {
                // Its placement moves to the new image this frame.
                self.placed.retain(|k, _| k.0 != o);
                self.held.remove(&o);
            }
            return;
        }
        if self.sheets.contains_key(&id) && self.uploaded.contains(&id.image_id()) {
            return;
        }
        let p = paint_sheet(id, res, look, self.cfg.vector);
        self.send_painted(out, id.image_id(), &p, st);
        self.sheets.insert(id, SheetState { spec: p.spec, image: id.image_id() });
    }

    /// Whether drawing `f` means sending a lot first (a new size, the first
    /// frame): the app puts a banner up meanwhile over slow links.
    pub fn needs_heavy_upload(&self, f: &FrameModel) -> bool {
        let Some(l) = self.layout else { return false };
        (self.sheets_for != Some(l.tile_px) || f.sprites.iter().any(|s| !self.sheets.contains_key(&s.sheet))) && !self.cfg.uploader.temp_file
    }

    pub fn render(&mut self, f: &FrameModel) -> Vec<u8> {
        let mut st = FrameStats::default();
        let mut out = Vec::new();
        let Some(l) = self.layout else {
            return out;
        };
        let res = l.tile_px;
        self.frame_no += 1;
        // A new size: the old chunks go before anything new is sent.
        if self.world.as_ref().is_some_and(|w| w.px != res || w.tiles != chunk_tiles(res)) {
            self.drop_world(&mut out, &mut st);
        }
        let look = f.look.unwrap_or_else(|| crate::sim::costume::Costume::default().look(0));
        let needed: BTreeSet<SheetId> = f.sprites.iter().map(|s| s.sheet).collect();
        for &id in &needed {
            self.ensure_sheet(&mut out, id, res, &look, &mut st);
            self.sheet_used.insert(id, self.frame_no);
        }
        // Townsfolk sheets nobody has worn for a while go.
        let idle: Vec<SheetId> = self
            .sheet_used
            .iter()
            .filter(|(id, at)| matches!(id, SheetId::Folk(_)) && self.frame_no - **at > SHEET_IDLE_FRAMES)
            .map(|(id, _)| *id)
            .collect();
        for id in idle {
            self.sheet_used.remove(&id);
            if let Some(s) = self.sheets.remove(&id) {
                self.forget(&mut out, s.image);
            }
        }
        let mut desired: BTreeMap<(u32, u32), Placed> = BTreeMap::new();
        match &f.town {
            Some(t) => self.world_frame(&mut out, &l, t, f, &mut desired, &mut st),
            None => self.drop_world(&mut out, &mut st),
        }
        if let (Some(m), Some(t)) = (&f.map, &f.town) {
            self.map_frame(&mut out, &l, t, &mut desired, &mut st);
            let _ = m;
        }
        if f.title {
            self.title_frame(&mut out, &l, &mut desired, &mut st);
        }
        st.clipped = self.sprite_placements(&l, f.cam, &f.sprites, &mut desired);
        self.apply_placements(&mut out, desired, &mut st);
        let mut text = Vec::new();
        if st.upload_bytes == 0 {
            if let Some((row, col, w)) = self.banner.take() {
                let _ = write!(text, "\x1b[{};{}H\x1b[0m{}", row + 1, col + 1, " ".repeat(w as usize));
            }
        }
        self.status_lines(&mut text, &l, f);
        let mut overlays = self.bubble_overlays(&l, f);
        if let Some(m) = &f.map {
            overlays.extend(map_markers(&l, f.town.as_ref().map(|t| t.town.size()), m));
        }
        overlays.extend(f.overlays.iter().cloned());
        self.overlays(&mut text, &l, &overlays);
        st.text_bytes = text.len();
        if st.upload_bytes > 0 {
            // Big uploads stream for a while: the text first, outside a
            // synchronized update, so the screen is not frozen meanwhile.
            text.extend_from_slice(&out);
            out = text;
        } else {
            out.extend_from_slice(&text);
            if !out.is_empty() && self.cfg.sync_output {
                let mut wrapped = Vec::with_capacity(out.len() + 16);
                wrapped.extend_from_slice(b"\x1b[?2026h");
                wrapped.extend_from_slice(&out);
                wrapped.extend_from_slice(b"\x1b[?2026l");
                out = wrapped;
            }
        }
        st.bytes = out.len();
        self.peak_held = self.peak_held.max(self.raster_bytes());
        self.count(&st);
        self.last = st;
        out
    }

    fn count(&mut self, st: &FrameStats) {
        if st.bytes == 0 {
            return;
        }
        self.totals.frames += 1;
        self.totals.bytes += st.bytes as u64;
        self.totals.placements += st.placements as u64;
        self.totals.uploads += st.uploads as u64;
        self.totals.upload_bytes += st.upload_bytes as u64;
        self.totals.vector_edits += st.vector_edits as u64;
        self.totals.vector_bytes += st.vector_bytes as u64;
        self.totals.chunks_sent += st.chunks_sent as u64;
    }

    /// A one-line banner in the middle of the field, cleared by the next
    /// frame without uploads. For long uploads over slow links.
    pub fn banner(&mut self, msg: &str) -> Vec<u8> {
        let Some(l) = self.layout else { return Vec::new() };
        let w = text::width(msg) as u32 + 4;
        let row = l.field_row + l.field_rows / 2;
        let col = l.cols.saturating_sub(w) / 2;
        self.banner = Some((row, col, w));
        let mut out = Vec::new();
        let _ = write!(out, "\x1b[{};{}H\x1b[0m\x1b[48;2;{};{};{}m\x1b[38;2;255;190;90m  {msg}  \x1b[0m", row + 1, col + 1, text::BG.0, text::BG.1, text::BG.2);
        out
    }

    fn drop_world(&mut self, out: &mut Vec<u8>, st: &mut FrameStats) {
        if let Some(w) = self.world.take() {
            for (c, l) in w.loaded {
                self.forget(out, World::image(c, l));
                st.chunks_dropped += 1;
            }
        }
    }

    fn world_frame(&mut self, out: &mut Vec<u8>, l: &Layout, t: &TownRef, f: &FrameModel, desired: &mut BTreeMap<(u32, u32), Placed>, st: &mut FrameStats) {
        let tiles = chunk_tiles(l.tile_px);
        let stale = self.world.as_ref().is_none_or(|w| w.key != t.key || w.tiles != tiles || w.px != l.tile_px);
        if stale {
            self.drop_world(out, st);
            let art = t.art.clone();
            let (cw, ch, pieces, slots) = scenery::chunk_index(&art, tiles);
            self.world = Some(World {
                key: t.key.clone(),
                art,
                tiles,
                px: l.tile_px,
                cw,
                ch,
                pieces,
                slots,
                loaded: BTreeSet::new(),
                sent: f.decals.clone(),
                sent_flicker: f.flicker,
            });
        }
        let vector = self.cfg.vector;
        let not_chunks: u64 = self.held.iter().filter(|(id, _)| **id < IMG_CHUNKS).map(|(_, b)| *b).sum();
        let chunk_budget = RASTER_BUDGET.saturating_sub(not_chunks);
        let w = self.world.as_mut().expect("built above");
        let view = l.view();
        let tf = tiles as f32;
        let span = |a: f32, b: f32, n: u32| -> (u32, u32) {
            let lo = (a / tf).floor().max(0.0) as u32;
            let hi = ((b / tf).floor().max(0.0) as u32).min(n.saturating_sub(1));
            (lo.min(hi), hi)
        };
        let (vx0, vx1) = span(f.cam.x, f.cam.x + view.x, w.cw);
        let (vy0, vy1) = span(f.cam.y, f.cam.y + view.y, w.ch);
        // Chunks well outside the view are let go: the terminal's image
        // memory is not ours to fill. Vector chunks are small to keep and
        // cheap to send again, so more of them stay; pixel chunks also stay
        // within a budget under kitty's image storage quota (320 MB by
        // default), farthest first.
        let margin = if vector { 2 } else { 1 };
        let keep = |c: u32| {
            let (cx, cy) = (c % w.cw, c / w.cw);
            cx + margin >= vx0 && cx <= vx1 + margin && cy + margin >= vy0 && cy <= vy1 + margin
        };
        let mut gone: Vec<(u32, u8)> = w.loaded.iter().copied().filter(|(c, _)| !keep(*c)).collect();
        if !vector {
            let side = (tiles * l.tile_px) as u64;
            let per = side * side * 4;
            let kept = w.loaded.len() as u64 - gone.len() as u64;
            let over = (kept * per).saturating_sub(chunk_budget);
            if over > 0 {
                let (mx, my) = ((vx0 + vx1) as f32 / 2.0, (vy0 + vy1) as f32 / 2.0);
                let mut rest: Vec<(f32, (u32, u8))> = w
                    .loaded
                    .iter()
                    .copied()
                    .filter(|k| !gone.contains(k))
                    .filter(|(c, _)| {
                        let (cx, cy) = (c % w.cw, c / w.cw);
                        !(cx >= vx0 && cx <= vx1 && cy >= vy0 && cy <= vy1)
                    })
                    .map(|k| {
                        let (cx, cy) = ((k.0 % w.cw) as f32, (k.0 / w.cw) as f32);
                        (-((cx - mx).powi(2) + (cy - my).powi(2)), k)
                    })
                    .collect();
                rest.sort_by(|a, b| a.0.total_cmp(&b.0));
                let n = over.div_ceil(per) as usize;
                gone.extend(rest.into_iter().take(n).map(|(_, k)| k));
            }
        }
        for (c, ly) in gone {
            let id = World::image(c, ly);
            kitty_gfx::delete_image(out, id);
            w.loaded.remove(&(c, ly));
            self.uploaded.remove(&id);
            self.held.remove(&id);
            self.placed.retain(|k, _| k.0 != id);
            st.chunks_dropped += 1;
        }
        // Decal changes: patches on Silkie, re-sent chunks elsewhere.
        let mut changed: Vec<usize> = Vec::new();
        for (i, s) in f.decals.iter().enumerate() {
            if w.sent.get(i) != Some(s) {
                changed.push(i);
            }
        }
        w.sent = f.decals.clone();
        let flicker_changed = vector && f.flicker != w.sent_flicker;
        let old_flicker = w.sent_flicker;
        w.sent_flicker = f.flicker;
        let view_rect = crate::world::Rect::new(f.cam.x, f.cam.y, view.x, view.y);
        let mut resend: BTreeSet<(u32, u8)> = BTreeSet::new();
        let mut edits: BTreeMap<(u32, u8), String> = BTreeMap::new();
        for &(c, ly) in &w.loaded {
            for &si in &w.slots[c as usize] {
                let slot = &w.art.slots[si];
                if World::layer_of(ly) != slot.layer {
                    continue;
                }
                let animated = matches!(slot.kind, DecalKind::Pumpkin { .. } | DecalKind::Flag { .. });
                // Only what is on screen flickers: elsewhere the chunk keeps
                // whatever frame it had.
                let flick = flicker_changed && animated && slot.bounds.intersects(&view_rect);
                let state_changed = changed.contains(&si);
                if !state_changed && !flick {
                    continue;
                }
                if vector {
                    let state = f.decals.get(si).copied().unwrap_or(0);
                    let now = scenery::decal_elements(slot, state, f.flicker, w.art.hdr);
                    // A flicker changes a part or two of the decal: send just
                    // those.
                    let before = if state_changed { Vec::new() } else { scenery::decal_elements(slot, state, old_flicker, w.art.hdr) };
                    let rec = edits.entry((c, ly)).or_default();
                    for (k, (id, e)) in now.into_iter().enumerate() {
                        if before.get(k).is_some_and(|(bid, be)| *bid == id && *be == e) {
                            continue;
                        }
                        rec.push_str(&Edit::Set(id, e).record());
                        rec.push('\n');
                    }
                } else if state_changed {
                    resend.insert((c, ly));
                }
            }
        }
        edits.retain(|_, rec| !rec.is_empty());
        for ((c, ly), rec) in edits {
            let s = kitty_gfx::vector_edit(out, World::image(c, ly), &rec, Quiet::ErrorsOnly);
            st.vector_edits += rec.lines().count() as u32;
            st.vector_bytes += s.bytes as usize;
            st.commands += s.commands;
        }
        for k in resend {
            w.loaded.remove(&k);
        }
        // Send what the view needs, then a ring around it, a little at a
        // time, so walking on finds the next chunks already there.
        // Prefetching stops where pixel chunks would go over budget (the
        // budget would only drop them again).
        let per_chunk = if vector { 0 } else { (tiles * l.tile_px) as u64 * (tiles * l.tile_px) as u64 * 4 };
        let mut held_chunks = w.loaded.len() as u64 * per_chunk;
        let mut prefetch_budget = 1;
        for ring in [false, true] {
            let (x0, x1, y0, y1) = if ring {
                (vx0.saturating_sub(1), (vx1 + 1).min(w.cw - 1), vy0.saturating_sub(1), (vy1 + 1).min(w.ch - 1))
            } else {
                (vx0, vx1, vy0, vy1)
            };
            for cy in y0..=y1 {
                for cx in x0..=x1 {
                    let c = cy * w.cw + cx;
                    for ly in 0..2u8 {
                        if w.loaded.contains(&(c, ly)) {
                            continue;
                        }
                        if ring {
                            if prefetch_budget == 0 || held_chunks + per_chunk > chunk_budget {
                                continue;
                            }
                            prefetch_budget -= 1;
                        }
                        held_chunks += per_chunk;
                        let origin = Vec2::new((cx * tiles) as f32, (cy * tiles) as f32);
                        let scene = scenery::chunk_scene(&w.art, World::layer_of(ly), origin, tiles, l.tile_px, &w.pieces[c as usize], &w.slots[c as usize], &f.decals, f.flicker);
                        let id = World::image(c, ly);
                        let s = if vector {
                            let s = kitty_gfx::vector_transmit(out, id, scene.width as f64, scene.height as f64, &scene.records(), Quiet::ErrorsOnly);
                            st.vector_bytes += s.bytes as usize;
                            s
                        } else {
                            let side = tiles * l.tile_px;
                            let mut pm = Pixmap::new(side, side).expect("chunk size");
                            scene.render(&mut pm, Transform::identity());
                            self.held.insert(id, side as u64 * side as u64 * 4);
                            self.cfg.uploader.pixmap(out, id, &pm)
                        };
                        w.loaded.insert((c, ly));
                        self.uploaded.insert(id);
                        self.placed.retain(|k, _| k.0 != id);
                        st.uploads += 1;
                        st.upload_bytes += s.bytes as usize;
                        st.commands += s.commands;
                        st.chunks_sent += 1;
                    }
                }
            }
        }
        // Place the visible chunks, clipped to the field.
        let side = (tiles * l.tile_px) as f32;
        for cy in vy0..=vy1 {
            for cx in vx0..=vx1 {
                let c = cy * w.cw + cx;
                let origin = Vec2::new((cx * tiles) as f32, (cy * tiles) as f32);
                let (x, y) = l.world_to_px(f.cam, origin);
                for ly in 0..2u8 {
                    let id = World::image(c, ly);
                    if let Some(p) = clip_place(l, x, y, [0, 0, side as u32, side as u32], id, 1, if ly == 0 { Z_GROUND } else { Z_CANOPY }) {
                        desired.insert((id, 1), p);
                    }
                }
            }
        }
    }

    fn map_frame(&mut self, out: &mut Vec<u8>, l: &Layout, t: &TownRef, desired: &mut BTreeMap<(u32, u32), Placed>, st: &mut FrameStats) {
        let size = t.town.size();
        // Fit the whole town in the field.
        let k = (l.field_w as f32 / size.x).min(l.field_h as f32 / size.y);
        let (w, h) = ((size.x * k).floor().max(1.0) as u32, (size.y * k).floor().max(1.0) as u32);
        let fresh = self.map_for.as_ref().is_none_or(|(key, ml)| *key != t.key || (!self.cfg.vector && ml != l)) || !self.uploaded.contains(&IMG_MAP);
        if fresh {
            let scene = crate::gfx::minimap::scene(&t.town);
            let s = if self.cfg.vector {
                let s = kitty_gfx::vector_transmit(out, IMG_MAP, scene.width as f64, scene.height as f64, &scene.records(), Quiet::ErrorsOnly);
                st.vector_bytes += s.bytes as usize;
                s
            } else {
                let mut pm = Pixmap::new(w, h).expect("map size");
                let s = w as f32 / scene.width;
                scene.render(&mut pm, Transform::from_scale(s, s));
                self.held.insert(IMG_MAP, w as u64 * h as u64 * 4);
                self.cfg.uploader.pixmap(out, IMG_MAP, &pm)
            };
            self.note_upload(IMG_MAP, s, st);
            self.map_for = Some((t.key.clone(), *l));
        }
        let (ox, oy) = l.origin_px();
        let x = ox + (l.field_w - w) as f32 / 2.0;
        let y = oy + (l.field_h - h) as f32 / 2.0;
        if let Some((row, col, xo, yo)) = l.cell_at(x, y) {
            let cols = (w / l.cell_w).max(1);
            desired.insert(
                (IMG_MAP, 1),
                Placed {
                    row,
                    col,
                    place: Place {
                        id: IMG_MAP,
                        placement: 1,
                        src: None,
                        offset: (xo, yo),
                        z: Z_MAP,
                        cols: self.cfg.vector.then_some(cols),
                        rows: None,
                    },
                },
            );
        }
    }

    fn title_frame(&mut self, out: &mut Vec<u8>, l: &Layout, desired: &mut BTreeMap<(u32, u32), Placed>, st: &mut FrameStats) {
        use crate::gfx::title::{LOGO_H, LOGO_W};
        let cols = ((l.field_cols as f32) * 0.72).round().max(1.0) as u32;
        let w = cols * l.cell_w;
        let h = (w as f32 * LOGO_H / LOGO_W).round() as u32;
        if self.cfg.vector {
            if !self.uploaded.contains(&IMG_TITLE) {
                let scene = crate::gfx::title::logo_scene();
                let s = kitty_gfx::vector_transmit(out, IMG_TITLE, LOGO_W as f64, LOGO_H as f64, &scene.records(), Quiet::ErrorsOnly);
                st.vector_bytes += s.bytes as usize;
                self.note_upload(IMG_TITLE, s, st);
            }
        } else if self.title_for != Some(*l) || !self.uploaded.contains(&IMG_TITLE) {
            let pm = crate::gfx::title::logo(w, h);
            let s = self.cfg.uploader.pixmap(out, IMG_TITLE, &pm);
            self.note_upload(IMG_TITLE, s, st);
            self.title_for = Some(*l);
        }
        let (ox, oy) = l.origin_px();
        let x = ox + ((l.field_cols - cols) / 2 * l.cell_w) as f32;
        let y = oy + l.field_h as f32 * 0.08;
        if let Some((row, col, xo, yo)) = l.cell_at(x, y) {
            desired.insert(
                (IMG_TITLE, 1),
                Placed {
                    row,
                    col,
                    place: Place {
                        id: IMG_TITLE,
                        placement: 1,
                        src: None,
                        offset: (xo, yo),
                        z: Z_TITLE,
                        cols: self.cfg.vector.then_some(cols),
                        rows: None,
                    },
                },
            );
        }
    }

    /// Places every sprite, clipped to the field. Returns how many were
    /// clipped.
    fn sprite_placements(&self, l: &Layout, cam: Vec2, sprites: &[Sprite], desired: &mut BTreeMap<(u32, u32), Placed>) -> u32 {
        let mut clipped = 0;
        for sp in sprites {
            let Some(sheet) = self.sheets.get(&sp.sheet) else { continue };
            let Some(frames) = sheet.spec.anims.get(&sp.anim) else { continue };
            if frames.is_empty() {
                continue;
            }
            let frame = frames[sp.frame % frames.len()];
            let cols = sheet.spec.cols.max(1);
            let src = [(frame % cols) * sheet.spec.cell_w, (frame / cols) * sheet.spec.cell_h, sheet.spec.cell_w, sheet.spec.cell_h];
            let (ax, ay) = l.world_to_px(cam, sp.pos);
            let x0 = (ax - sheet.spec.anchor_x).round();
            let y0 = (ay - sheet.spec.anchor_y).round();
            let foot = ay.round() as i32;
            let z = match sp.layer {
                Layer::Decal => Z_DECAL + sp.z_bias,
                Layer::Body => Z_SPRITE + foot * 16 + sp.z_bias,
                Layer::Cast => Z_CAST + foot * 16 + sp.z_bias,
                Layer::Over => Z_OVER + foot * 16 + sp.z_bias,
            };
            if let Some(p) = clip_place(l, x0, y0, src, sheet.image, sp.key, z) {
                if p.place.src.is_some_and(|s| s[2] != src[2] || s[3] != src[3]) {
                    clipped += 1;
                }
                desired.insert((sheet.image, sp.key), p);
            }
        }
        clipped
    }

    fn apply_placements(&mut self, out: &mut Vec<u8>, desired: BTreeMap<(u32, u32), Placed>, st: &mut FrameStats) {
        let stale: Vec<(u32, u32)> = self.placed.keys().filter(|k| !desired.contains_key(k)).copied().collect();
        for k in stale {
            let s = kitty_gfx::delete_placement(out, k.0, k.1);
            st.commands += s.commands;
            st.deletions += 1;
            self.placed.remove(&k);
        }
        for (k, p) in desired {
            if self.placed.get(&k) == Some(&p) || !self.uploaded.contains(&k.0) {
                continue;
            }
            let _ = write!(out, "\x1b[{};{}H", p.row + 1, p.col + 1);
            let s = p.place.write(out, Quiet::ErrorsOnly);
            st.commands += s.commands;
            st.placements += 1;
            self.placed.insert(k, p);
        }
        // The raccoon's previous sheet, once nothing shows it any more.
        let other = IMG_RACCOON[1 - self.raccoon_slot];
        if self.uploaded.contains(&other) && !self.placed.keys().any(|k| k.0 == other) && self.look.is_some() {
            self.forget(out, other);
        }
    }

    fn status_lines(&mut self, out: &mut Vec<u8>, l: &Layout, f: &FrameModel) {
        for (row, line) in [(0u32, &f.top), (l.rows - 1, &f.bottom)] {
            if self.text_rows.get(&row) == Some(line) {
                continue;
            }
            let _ = write!(out, "\x1b[{};1H\x1b[0m\x1b[48;2;18;14;28m\x1b[2K{}\x1b[0m", row + 1, text::fit(line, l.cols as usize));
            self.text_rows.insert(row, line.clone());
        }
    }

    /// Speech bubbles, nudged upwards where they would overlap, so two
    /// people talking at once stay readable.
    fn bubble_overlays(&self, l: &Layout, f: &FrameModel) -> Vec<Overlay> {
        let mut v = Vec::new();
        let mut taken: Vec<(u32, u32, u32, u32)> = Vec::new();
        let (fx0, fy0) = l.origin_px();
        for b in &f.bubbles {
            let (x, y) = l.world_to_px(f.cam, b.at);
            if x < fx0 || y < fy0 || x >= fx0 + l.field_w as f32 || y >= fy0 + l.field_h as f32 {
                continue;
            }
            let col = (x / l.cell_w as f32) as u32;
            let row = (y / l.cell_h as f32) as u32;
            let lines: Vec<Line> = text::wrap(&b.text, 30).into_iter().map(|t| Line::new(t, Style::Rgb(b.colour.0, b.colour.1, b.colour.2))).collect();
            let w = lines.iter().map(|ln| text::width(&ln.text) as u32).max().unwrap_or(0) + 2;
            let h = lines.len() as u32;
            let x0 = col.saturating_sub(w / 2).min(l.cols.saturating_sub(w));
            let mut bottom = row.saturating_sub(1).max(l.field_row + h - 1);
            let hits = |bottom: u32, taken: &[(u32, u32, u32, u32)]| {
                let top = bottom + 1 - h;
                taken.iter().any(|&(tx, ty, tw, th)| x0 < tx + tw + 1 && tx < x0 + w + 1 && top < ty + th && ty < top + h)
            };
            while hits(bottom, &taken) && bottom > l.field_row + h - 1 {
                bottom -= 1;
            }
            taken.push((x0, bottom + 1 - h, w, h));
            let mut o = Overlay::new(Anchor::Above(col, bottom), lines);
            o.compact = true;
            o.alpha = 78;
            v.push(o);
        }
        v
    }

    fn overlays(&mut self, out: &mut Vec<u8>, l: &Layout, overlays: &[Overlay]) {
        let rendered: Vec<(String, (u32, u32, u32, u32))> = overlays.iter().map(|o| text::render_overlay(o, l, self.cfg.cell_alpha)).collect();
        let texts: Vec<String> = rendered.iter().map(|r| r.0.clone()).collect();
        if texts == self.overlay_text {
            return;
        }
        for &(col, row, w, h) in &self.overlay_rects {
            for r in row..row + h {
                let _ = write!(out, "\x1b[{};{}H\x1b[0m{}", r + 1, col + 1, " ".repeat(w as usize));
            }
        }
        for (s, _) in &rendered {
            out.extend_from_slice(s.as_bytes());
        }
        out.extend_from_slice(b"\x1b[0m");
        self.overlay_rects = rendered.iter().map(|r| r.1).collect();
        self.overlay_text = texts;
    }

    pub fn reset_text(&mut self) {
        self.text_rows.clear();
        self.overlay_rects.clear();
        self.overlay_text.clear();
    }

    pub fn placement_count(&self) -> usize {
        self.placed.len()
    }

    pub fn placements_of(&self, image: u32) -> usize {
        self.placed.keys().filter(|k| k.0 == image).count()
    }

    pub fn is_uploaded(&self, image: u32) -> bool {
        self.uploaded.contains(&image)
    }

    pub fn chunks_loaded(&self) -> usize {
        self.world.as_ref().map_or(0, |w| w.loaded.len())
    }

    /// Takes every placement off the screen but keeps the images (a window
    /// too small to play in).
    pub fn hide(&mut self) -> Vec<u8> {
        let mut out = Vec::new();
        for (image, placement) in std::mem::take(&mut self.placed).into_keys() {
            kitty_gfx::delete_placement(&mut out, image, placement);
        }
        self.reset_text();
        out
    }

    /// Deletes every image (on exit or before handing the screen over).
    pub fn teardown(&mut self) -> Vec<u8> {
        let mut out = Vec::new();
        for id in std::mem::take(&mut self.uploaded) {
            kitty_gfx::delete_image(&mut out, id);
        }
        self.held.clear();
        if let Some(w) = &mut self.world {
            w.loaded.clear();
        }
        self.placed.clear();
        out
    }
}

/// A placement of `src` (a rectangle of image `id`) with its top left at
/// absolute pixel (x0, y0), clipped to the field. None if none of it shows.
fn clip_place(l: &Layout, x0: f32, y0: f32, src: [u32; 4], id: u32, placement: u32, z: i32) -> Option<Placed> {
    let (fx0, fy0) = l.origin_px();
    let (fx1, fy1) = (fx0 + l.field_w as f32, fy0 + l.field_h as f32);
    let x0 = x0.round();
    let y0 = y0.round();
    let cx0 = x0.max(fx0.round());
    let cy0 = y0.max(fy0.round());
    let cx1 = (x0 + src[2] as f32).min(fx1.round());
    let cy1 = (y0 + src[3] as f32).min(fy1.round());
    if cx1 <= cx0 || cy1 <= cy0 {
        return None;
    }
    let clipped = [src[0] + (cx0 - x0) as u32, src[1] + (cy0 - y0) as u32, (cx1 - cx0) as u32, (cy1 - cy0) as u32];
    let (row, col, xo, yo) = l.cell_at(cx0, cy0)?;
    Some(Placed {
        row,
        col,
        place: Place {
            id,
            placement,
            src: Some(clipped),
            offset: (xo, yo),
            z,
            cols: None,
            rows: None,
        },
    })
}

/// The map's markers as one-cell text overlays.
fn map_markers(l: &Layout, size: Option<Vec2>, m: &MapModel) -> Vec<Overlay> {
    let Some(size) = size else { return Vec::new() };
    let k = (l.field_w as f32 / size.x).min(l.field_h as f32 / size.y);
    let (w, h) = (size.x * k, size.y * k);
    let (ox, oy) = l.origin_px();
    let x0 = ox + (l.field_w as f32 - w.floor()) / 2.0;
    let y0 = oy + (l.field_h as f32 - h.floor()) / 2.0;
    m.markers
        .iter()
        .map(|(p, c, rgb)| {
            let x = x0 + p.x * k;
            let y = y0 + p.y * k;
            let col = (x / l.cell_w as f32) as u32;
            let row = (y / l.cell_h as f32) as u32 + 1;
            let mut o = Overlay::new(Anchor::Above(col, row), vec![Line::new(c.to_string(), Style::Rgb(rgb.0, rgb.1, rgb.2))]);
            o.compact = true;
            o.alpha = 90;
            o
        })
        .collect()
}

#[cfg(test)]
mod tests;
