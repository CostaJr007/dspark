//! `/pair` — show or set the DSpark creator/curator model pair.

use crate::app::actions::Action;
use crate::slash::command::{CommandExecCtx, CommandResult, SlashCommand};
use xai_grok_tools::dspark_pair::{
    load_pair_from, pair_path, save_pair, DsparkPair, DEFAULT_CREATOR, DEFAULT_CURATOR,
};

/// Dual-engine pair: creator drafts, curator of another family audits I/O.
pub struct PairCommand;

impl SlashCommand for PairCommand {
    fn name(&self) -> &str {
        "pair"
    }

    fn aliases(&self) -> &[&str] {
        &["dspark"]
    }

    fn description(&self) -> &str {
        "Show or set the default creator/curator pair"
    }

    fn usage(&self) -> &str {
        "/pair [creator] [curator]"
    }

    fn takes_args(&self) -> bool {
        true
    }

    fn session_scoped(&self) -> bool {
        true
    }

    fn run(&self, ctx: &mut CommandExecCtx, args: &str) -> CommandResult {
        let path = pair_path();
        let mut pair = load_pair_from(&path);
        if pair.creator.is_empty() {
            pair.creator = DEFAULT_CREATOR.to_string();
        }
        if pair.curator.is_empty() {
            pair.curator = DEFAULT_CURATOR.to_string();
        }
        let parts: Vec<&str> = args.split_whitespace().collect();
        match parts.as_slice() {
            [] => {
                save_pair(&path, &pair);
                match apply_pair(ctx, &pair, &path) {
                    CommandResult::Error(_) => CommandResult::Message(format_pair(&pair, &path)),
                    other => other,
                }
            }
            [creator] => {
                pair.creator = (*creator).to_string();
                save_pair(&path, &pair);
                apply_pair(ctx, &pair, &path)
            }
            [creator, curator, ..] => {
                pair.creator = (*creator).to_string();
                pair.curator = (*curator).to_string();
                save_pair(&path, &pair);
                apply_pair(ctx, &pair, &path)
            }
        }
    }
}

fn apply_pair(ctx: &CommandExecCtx, pair: &DsparkPair, path: &std::path::Path) -> CommandResult {
    let mut actions = Vec::new();
    match ctx.models.resolve_by_name_or_id(&pair.creator) {
        Some(id) => actions.push(Action::SetDefaultModel(id)),
        None => {
            return CommandResult::Error(format!(
                "Unknown creator model: {}. Pair saved to {} but not applied.",
                pair.creator,
                path.display()
            ));
        }
    }
    match ctx.models.resolve_by_name_or_id(&pair.curator) {
        Some(id) => actions.push(Action::SetForkSecondaryModel(id)),
        None => {
            return CommandResult::Error(format!(
                "Unknown curator model: {}. Pair saved to {} but curator was not applied.",
                pair.curator,
                path.display()
            ));
        }
    }
    CommandResult::Actions(actions)
}

fn format_pair(pair: &DsparkPair, path: &std::path::Path) -> String {
    let warning = if pair.creator.eq_ignore_ascii_case(&pair.curator) {
        "\nWarning: creator and curator are the same model (confirmation bias)."
    } else {
        ""
    };
    format!(
        "DSpark default pair\n  creator = {}\n  curator = {}\n  file    = {}{warning}\nCuration runs automatically after code edits. /pair only chooses the two models.",
        pair.creator,
        pair.curator,
        path.display()
    )
}
