//! What goes in the bag, what goes in the belly, and what goes on the
//! sheet: candy, the odd things people hand out instead, and the scraps a
//! raccoon patches its costume with.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Candy {
    CandyCorn,
    Lollipop,
    Chocolate,
    /// The legend: a full-size bar.
    FullSize,
    /// Maple sugar pressed into a leaf.
    MapleLeaf,
    /// A tiny jug of syrup, grade A amber.
    SyrupJug,
    /// Maple cotton candy, a Vermont fair classic.
    MapleCotton,
    GummyWorms,
    Caramel,
    PeanutCup,
    PopcornBall,
    Raspberry,
    /// A cube of sharp cheddar. Vermont. A raccoon is not complaining.
    Cheddar,
    Apple,
    Raisins,
    Carob,
    Toothbrush,
    Pennies,
    /// Fermented shark, from the children's mother. Other children run.
    Hakarl,
}

impl Candy {
    pub const ALL: [Candy; 19] = [
        Candy::CandyCorn,
        Candy::Lollipop,
        Candy::Chocolate,
        Candy::FullSize,
        Candy::MapleLeaf,
        Candy::SyrupJug,
        Candy::MapleCotton,
        Candy::GummyWorms,
        Candy::Caramel,
        Candy::PeanutCup,
        Candy::PopcornBall,
        Candy::Raspberry,
        Candy::Cheddar,
        Candy::Apple,
        Candy::Raisins,
        Candy::Carob,
        Candy::Toothbrush,
        Candy::Pennies,
        Candy::Hakarl,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Candy::CandyCorn => "candy corn",
            Candy::Lollipop => "a lollipop",
            Candy::Chocolate => "a fun-size chocolate",
            Candy::FullSize => "a FULL-SIZE BAR",
            Candy::MapleLeaf => "a maple sugar leaf",
            Candy::SyrupJug => "a tiny jug of maple syrup",
            Candy::MapleCotton => "maple cotton candy",
            Candy::GummyWorms => "gummy worms",
            Candy::Caramel => "a caramel",
            Candy::PeanutCup => "a peanut butter cup",
            Candy::PopcornBall => "a popcorn ball",
            Candy::Raspberry => "a raspberry",
            Candy::Cheddar => "a cube of sharp cheddar",
            Candy::Apple => "an apple",
            Candy::Raisins => "a box of raisins",
            Candy::Carob => "carob drops",
            Candy::Toothbrush => "a toothbrush",
            Candy::Pennies => "three pennies",
            Candy::Hakarl => "a cube of hákarl (fermented shark)",
        }
    }

    /// Points it is worth in the bag.
    pub fn value(self) -> u32 {
        match self {
            Candy::FullSize | Candy::Hakarl => 50,
            Candy::SyrupJug => 25,
            Candy::MapleLeaf | Candy::MapleCotton => 15,
            Candy::Chocolate | Candy::PeanutCup | Candy::Caramel => 10,
            Candy::Lollipop | Candy::GummyWorms | Candy::PopcornBall | Candy::Cheddar => 8,
            Candy::CandyCorn => 5,
            Candy::Raspberry => 3,
            Candy::Apple => 2,
            Candy::Raisins | Candy::Carob | Candy::Pennies => 1,
            Candy::Toothbrush => 0,
        }
    }

    /// How much it fills a belly.
    pub fn filling(self) -> f32 {
        match self {
            Candy::FullSize => 3.0,
            Candy::SyrupJug | Candy::PopcornBall | Candy::Apple | Candy::MapleCotton | Candy::Hakarl => 1.6,
            Candy::Raspberry => 0.25,
            Candy::Toothbrush | Candy::Pennies => 0.0,
            _ => 1.0,
        }
    }

    /// Seconds of sugar rush (running quicker) it gives.
    pub fn rush(self) -> f32 {
        match self {
            Candy::SyrupJug => 14.0,
            Candy::FullSize | Candy::MapleCotton => 10.0,
            Candy::MapleLeaf | Candy::Lollipop | Candy::GummyWorms | Candy::CandyCorn => 6.0,
            Candy::Chocolate | Candy::PeanutCup | Candy::Caramel | Candy::PopcornBall => 5.0,
            Candy::Raspberry => 1.0,
            _ => 0.0,
        }
    }

    /// How much happier eating it makes a raccoon (happiness runs from 0
    /// to 1). Hákarl is a raccoon's favourite thing in the world.
    pub fn joy(self) -> f32 {
        match self {
            Candy::Hakarl => 1.0,
            Candy::FullSize => 0.45,
            Candy::Cheddar => 0.32,
            Candy::SyrupJug => 0.3,
            Candy::MapleLeaf | Candy::MapleCotton | Candy::PeanutCup => 0.24,
            Candy::Chocolate | Candy::Caramel | Candy::PopcornBall => 0.18,
            Candy::Lollipop | Candy::GummyWorms => 0.14,
            Candy::CandyCorn => 0.1,
            Candy::Raspberry => 0.06,
            Candy::Apple => 0.05,
            Candy::Raisins | Candy::Carob => 0.02,
            Candy::Toothbrush | Candy::Pennies => 0.0,
        }
    }

    /// The raccoon loves it: a little dance after eating it.
    pub fn loved(self) -> bool {
        self.joy() >= 0.24
    }

    /// Whether a raccoon can eat it at all.
    pub fn edible(self) -> bool {
        !matches!(self, Candy::Toothbrush | Candy::Pennies)
    }

    /// What is left in the paw after eating it, if anything.
    pub fn leaves(self) -> Option<Mat> {
        match self {
            Candy::Chocolate | Candy::FullSize | Candy::PeanutCup | Candy::Caramel => Some(Mat::Wrapper),
            Candy::Lollipop | Candy::GummyWorms | Candy::MapleCotton => Some(Mat::Wrapper),
            Candy::Raisins | Candy::Carob => Some(Mat::Paper),
            _ => None,
        }
    }

    /// Sweets, as opposed to the healthy and the frankly disappointing.
    pub fn is_treat(self) -> bool {
        self.value() >= 5
    }
}

