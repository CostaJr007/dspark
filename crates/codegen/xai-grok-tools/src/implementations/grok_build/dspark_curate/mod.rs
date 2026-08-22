//! Dual-engine curator: audit (+ refine) with a different-family model.

use std::path::PathBuf;

use crate::types::output::{TextOutput, ToolOutput};
use crate::types::requirements::{Expr, ToolRequirement};
use crate::types::resources::Cwd;
use crate::types::tool::{ToolKind, ToolNamespace};
use crate::types::tool_metadata::shared_resources;

pub const DSPARK_CURATE_TOOL_NAME: &str = "dspark_curate";

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct DsparkCurateInput {
    #[schemars(description = "File to audit (workspace-relative or absolute). Provide this or `code`.")]
    pub path: Option<String>,
    #[schemars(description = "Inline source if no path is given.")]
    pub code: Option<String>,
    #[schemars(description = "Specification / I/O contract the code must satisfy.")]
    pub spec: Option<String>,
    #[schemars(description = "Language hint, e.g. python, rust.")]
    pub language: Option<String>,
    #[schemars(description = "Curator model override. Default: ~/.dspark/pair.toml curator.")]
    pub curator: Option<String>,
}

#[derive(Debug, Default)]
pub struct DsparkCurateTool;

impl crate::types::tool_metadata::ToolMetadata for DsparkCurateTool {
    fn kind(&self) -> ToolKind {
        ToolKind::Other
    }

    fn tool_namespace(&self) -> ToolNamespace {
        ToolNamespace::GrokBuild
    }

    fn description_template(&self) -> &str {
        r#"Independent curator audit (LLM-as-a-Verifier). A different-family model scores specification, I/O contracts, and errors. Do not trust the creator's self-assessment.

Call after writing or refactoring code, before claiming the work is done. If the verdict is not APPROVED, apply `refined_code` (or fix the listed issues) and call again.

Uses the curator from `/pair` (`~/.dspark/pair.toml`) unless `curator` is set."#
    }

    fn requires_expr(&self) -> Expr<ToolRequirement> {
        Expr::True
    }
}

impl xai_tool_runtime::Tool for DsparkCurateTool {
    type Args = DsparkCurateInput;
    type Output = ToolOutput;

    fn id(&self) -> xai_tool_protocol::ToolId {
        xai_tool_protocol::ToolId::new(DSPARK_CURATE_TOOL_NAME).expect("valid tool id")
    }

    fn description(
        &self,
        _ctx: &::xai_tool_runtime::ListToolsContext,
    ) -> xai_tool_types::ToolDescription {
        xai_tool_types::ToolDescription::new(
            DSPARK_CURATE_TOOL_NAME,
            crate::types::tool_metadata::ToolMetadata::sanitized_description_template(self),
        )
    }

    fn capabilities(&self) -> xai_tool_protocol::ToolCapabilities {
        xai_tool_protocol::ToolCapabilities {
            is_read_only: true,
            tool_scope: Some(xai_tool_protocol::ToolScope::Read),
            ..Default::default()
        }
    }

    #[tracing::instrument(name = "new_tool.dspark_curate", skip_all)]
    async fn run(
        &self,
        ctx: xai_tool_runtime::ToolCallContext,
        input: DsparkCurateInput,
    ) -> Result<ToolOutput, xai_tool_runtime::ToolError> {
        let tool_id = xai_tool_protocol::ToolId::new(DSPARK_CURATE_TOOL_NAME).expect("valid");
        let resources = shared_resources(&ctx)?;
        let cwd = {
            let res = resources.lock().await;
            res.get::<Cwd>()
                .map(|c| c.0.clone())
                .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
        };

        let code = if let Some(raw) = input.code.as_ref().filter(|s| !s.trim().is_empty()) {
            raw.clone()
        } else if let Some(path) = input.path.as_ref() {
            let p = PathBuf::from(path);
            let abs = if p.is_absolute() { p } else { cwd.join(p) };
            std::fs::read_to_string(&abs).map_err(|e| {
                xai_tool_runtime::ToolError::execution(
                    tool_id.clone(),
                    format!("failed to read {}: {e}", abs.display()),
                )
            })?
        } else {
            return Err(xai_tool_runtime::ToolError::execution(
                tool_id,
                "provide `path` or `code`",
            ));
        };

        let spec = input
            .spec
            .as_deref()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or("Audit specification, I/O contracts, edge cases, and errors. Do not trust the author's self-assessment.");
        let curator_id = input
            .curator
            .clone()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| crate::dspark_pair::load_pair().curator);
        let lang = input.language.as_deref();

        let curator = dspark::DeepSeekCurator::with_model(&curator_id).map_err(|e| {
            xai_tool_runtime::ToolError::execution(tool_id.clone(), e.to_string())
        })?;

        let audit = curator.audit(&code, spec, lang).await.map_err(|e| {
            xai_tool_runtime::ToolError::execution(tool_id.clone(), e.to_string())
        })?;

        let mut refined = None;
        if audit.must_revise() {
            let mut feedback = audit.critical_issues.clone();
            for ce in &audit.counter_examples {
                feedback.push(format!(
                    "Counter-example `{}` expected `{}` got `{}`",
                    ce.failing_input, ce.expected_behavior, ce.actual_behavior
                ));
            }
            match curator
                .refine(&code, spec, Some(&feedback.join("\n")), lang)
                .await
            {
                Ok(r) if !r.refined_code.trim().is_empty() => refined = Some(r.refined_code),
                Ok(_) => {}
                Err(e) => {
                    tracing::warn!(error = %e, "dspark_curate refine failed");
                }
            }
        }

        let body = serde_json::json!({
            "curator": curator_id,
            "verdict": audit.verdict.to_string(),
            "score": audit.score,
            "must_revise": audit.must_revise(),
            "summary": audit.summary,
            "critical_issues": audit.critical_issues,
            "counter_examples": audit.counter_examples,
            "refined_code": refined,
        });
        Ok(ToolOutput::Text(TextOutput::from(
            serde_json::to_string_pretty(&body).unwrap_or_else(|_| body.to_string()),
        )))
    }
}


