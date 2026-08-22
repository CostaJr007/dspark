"""
Backwards compatibility pipeline wrapper.
"""

from dataclasses import dataclass
import logging
from typing import Optional

from ..curator import AuditResult, DeepSeekCurator, RefineResult
from ..generator import GeminiGenerator

logger = logging.getLogger("dspark.pipeline.legacy")


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
        self.curator = curator or DeepSeekCurator()
        self.generator = generator
        self.auto_refine_threshold = auto_refine_threshold

    def run(
        self,
        specification: str,
        draft_code: Optional[str] = None,
        language: Optional[str] = None,
        max_refine_attempts: int = 1,
    ) -> PipelineResult:
        if not draft_code:
            if not self.generator:
                self.generator = GeminiGenerator()
            draft_code = self.generator.generate_draft(specification, language=language)

        audit = self.curator.audit(
            code=draft_code,
            specification=specification,
            language=language,
        )

        final_code = draft_code
        refined = False
        refine_res = None

        if not audit.is_approved or audit.score < self.auto_refine_threshold:
            if audit.refined_code:
                final_code = audit.refined_code
                refined = True
            else:
                refine_res = self.curator.refine(
                    code=draft_code,
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
