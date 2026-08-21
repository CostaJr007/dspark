//! End-to-end speculative arbitration pipeline:
//! research → generate → audit → refine → reaudit.

use crate::client::ModelClient;
use crate::curator::{AuditResult, CuratorError, DeepSeekCurator, RefineResult};
use crate::generator::GeminiGenerator;
use crate::search::WebSearchEngine;

pub struct PipelineResult {
    pub specification: String,
    pub research: Option<String>,
    pub draft_code: String,
    pub audit_result: AuditResult,
    pub reaudit_result: Option<AuditResult>,
    pub final_code: String,
    pub refined: bool,
    pub refine_result: Option<RefineResult>,
}

pub struct DSparkPipeline {
    curator: DeepSeekCurator,
    generator_spec: Option<String>,
    auto_refine_threshold: u32,
}

impl DSparkPipeline {
    pub fn new() -> Result<Self, CuratorError> {
        Ok(Self {
            curator: DeepSeekCurator::new()?,
            generator_spec: None,
            auto_refine_threshold: 85,
        })
    }

    pub fn with_models(generator: &str, curator: &str) -> Result<Self, CuratorError> {
        Ok(Self {
            curator: DeepSeekCurator::with_model(curator)?,
            generator_spec: Some(generator.to_string()),
            auto_refine_threshold: 85,
        })
    }

    pub async fn run(
        &self,
        specification: &str,
        draft_code: Option<&str>,
        language: Option<&str>,
        research: bool,
    ) -> Result<PipelineResult, CuratorError> {
        let mut spec_for_models = specification.to_string();
        let mut research_notes = None;

        if research {
            let engine = WebSearchEngine::new();
            let report = engine.research_topic(specification, 3).await;
            if !report.starts_with("No results found") {
                spec_for_models = format!(
                    "{specification}\n\n### LIVE WEB RESEARCH (do not invent APIs; prefer this over training memory):\n{report}"
                );
                research_notes = Some(report);
            }
        }

        let draft = if let Some(code) = draft_code {
            code.to_string()
        } else {
            let spec = self
                .generator_spec
                .clone()
                .unwrap_or_else(|| "gemini-2.5-flash".to_string());
            let generator = GeminiGenerator::with_client(
                ModelClient::from_spec(&spec).map_err(CuratorError::from)?,
            );
            generator
                .generate_draft(&spec_for_models, language, 0.7)
                .await
                .map_err(CuratorError::from)?
        };

        let audit = self
            .curator
            .audit(&draft, &spec_for_models, language)
            .await?;

        let mut final_code = draft.clone();
        let mut refined = false;
        let mut refine_result = None;

        if !audit.is_approved() || audit.score < self.auto_refine_threshold {
            if let Some(code) = &audit.refined_code {
                final_code = code.clone();
                refined = true;
            } else {
                let mut feedback: Vec<String> = audit.critical_issues.clone();
                feedback.extend(audit.suggested_improvements.clone());
                for ce in &audit.counter_examples {
                    feedback.push(format!(
                        "Counter-example `{}` expected `{}` got `{}`",
                        ce.failing_input, ce.expected_behavior, ce.actual_behavior
                    ));
                }
                let res = self
                    .curator
                    .refine(
                        &draft,
                        &spec_for_models,
                        Some(&feedback.join("\n")),
                        language,
                    )
                    .await?;
                final_code = res.refined_code.clone();
                refined = true;
                refine_result = Some(res);
            }
        }

        let reaudit_result = if refined {
            Some(
                self.curator
                    .audit(&final_code, &spec_for_models, language)
                    .await?,
            )
        } else {
            None
        };

        Ok(PipelineResult {
            specification: specification.to_string(),
            research: research_notes,
            draft_code: draft,
            audit_result: audit,
            reaudit_result,
            final_code,
            refined,
            refine_result,
        })
    }
}
