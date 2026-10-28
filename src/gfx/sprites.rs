//! Sprite sheets: frames painted by the art module, laid out in a grid,
//! with named animations.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SheetSpec {
    pub name: String,
    pub file: String,
    pub cell_w: u32,
    pub cell_h: u32,
    pub cols: u32,
    pub frames: u32,
    /// The ground point of the creature within a cell, in pixels.
    pub anchor_x: f32,
    pub anchor_y: f32,
    pub anims: BTreeMap<String, Vec<u32>>,
}

/// A sheet in memory, straight (not premultiplied) RGBA.
#[derive(Clone, Debug)]
pub struct Sheet {
    pub spec: SheetSpec,
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

impl Sheet {
    pub fn anim(&self, name: &str) -> Option<&[u32]> {
        self.spec.anims.get(name).map(|v| v.as_slice())
    }

    /// The source rectangle of a frame.
    pub fn frame_rect(&self, frame: u32) -> [u32; 4] {
        let cols = self.spec.cols.max(1);
        [
            (frame % cols) * self.spec.cell_w,
            (frame / cols) * self.spec.cell_h,
            self.spec.cell_w,
            self.spec.cell_h,
        ]
    }
}
