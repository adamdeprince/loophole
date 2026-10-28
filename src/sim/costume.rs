//! The bedsheet: where it is torn, what it is patched with, how muddy and
//! wet it is, and how well the raccoon inside passes for a child.

use super::candy::Mat;
use serde::{Deserialize, Serialize};

/// Places on the sheet that can tear, as a grid over the drape seen from
/// the front: four columns by three rows, the last row being the hem. A
/// sheet is a fixed piece of cloth, so this is its whole surface.
pub const COLS: usize = 4;
pub const ROWS: usize = 3;
pub const SLOTS: usize = COLS * ROWS;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum Slot {
    Whole,
    /// A snag: a little fur shows.
    Snag,
    /// A hole: a lot of fur shows.
    Hole,
    /// Patched, with what is left of the patch's strength.
    Patched(Mat, f32),
}

impl Slot {
    pub fn is_damaged(self) -> bool {
        matches!(self, Slot::Snag | Slot::Hole)
    }
}

/// Why a sheet got worse, for the slot it lands on and the message.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Wear {
    /// Brushing through a shrub: the hem and the sides.
    Bush,
    /// Raspberry canes: anywhere.
    Thorns,
    /// A fall: the front.
    Fall,
    /// A dog, or a small boy, pulling.
    Pull,
    /// Just walking: the hem drags.
    Walk,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Costume {
    pub slots: [Slot; SLOTS],
    /// Damage built up in each slot towards the next tear.
    pub strain: [f32; SLOTS],
    /// 0 clean to 1 filthy.
    pub dirt: f32,
    /// 0 dry to 1 soaked (a wet sheet shows the fur through it).
    pub wet: f32,
    /// It smells of hen.
    pub smell: f32,
    /// Leaves stuck on, for show: they also hide holes they land on.
    pub leaves: u8,
}

impl Default for Costume {
    fn default() -> Self {
        Costume {
            slots: [Slot::Whole; SLOTS],
            strain: [0.0; SLOTS],
            dirt: 0.0,
            wet: 0.0,
            smell: 0.0,
            leaves: 0,
        }
    }
}

/// What the costume looks like, coarsely: the costume art is repainted only
/// when this changes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Look {
    pub slots: [u8; SLOTS],
    pub dirt: u8,
    pub wet: u8,
    pub leaves: u8,
    pub belly: u8,
}

impl Costume {
    pub fn holes(&self) -> usize {
        self.slots.iter().filter(|s| matches!(s, Slot::Hole)).count()
    }

    pub fn snags(&self) -> usize {
        self.slots.iter().filter(|s| matches!(s, Slot::Snag)).count()
    }

    pub fn patches(&self) -> usize {
        self.slots.iter().filter(|s| matches!(s, Slot::Patched(..))).count()
    }

    pub fn damaged(&self) -> usize {
        self.holes() + self.snags()
    }

    /// How well the sheet alone passes, 0 to 1 (a brand new sheet with a
    /// striped tail sticking out of it: 0.92).
    pub fn passing(&self) -> f32 {
        let mut p = 0.92;
        for s in &self.slots {
            p += match s {
                Slot::Whole => 0.0,
                Slot::Snag => -0.035,
                Slot::Hole => -0.085,
                Slot::Patched(m, _) => m.look(),
            };
        }
        p -= self.dirt * 0.22 + self.wet * 0.2 + self.smell * 0.15;
        p.clamp(0.0, 1.0)
    }

    /// Chooses the slot a tear of this kind lands on.
    fn slot_for(&self, why: Wear, roll: f32) -> usize {
        let pick = |cands: &[usize]| cands[((roll * cands.len() as f32) as usize).min(cands.len() - 1)];
        match why {
            Wear::Walk | Wear::Bush => {
                // The hem, and the sides of the middle row.
                pick(&[8, 9, 10, 11, 8, 11, 4, 7])
            }
            Wear::Fall => pick(&[1, 2, 5, 6, 9, 10]),
            Wear::Pull => pick(&[8, 11, 4, 7, 9, 10]),
            Wear::Thorns => ((roll * SLOTS as f32) as usize).min(SLOTS - 1),
        }
    }

