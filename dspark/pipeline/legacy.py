"""
Backwards compatibility pipeline wrapper.
"""

from dataclasses import dataclass
import logging
from typing import Optional

from ..curator import AuditResult, DeepSeekCurator, RefineResult
from ..config import config
from ..generator import GeminiGenerator

logger = logging.getLogger("dspark.pipeline.legacy")


def _model_without_provider(model: str, provider: str) -> str:
    prefix = f"{provider}/"
    return model[len(prefix):] if model.startswith(prefix) else model


@dataclass
class PipelineResult:
    specification: str
    draft_code: str
    audit_result: AuditResult
    final_code: str
    refined: bool
    refine_result: Optional[RefineResult] = None


class DSparkPipeline:
    def __init__(
        self,
        curator: Optional[DeepSeekCurator] = None,
        generator: Optional[GeminiGenerator] = None,
        auto_refine_threshold: int = 85,
    ):
        self.curator = curator or DeepSeekCurator(model=config.curator_model)
        self.generator = generator
        self.auto_refine_threshold = auto_refine_threshold

    def run(
        self,
        specification: str,
        draft_code: Optional[str] = None,
        language: Optional[str] = None,
        max_refine_attempts: int = 1,
    ) -> PipelineResult:
        if max_refine_attempts < 0:
            raise ValueError("max_refine_attempts must be non-negative")

        if not draft_code:
            if not self.generator:
                self.generator = GeminiGenerator(
                    model=_model_without_provider(config.creator_model, "gemini")
                )
            draft_code = self.generator.generate_draft(specification, language=language)

        final_code = draft_code
        refined = False
        refine_res = None
        audit = None

        for attempt in range(max_refine_attempts + 1):
            audit = self.curator.audit(
                code=final_code,
                specification=specification,
                language=language,
            )
            if audit.is_approved and audit.score >= self.auto_refine_threshold:
                break
            if attempt == max_refine_attempts:
                break

            if audit.refined_code:
                final_code = audit.refined_code
            else:
                refine_res = self.curator.refine(
                    code=final_code,
                    specification=specification,
                    feedback="\n".join(audit.critical_issues + audit.suggested_improvements),
                    language=language,
                )
                final_code = refine_res.refined_code
            refined = True

        return PipelineResult(
            specification=specification,
            draft_code=draft_code,
            audit_result=audit,
            final_code=final_code,
            refined=refined,
            refine_result=refine_res,
        )
