//! After code edits, run the independent curator automatically.
//! The user does not have to ask for curation; `/pair` only chooses models.

use std::path::{Path, PathBuf};

use crate::types::output::{ApplyPatchOutput, SearchReplaceOutput, ToolOutput};
use crate::types::requirements::{Expr, ToolRequirement};
use crate::types::resources::SharedResources;
use crate::types::tool::{Reminder, ToolKind};

const MAX_SOURCE_BYTES: u64 = 80_000;
const MAX_FILES_PER_EDIT: usize = 3;

pub struct DsparkCurateReminder;

impl DsparkCurateReminder {
    fn edited_paths(tool_output: &ToolOutput) -> Vec<PathBuf> {
        match tool_output {
            ToolOutput::SearchReplace(SearchReplaceOutput::EditsApplied(r)) => {
                vec![r.absolute_path.clone()]
            }
            ToolOutput::ApplyPatch(ApplyPatchOutput::Success { files, .. }) => {
                files.iter().map(|f| f.path.clone()).collect()
            }
            _ => Vec::new(),
        }
    }

    fn language_hint(path: &Path) -> Option<&'static str> {
        match path
            .extension()
            .and_then(|e| e.to_str())
            .map(|s| s.to_ascii_lowercase())
            .as_deref()
        {
            Some("py") => Some("python"),
            Some("rs") => Some("rust"),
            Some("ts" | "tsx") => Some("typescript"),
            Some("js" | "jsx" | "mjs" | "cjs") => Some("javascript"),
            Some("go") => Some("go"),
            Some("java") => Some("java"),
            Some("kt" | "kts") => Some("kotlin"),
            Some("swift") => Some("swift"),
            Some("c") => Some("c"),
            Some("cc" | "cpp" | "cxx" | "h" | "hpp") => Some("cpp"),
            Some("cs") => Some("csharp"),
            Some("rb") => Some("ruby"),
            Some("php") => Some("php"),
            Some("scala") => Some("scala"),
            Some("sh" | "bash" | "zsh") => Some("bash"),
            Some("sql") => Some("sql"),
            _ => None,
        }
    }

    fn display_name(path: &Path) -> String {
        path.file_name()
            .and_then(|n| n.to_str())
            .map(str::to_string)
            .unwrap_or_else(|| path.display().to_string())
    }

    async fn curate_file(path: &Path) -> String {
        let name = Self::display_name(path);
        let Some(lang) = Self::language_hint(path) else {
            return String::new();
        };
        let meta = match std::fs::metadata(path) {
            Ok(m) => m,
            Err(e) => {
                return format!("DSpark curator skipped {name}: could not stat file ({e}).");
            }
        };
        if meta.len() > MAX_SOURCE_BYTES {
            return format!(
                "DSpark curator skipped {name}: file larger than {MAX_SOURCE_BYTES} bytes."
            );
        }
        let code = match std::fs::read_to_string(path) {
            Ok(c) if !c.trim().is_empty() => c,
            Ok(_) => return format!("DSpark curator skipped {name}: empty file."),
            Err(e) => return format!("DSpark curator skipped {name}: read failed ({e})."),
        };

        let pair = crate::dspark_pair::load_pair();
        let curator = match dspark::DeepSeekCurator::with_model(&pair.curator) {
            Ok(c) => c,
            Err(e) => {
                return format!(
                    "DSpark curator unavailable ({e}). Dual-engine is still the default; set the curator key or `/pair`."
                );
            }
        };

        let spec = format!(
            "File `{name}` was just written by the creator. Audit the implementation against its signatures, docstrings, and I/O contracts. Do not trust the author's self-assessment. Flag missing edge cases and incorrect examples."
        );

        let audit = match curator.audit(&code, &spec, Some(lang)).await {
            Ok(a) => a,
            Err(e) => return format!("DSpark curator failed on {name}: {e}"),
        };

        let mut applied = false;
        if audit.must_revise() {
            let mut feedback = audit.critical_issues.clone();
            for ce in &audit.counter_examples {
                feedback.push(format!(
                    "Counter-example `{}` expected `{}` got `{}`",
                    ce.failing_input, ce.expected_behavior, ce.actual_behavior
                ));
            }
            if let Ok(r) = curator
                .refine(&code, &spec, Some(&feedback.join("\n")), Some(lang))
                .await
            {
                if r.refined_code.trim().len() > 20 {
                    if std::fs::write(path, &r.refined_code).is_ok() {
                        applied = true;
                    }
                }
            }
        }

        let issues = if audit.critical_issues.is_empty() {
            String::new()
        } else {
            format!(" Issues: {}.", audit.critical_issues.join("; "))
        };

        if applied {
            format!(
                "DSpark curator ({}) ran automatically on {name}: {} {}/100. Refined code was applied — do not ask the user for permission to curate. Re-read the file before further edits.{issues}",
                pair.curator,
                audit.verdict,
                audit.score
            )
        } else {
            format!(
                "DSpark curator ({}) ran automatically on {name}: {} {}/100. Curation is the default after code edits; the user does not request it.{issues}",
                pair.curator,
                audit.verdict,
                audit.score
            )
        }
    }
}

#[async_trait::async_trait]
impl Reminder for DsparkCurateReminder {
    fn requires_expr(&self) -> Expr<ToolRequirement> {
        Expr::Or(vec![
            Expr::Value(ToolRequirement::tool_kind(ToolKind::Edit)),
            Expr::Value(ToolRequirement::tool_kind(ToolKind::Write)),
        ])
    }

    async fn collect_reminders(
        &self,
        _resources: SharedResources,
        tool_output: &ToolOutput,
    ) -> Vec<String> {
        let mut paths: Vec<PathBuf> = Self::edited_paths(tool_output)
            .into_iter()
            .filter(|p| Self::language_hint(p).is_some())
            .collect();
        paths.truncate(MAX_FILES_PER_EDIT);
        if paths.is_empty() {
            return vec![];
        }
        let mut out = Vec::new();
        for path in paths {
            let msg = Self::curate_file(&path).await;
            if !msg.is_empty() {
                out.push(msg);
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::output::SearchReplaceEditsApplied;

    #[test]
    fn extracts_search_replace_path() {
        let out = ToolOutput::SearchReplace(SearchReplaceOutput::EditsApplied(
            SearchReplaceEditsApplied {
                old_string: "a".into(),
                new_string: "b".into(),
                tool_output_for_prompt: String::new(),
                tool_output_for_prompt_concise: None,
                absolute_path: PathBuf::from("/repo/src/lib.rs"),
                edits: Default::default(),
                patch: None,
                unicode_normalized: false,
            },
        ));
        let paths = DsparkCurateReminder::edited_paths(&out);
        assert_eq!(paths, vec![PathBuf::from("/repo/src/lib.rs")]);
        assert_eq!(
            DsparkCurateReminder::language_hint(Path::new("lib.rs")),
            Some("rust")
        );
        assert_eq!(DsparkCurateReminder::language_hint(Path::new("README.md")), None);
    }
}
