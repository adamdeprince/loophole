//! The terrain grid: one terrain per tile, derived from the town
//! description, for movement, sounds underfoot and the location line.

use glam::Vec2;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum Terrain {
    #[default]
    Grass,
    /// A yard: grass somebody owns (the householder notices trampling).
    Yard,
    Lawn,
    Woods,
    Green,
    Parking,
    Plaza,
    Road,
    Sidewalk,
    Path,
    /// Anything walkable over water.
    Bridge,
    Water,
    Building,
}

impl Terrain {
    pub fn walkable(self) -> bool {
        !matches!(self, Terrain::Building)
    }

    /// How fast a creature on foot gets along, as a fraction of its pace.
    pub fn pace(self) -> f32 {
        match self {
            Terrain::Woods => 0.78,
            Terrain::Water => 0.45,
            Terrain::Grass | Terrain::Yard | Terrain::Lawn | Terrain::Green => 0.95,
            Terrain::Building => 0.0,
            _ => 1.0,
        }
    }

    /// Paved underfoot.
    pub fn paved(self) -> bool {
        matches!(
            self,
            Terrain::Road | Terrain::Sidewalk | Terrain::Path | Terrain::Bridge | Terrain::Parking | Terrain::Plaza
        )
    }
}

#[derive(Clone, Debug)]
pub struct Grid {
    pub w: i32,
    pub h: i32,
    cells: Vec<Terrain>,
}

impl Grid {
    pub fn new(w: u32, h: u32) -> Grid {
        let w = w as i32;
        let h = h as i32;
        let n = (w.max(0) as usize).checked_mul(h.max(0) as usize).expect("town size overflows memory");
        Grid {
            w,
            h,
            cells: vec![Terrain::Grass; n],
        }
    }

    pub fn in_bounds(&self, x: i32, y: i32) -> bool {
        x >= 0 && y >= 0 && x < self.w && y < self.h
    }

    /// Off the map counts as a wall.
    pub fn get(&self, x: i32, y: i32) -> Terrain {
        if self.in_bounds(x, y) {
            self.cells[(y * self.w + x) as usize]
        } else {
            Terrain::Building
        }
    }

    pub fn set(&mut self, x: i32, y: i32, t: Terrain) {
        if self.in_bounds(x, y) {
            let i = (y * self.w + x) as usize;
            self.cells[i] = t;
        }
    }

    pub fn at(&self, p: Vec2) -> Terrain {
        self.get(p.x.floor() as i32, p.y.floor() as i32)
    }

    /// Tiles whose centres lie in the rectangle `x0..x1`, `y0..y1`, clipped
    /// to the grid.
    pub fn tiles_in(&self, x0: f32, y0: f32, x1: f32, y1: f32) -> impl Iterator<Item = (i32, i32)> + use<> {
        let tx0 = (x0 - 0.5).ceil().max(0.0) as i32;
        let ty0 = (y0 - 0.5).ceil().max(0.0) as i32;
        let tx1 = ((x1 - 0.5).floor() as i32).min(self.w - 1);
        let ty1 = ((y1 - 0.5).floor() as i32).min(self.h - 1);
        (ty0..=ty1).flat_map(move |y| (tx0..=tx1).map(move |x| (x, y)))
    }

    pub fn count(&self, t: Terrain) -> usize {
        self.cells.iter().filter(|&&c| c == t).count()
    }
}

/// The centre of a tile.
pub fn centre(x: i32, y: i32) -> Vec2 {
    Vec2::new(x as f32 + 0.5, y as f32 + 0.5)
}
