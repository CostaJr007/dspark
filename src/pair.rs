//! Creator/curator pairing. Either role can be any OpenAI-compatible model.
//! Live pair is `~/.dspark/pair.toml` (TUI and engine share it).

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

pub const DEFAULT_CREATOR: &str = "gpt-4o-mini";
pub const DEFAULT_CURATOR: &str = "deepseek-v4-pro";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DsparkPair {
    #[serde(default = "default_creator")]
    pub creator: String,
    #[serde(default = "default_curator")]
    pub curator: String,
    #[serde(default = "default_true")]
    pub research: bool,
}

fn default_creator() -> String {
    DEFAULT_CREATOR.to_string()
}
fn default_curator() -> String {
    DEFAULT_CURATOR.to_string()
}
fn default_true() -> bool {
    true
}

impl Default for DsparkPair {
    fn default() -> Self {
        Self {
            creator: default_creator(),
            curator: default_curator(),
            research: true,
        }
    }
}

impl DsparkPair {
    fn home_dir() -> PathBuf {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".dspark")
    }

    /// Shared pair file written by the TUI (`/pair`) and this engine.
    pub fn pair_path() -> PathBuf {
        Self::home_dir().join("pair.toml")
    }

    /// Legacy path; only used if `pair.toml` is missing.
    pub fn config_path() -> PathBuf {
        Self::home_dir().join("config.toml")
    }

    fn load_file(path: &std::path::Path) -> Option<Self> {
        let text = fs::read_to_string(path).ok()?;
        toml::from_str::<DsparkPair>(&text).ok()
    }

    pub fn load() -> Self {
        let mut pair = Self::load_file(&Self::pair_path())
            .or_else(|| Self::load_file(&Self::config_path()))
            .unwrap_or_default();
        if let Ok(c) = std::env::var("DSPARK_CREATOR") {
            if !c.trim().is_empty() {
                pair.creator = c;
            }
        }
        if let Ok(c) = std::env::var("DSPARK_CURATOR") {
            if !c.trim().is_empty() {
                pair.curator = c;
            }
        }
        pair
    }

    pub fn save(&self) -> std::io::Result<()> {
        let path = Self::pair_path();
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }
        fs::write(
            path,
            format!(
                "creator = \"{}\"\ncurator = \"{}\"\nresearch = {}\n",
                self.creator, self.curator, self.research
            ),
        )
    }

    pub fn same_family_warning(&self) -> Option<String> {
        let a = self.creator.to_lowercase();
        let b = self.curator.to_lowercase();
        if a == b {
            return Some(
                "Creator and curator are the same model. Same-model self-review is confirmation-biased."
                    .into(),
            );
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn warns_when_roles_identical() {
        let p = DsparkPair {
            creator: "x".into(),
            curator: "x".into(),
            research: true,
        };
        assert!(p.same_family_warning().is_some());
    }

    #[test]
    fn silent_when_roles_differ() {
        assert!(DsparkPair::default().same_family_warning().is_none());
    }
}