/// Things a sheet can be patched with.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Mat {
    Paper,
    /// Already chewed. Sticks to anything.
    Gum,
    Leaf,
    Wrapper,
    Tape,
    Sticker,
    Bookmark,
    Pamphlet,
    /// Toilet paper, from a tree somebody decorated.
    Tp,
    Napkin,
    /// "LOST CAT" from a telephone pole.
    Flyer,
    /// A sprig of evergreen, as the Green Mountain Boys wore in their hats.
    Sprig,
}

impl Mat {
    pub const ALL: [Mat; 12] = [
        Mat::Sprig,
        Mat::Tape,
        Mat::Gum,
        Mat::Tp,
        Mat::Bookmark,
        Mat::Pamphlet,
        Mat::Flyer,
        Mat::Paper,
        Mat::Napkin,
        Mat::Wrapper,
        Mat::Sticker,
        Mat::Leaf,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Mat::Paper => "a scrap of paper",
            Mat::Gum => "some used chewing gum",
            Mat::Leaf => "a maple leaf",
            Mat::Wrapper => "a candy wrapper",
            Mat::Tape => "duct tape",
            Mat::Sticker => "a fire station sticker",
            Mat::Bookmark => "a library bookmark",
            Mat::Pamphlet => "a pamphlet about composting",
            Mat::Tp => "a streamer of toilet paper",
            Mat::Napkin => "a napkin",
            Mat::Flyer => "a LOST CAT flyer",
            Mat::Sprig => "a sprig of evergreen, Green Mountain Boys style",
        }
    }

    pub fn short(self) -> &'static str {
        match self {
            Mat::Paper => "paper",
            Mat::Gum => "gum",
            Mat::Leaf => "leaf",
            Mat::Wrapper => "wrapper",
            Mat::Tape => "tape",
            Mat::Sticker => "sticker",
            Mat::Bookmark => "bookmark",
            Mat::Pamphlet => "pamphlet",
            Mat::Tp => "TP",
            Mat::Napkin => "napkin",
            Mat::Flyer => "flyer",
            Mat::Sprig => "sprig",
        }
    }

    /// How much wear a patch of it takes before it gives way.
    pub fn strength(self) -> f32 {
        match self {
            Mat::Tape => 5.0,
            Mat::Sprig => 3.0,
            Mat::Gum => 3.5,
            Mat::Tp => 2.4,
            Mat::Bookmark | Mat::Pamphlet | Mat::Flyer => 2.0,
            Mat::Paper | Mat::Napkin => 1.5,
            Mat::Wrapper | Mat::Sticker => 1.3,
            Mat::Leaf => 0.8,
        }
    }

    /// How a patch of it looks to somebody at the door: added to how well
    /// the costume passes (most are a little odd; a sticker is decoration).
    pub fn look(self) -> f32 {
        match self {
            Mat::Sprig => 0.03,
            Mat::Sticker => 0.01,
            Mat::Tp | Mat::Wrapper | Mat::Tape => 0.0,
            Mat::Bookmark | Mat::Paper | Mat::Napkin => -0.01,
            Mat::Leaf | Mat::Pamphlet | Mat::Flyer => -0.02,
            Mat::Gum => -0.03,
        }
    }

    /// Which to use first: the strongest that looks all right.
    pub fn preference(self) -> f32 {
        self.strength() + self.look() * 30.0
    }

    /// Patch colour in the costume art: (r, g, b).
    pub fn colour(self) -> (u8, u8, u8) {
        match self {
            Mat::Paper => (238, 232, 214),
            Mat::Gum => (240, 130, 170),
            Mat::Leaf => (214, 92, 34),
            Mat::Wrapper => (196, 60, 200),
            Mat::Tape => (168, 172, 178),
            Mat::Sticker => (220, 40, 40),
            Mat::Bookmark => (60, 110, 200),
            Mat::Pamphlet => (120, 180, 90),
            Mat::Tp => (250, 250, 250),
            Mat::Napkin => (250, 246, 236),
            Mat::Flyer => (250, 228, 90),
            Mat::Sprig => (46, 110, 60),
        }
    }
}

