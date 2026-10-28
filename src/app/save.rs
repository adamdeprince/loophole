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
        let home = std::env::var_os("HOME").map(PathBuf::from);
        let xdg = std::env::var_os("XDG_DATA_HOME").map(PathBuf::from);
        data_dir(cfg!(target_os = "macos"), home, xdg).map(|d| d.join("journal.json"))
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

/// The platform's data directory for the game: on macOS
/// `~/Library/Application Support/com.goblinreactor.silkie-loophole`, and
/// elsewhere `$XDG_DATA_HOME/silkie-loophole` (an absolute `XDG_DATA_HOME`
/// only, as the XDG base directory specification says), falling back to
/// `~/.local/share/silkie-loophole`. None without a home directory: the
/// journal is then kept in memory for the session.
fn data_dir(macos: bool, home: Option<PathBuf>, xdg_data_home: Option<PathBuf>) -> Option<PathBuf> {
    let home = home.filter(|h| h.is_absolute());
    if macos {
        return home.map(|h| h.join("Library/Application Support/com.goblinreactor.silkie-loophole"));
    }
    match xdg_data_home.filter(|x| x.is_absolute()) {
        Some(x) => Some(x.join("silkie-loophole")),
        None => home.map(|h| h.join(".local/share/silkie-loophole")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_journal_lives_where_each_platform_keeps_data() {
        let home = Some(PathBuf::from("/home/r"));
        assert_eq!(
            data_dir(true, home.clone(), None),
            Some(PathBuf::from("/home/r/Library/Application Support/com.goblinreactor.silkie-loophole"))
        );
        assert_eq!(data_dir(false, home.clone(), None), Some(PathBuf::from("/home/r/.local/share/silkie-loophole")));
        assert_eq!(data_dir(false, home.clone(), Some(PathBuf::from("/data"))), Some(PathBuf::from("/data/silkie-loophole")));
        // A relative XDG_DATA_HOME is ignored, as the specification says.
        assert_eq!(data_dir(false, home, Some(PathBuf::from("data"))), Some(PathBuf::from("/home/r/.local/share/silkie-loophole")));
        assert_eq!(data_dir(false, None, None), None);
        assert_eq!(data_dir(true, Some(PathBuf::from("relative")), None), None);
    }

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