    /// Wears the sheet by `amount` (1 is a snag's worth). Returns the slot
    /// that tore further, if one did.
    pub fn wear(&mut self, why: Wear, amount: f32, roll: f32) -> Option<usize> {
        let i = self.slot_for(why, roll);
        match &mut self.slots[i] {
            Slot::Patched(m, left) => {
                *left -= amount;
                if *left <= 0.0 {
                    // The patch gives way.
                    let _ = m;
                    self.slots[i] = Slot::Hole;
                    self.strain[i] = 0.0;
                    return Some(i);
                }
                None
            }
            Slot::Hole => None,
            _ => {
                self.strain[i] += amount;
                if self.strain[i] >= 1.0 {
                    self.strain[i] = 0.0;
                    self.slots[i] = match self.slots[i] {
                        Slot::Whole => Slot::Snag,
                        _ => Slot::Hole,
                    };
                    Some(i)
                } else {
                    None
                }
            }
        }
    }

    /// The slot most worth patching: holes before snags.
    pub fn worst(&self) -> Option<usize> {
        let hole = self.slots.iter().position(|s| matches!(s, Slot::Hole));
        hole.or_else(|| self.slots.iter().position(|s| matches!(s, Slot::Snag)))
    }

    /// Patches the worst slot with `m`. Returns the slot.
    pub fn patch(&mut self, m: Mat) -> Option<usize> {
        let i = self.worst()?;
        self.slots[i] = Slot::Patched(m, m.strength());
        self.strain[i] = 0.0;
        Some(i)
    }

    pub fn look(&self, belly: u8) -> Look {
        let mut slots = [0u8; SLOTS];
        for (i, s) in self.slots.iter().enumerate() {
            slots[i] = match s {
                Slot::Whole => 0,
                Slot::Snag => 1,
                Slot::Hole => 2,
                Slot::Patched(m, _) => 3 + *m as u8,
            };
        }
        Look {
            slots,
            dirt: (self.dirt * 3.0).round() as u8,
            wet: (self.wet * 2.0).round() as u8,
            leaves: self.leaves.min(6),
            belly,
        }
    }

    /// A rough fingerprint of the costume, for householders who remember
    /// what visited them: patched or torn differently enough, it is a new
    /// ghost.
    pub fn fingerprint(&self) -> u32 {
        let mut h: u32 = 0;
        for (i, s) in self.slots.iter().enumerate() {
            let v = match s {
                Slot::Whole => 0,
                Slot::Snag | Slot::Hole => 1,
                Slot::Patched(..) => 2,
            };
            h = h.wrapping_add(v << (i * 2 % 30));
        }
        h ^ ((self.dirt * 2.0) as u32) << 28
    }

    /// How different two fingerprints look: the number of slots that
    /// changed state.
    pub fn difference(a: u32, b: u32) -> u32 {
        let mut n = 0;
        for i in 0..SLOTS {
            let s = (i * 2) % 30;
            if (a >> s) & 3 != (b >> s) & 3 {
                n += 1;
            }
        }
        n
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wear_tears_then_patches_mend() {
        let mut c = Costume::default();
        let fresh = c.passing();
        let mut tore = 0;
        for k in 0..40 {
            if c.wear(Wear::Thorns, 0.6, (k as f32 * 0.37) % 1.0).is_some() {
                tore += 1;
            }
        }
        assert!(tore > 3);
        assert!(c.holes() > 0);
        assert!(c.passing() < fresh - 0.2);
        let worn = c.passing();
        while c.worst().is_some() {
            c.patch(Mat::Tape);
        }
        assert!(c.passing() > worn);
        assert_eq!(c.damaged(), 0);
    }

    #[test]
    fn patches_give_way() {
        let mut c = Costume::default();
        c.slots[8] = Slot::Hole;
        c.patch(Mat::Leaf);
        assert!(matches!(c.slots[8], Slot::Patched(Mat::Leaf, _)));
        for _ in 0..10 {
            c.wear(Wear::Walk, 0.3, 0.0);
        }
        assert_eq!(c.slots[8], Slot::Hole);
    }

    #[test]
    fn a_repatched_sheet_looks_like_another_ghost() {
        let mut c = Costume::default();
        let before = c.fingerprint();
        for i in [0, 3, 5, 8, 10] {
            c.slots[i] = Slot::Hole;
        }
        let torn = c.fingerprint();
        assert!(Costume::difference(before, torn) >= 5);
        assert_eq!(Costume::difference(torn, torn), 0);
    }
}
