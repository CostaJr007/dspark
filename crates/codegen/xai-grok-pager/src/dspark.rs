//! Product defaults: creator + curator is the normal dspark-cli workflow.

use xai_grok_tools::dspark_pair::{ensure_pair_file, DsparkPair};

/// Write `~/.dspark/pair.toml` if missing and persist it as the live creator
/// and curator. Dual-engine is the default dspark-cli workflow.
pub async fn apply_product_defaults() {
    let pair = ensure_pair_file();
    let _ = xai_grok_shell::util::config::set_default_model(pair.creator.clone()).await;
    let _ = xai_grok_shell::util::config::set_fork_secondary_model(pair.curator.clone()).await;
}

pub fn current_pair() -> DsparkPair {
    ensure_pair_file()
}
