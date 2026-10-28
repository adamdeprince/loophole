//! What is kept between games: the journal of discoveries, the biggest haul
//! and how many Halloweens have gone by. A small JSON file in the
//! platform's data directory.

use crate::sim::discover::Id;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::path::PathBuf;

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Save {
    pub found: BTreeSet<Id>,
    pub best_haul: u64,
    pub nights: u64,
    #[serde(skip)]
    path: Option<PathBuf>,
}

impl Save {
    fn path() -> Option<PathBuf> {
        directories::ProjectDirs::from("com", "goblinreactor", "silkie-loophole").map(|d| d.data_dir().join("journal.json"))
    }

    pub fn load() -> Save {
        let path = Self::path();
        let mut s: Save = path
            .as_ref()
            .and_then(|p| std::fs::read(p).ok())
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default();
        s.path = path;
        s
    }

    pub fn in_memory() -> Save {
        Save::default()
    }

    pub fn store(&self) -> std::io::Result<()> {
        let Some(p) = &self.path else { return Ok(()) };
        if let Some(dir) = p.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let tmp = p.with_extension("json.tmp");
        std::fs::write(&tmp, serde_json::to_vec_pretty(self).map_err(std::io::Error::other)?)?;
        std::fs::rename(tmp, p)
    }

    /// Records a discovery; true if it is new to the journal.
    pub fn found(&mut self, id: Id) -> bool {
        self.found.insert(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_journal_round_trips() {
        let mut s = Save::in_memory();
        s.found(Id::Hakarl);
        s.found(Id::EthanAllen);
        s.best_haul = 812;
        let json = serde_json::to_string(&s).unwrap();
        let back: Save = serde_json::from_str(&json).unwrap();
        assert_eq!(back.found, s.found);
        assert_eq!(back.best_haul, 812);
    }
}