/// Counts of things, kept in a stable order.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Counts<K: Ord>(pub BTreeMap<K, u64>);

impl<K: Ord> Default for Counts<K> {
    fn default() -> Self {
        Counts(BTreeMap::new())
    }
}

impl<K: Ord + Copy> Counts<K> {
    pub fn add(&mut self, k: K, n: u64) {
        if n > 0 {
            let e = self.0.entry(k).or_insert(0);
            *e = e.saturating_add(n);
        }
    }

    pub fn take(&mut self, k: K) -> bool {
        match self.0.get_mut(&k) {
            Some(n) if *n > 0 => {
                *n -= 1;
                if *n == 0 {
                    self.0.remove(&k);
                }
                true
            }
            _ => false,
        }
    }

    pub fn total(&self) -> u64 {
        self.0.values().fold(0u64, |a, &b| a.saturating_add(b))
    }

    pub fn get(&self, k: K) -> u64 {
        self.0.get(&k).copied().unwrap_or(0)
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn kinds(&self) -> impl Iterator<Item = (K, u64)> + '_ {
        self.0.iter().map(|(k, v)| (*k, *v))
    }
}

/// The pillowcase.
pub type Bag = Counts<Candy>;

impl Bag {
    pub fn value(&self) -> u64 {
        self.0.iter().fold(0u64, |a, (c, n)| a.saturating_add((c.value() as u64).saturating_mul(*n)))
    }

    /// The tastiest thing in the bag (to eat first).
    pub fn best(&self) -> Option<Candy> {
        self.0
            .keys()
            .copied()
            .filter(|c| c.edible())
            .max_by(|a, b| (a.rush() + a.filling()).total_cmp(&(b.rush() + b.filling())))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_add_and_take() {
        let mut b = Bag::default();
        b.add(Candy::Chocolate, 2);
        b.add(Candy::Raisins, 1);
        assert_eq!(b.total(), 3);
        assert!(b.take(Candy::Chocolate));
        assert!(b.take(Candy::Chocolate));
        assert!(!b.take(Candy::Chocolate));
        assert_eq!(b.total(), 1);
        assert_eq!(b.best(), Some(Candy::Raisins));
    }

    #[test]
    fn counts_saturate_instead_of_wrapping() {
        let mut b = Bag::default();
        b.add(Candy::FullSize, u64::MAX);
        b.add(Candy::FullSize, 5);
        assert_eq!(b.total(), u64::MAX);
        b.add(Candy::Chocolate, 5);
        assert_eq!(b.total(), u64::MAX);
        assert_eq!(b.value(), u64::MAX);
    }
}
