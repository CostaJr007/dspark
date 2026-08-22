//! Creator/curator pair stored in `~/.dspark/pair.toml`.

use std::path::{Path, PathBuf};

pub const DEFAULT_CREATOR: &str = "gpt-4o-mini";
pub const DEFAULT_CURATOR: &str = "deepseek-v4-pro";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DsparkPair {
    pub creator: String,
    pub curator: String,
}

impl Default for DsparkPair {
    fn default() -> Self {
        Self {
            creator: DEFAULT_CREATOR.to_string(),
            curator: DEFAULT_CURATOR.to_string(),
        }
    }
}

pub fn pair_path() -> PathBuf {
    crate::util::grok_home::grok_home().join("pair.toml")
}

pub fn load_pair() -> DsparkPair {
    load_pair_from(&pair_path())
}

pub fn load_pair_from(path: &Path) -> DsparkPair {
    let mut pair = DsparkPair::default();
    let Ok(text) = std::fs::read_to_string(path) else {
        return pair;
    };
    for line in text.lines() {
        let line = line.trim();
        if let Some(v) = line.strip_prefix("creator") {
            if let Some(v) = v.trim().strip_prefix('=') {
                let v = v.trim().trim_matches('"');
                if !v.is_empty() {
                    pair.creator = v.to_string();
                }
            }
        }
        if let Some(v) = line.strip_prefix("curator") {
            if let Some(v) = v.trim().strip_prefix('=') {
                let v = v.trim().trim_matches('"');
                if !v.is_empty() {
                    pair.curator = v.to_string();
                }
            }
        }
    }
    pair
}

pub fn save_pair(path: &Path, pair: &DsparkPair) {
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let body = format!(
        "creator = \"{}\"\ncurator = \"{}\"\nresearch = true\n",
        pair.creator, pair.curator
    );
    let _ = std::fs::write(path, body);
}

/// Create `~/.dspark/pair.toml` with product defaults if it is missing.
pub fn ensure_pair_file() -> DsparkPair {
    let path = pair_path();
    if path.exists() {
        return load_pair_from(&path);
    }
    let pair = DsparkPair::default();
    save_pair(&path, &pair);
    pair
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_pair_is_cross_family() {
        let p = DsparkPair::default();
        assert_eq!(p.creator, "gpt-4o-mini");
        assert_eq!(p.curator, "deepseek-v4-pro");
        assert_ne!(p.creator, p.curator);
    }
}
