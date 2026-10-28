//! Where the town sits in the terminal: the field fills the window between
//! a status line at the top and a message line at the bottom, and shows the
//! part of the town around the camera. Tiles are drawn at a size from a
//! fixed ladder, so that a small resize sends no new art.

use glam::Vec2;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Layout {
    pub cols: u32,
    pub rows: u32,
    pub cell_w: u32,
    pub cell_h: u32,
    /// Top-left cell of the field.
    pub field_col: u32,
    pub field_row: u32,
    pub field_cols: u32,
    pub field_rows: u32,
    pub field_w: u32,
    pub field_h: u32,
    /// Pixels per tile (a step of [`LADDER`]).
    pub tile_px: u32,
}

/// Rows kept for the status lines.
pub const HUD_ROWS: u32 = 2;

pub const MIN_COLS: u32 = 30;
pub const MIN_ROWS: u32 = HUD_ROWS + 8;

/// Tile sizes, about 11% apart.
pub const LADDER: [u32; 22] = [24, 28, 32, 36, 40, 45, 50, 56, 63, 70, 78, 87, 96, 107, 119, 133, 148, 165, 184, 205, 228, 254];

/// How many tiles of town a window shows across its height at zoom 0.
const VIEW_TILES: f32 = 15.0;

impl Layout {
    /// `zoom` steps up (closer) or down (further) the ladder from the size
    /// that shows about fifteen tiles top to bottom.
    pub fn new(cols: u32, rows: u32, cell_w: u32, cell_h: u32, zoom: i32) -> Option<Layout> {
        if cols < MIN_COLS || rows < MIN_ROWS || cell_w == 0 || cell_h == 0 {
            return None;
        }
        let field_rows = rows - HUD_ROWS;
        let field_w = cols * cell_w;
        let field_h = field_rows * cell_h;
        let want = (field_h as f32 / VIEW_TILES).min(field_w as f32 / 22.0);
        let base = LADDER.iter().rposition(|&s| s as f32 <= want * 1.04).unwrap_or(0) as i32;
        let step = (base + zoom).clamp(0, LADDER.len() as i32 - 1) as usize;
        Some(Layout {
            cols,
            rows,
            cell_w,
            cell_h,
            field_col: 0,
            field_row: 1,
            field_cols: cols,
            field_rows,
            field_w,
            field_h,
            tile_px: LADDER[step],
        })
    }

    /// The same layout with tiles no bigger than `max` pixels (a step of the
    /// ladder at or below it).
    pub fn capped(mut self, max: u32) -> Layout {
        if self.tile_px > max {
            self.tile_px = LADDER.iter().copied().rev().find(|&s| s <= max).unwrap_or(LADDER[0]);
        }
        self
    }

    /// How much town the field shows, in tiles.
    pub fn view(&self) -> Vec2 {
        Vec2::new(self.field_w as f32 / self.tile_px as f32, self.field_h as f32 / self.tile_px as f32)
    }

    pub fn origin_px(&self) -> (f32, f32) {
        ((self.field_col * self.cell_w) as f32, (self.field_row * self.cell_h) as f32)
    }

    /// Absolute pixel position of town point `p`, with the field's top left
    /// showing town point `cam`.
    pub fn world_to_px(&self, cam: Vec2, p: Vec2) -> (f32, f32) {
        let (ox, oy) = self.origin_px();
        let k = self.tile_px as f32;
        (ox + (p.x - cam.x) * k, oy + (p.y - cam.y) * k)
    }

    pub fn px_to_world(&self, cam: Vec2, x: f32, y: f32) -> Vec2 {
        let (ox, oy) = self.origin_px();
        let k = self.tile_px as f32;
        Vec2::new(cam.x + (x - ox) / k, cam.y + (y - oy) / k)
    }

    /// The cell and in-cell offset at which an image whose top-left corner
    /// is at absolute pixel (x, y) is placed. None when it would start above
    /// or left of the screen, or past its end.
    pub fn cell_at(&self, x: f32, y: f32) -> Option<(u32, u32, u32, u32)> {
        let x = x.round();
        let y = y.round();
        if x < 0.0 || y < 0.0 {
            return None;
        }
        let (x, y) = (x as u32, y as u32);
        let col = x / self.cell_w;
        let row = y / self.cell_h;
        if col >= self.cols || row >= self.rows {
            return None;
        }
        Some((row, col, x % self.cell_w, y % self.cell_h))
    }

    pub fn contains_cell(&self, col: u32, row: u32) -> bool {
        col >= self.field_col && col < self.field_col + self.field_cols && row >= self.field_row && row < self.field_row + self.field_rows
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_field_fills_the_window_between_the_status_lines() {
        let l = Layout::new(200, 56, 18, 36, 0).unwrap();
        assert_eq!(l.field_row, 1);
        assert_eq!(l.field_rows, 54);
        let v = l.view();
        assert!(v.y > 12.0 && v.y < 18.0, "{v:?}");
        let cam = Vec2::new(40.0, 50.0);
        let (x, y) = l.world_to_px(cam, Vec2::new(45.5, 52.25));
        let back = l.px_to_world(cam, x, y);
        assert!((back - Vec2::new(45.5, 52.25)).length() < 1e-3);
    }

    #[test]
    fn zoom_steps_the_ladder() {
        let a = Layout::new(160, 50, 10, 20, 0).unwrap();
        let b = Layout::new(160, 50, 10, 20, 2).unwrap();
        let c = Layout::new(160, 50, 10, 20, -2).unwrap();
        assert!(b.tile_px > a.tile_px && c.tile_px < a.tile_px);
        let d = Layout::new(160, 50, 10, 20, 99).unwrap();
        assert_eq!(d.tile_px, *LADDER.last().unwrap());
    }

    #[test]
    fn a_cap_keeps_tiles_on_the_ladder() {
        let l = Layout::new(214, 72, 14, 25, 2).unwrap().capped(96);
        assert_eq!(l.tile_px, 96);
        let small = Layout::new(80, 30, 8, 16, 0).unwrap();
        assert_eq!(small.capped(96), small);
    }

    #[test]
    fn tiny_windows_have_no_layout() {
        assert!(Layout::new(10, 5, 10, 20, 0).is_none());
    }

    #[test]
    fn placement_offsets_stay_inside_a_cell() {
        let l = Layout::new(100, 40, 9, 18, 0).unwrap();
        let (r, c, x, y) = l.cell_at(100.4, 37.0).unwrap();
        assert_eq!((r, c, x, y), (2, 11, 1, 1));
        assert!(l.cell_at(-1.0, 5.0).is_none());
    }
}
