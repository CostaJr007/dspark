"""
Unit tests for DSpark package components.
"""

import unittest
from dspark.curator import (
    _extract_json,
    _extract_code_blocks,
    CurationVerdict,
    EdgeCase,
    AuditResult,
    RefineResult,
)
from dspark.pipeline import DSparkPipeline
from dspark.prompts import CURATOR_SYSTEM_PROMPT, ARBITRATOR_SYSTEM_PROMPT


class SequencedCurator:
    def __init__(self):
        self.audited_code = []
        self.refine_calls = 0

    def audit(self, code, specification, language=None):
        self.audited_code.append(code)
        if len(self.audited_code) == 1:
            return AuditResult(
                verdict=CurationVerdict.NEEDS_REVISION,
                score=40,
                summary="Needs refinement",
            )
        return AuditResult(
            verdict=CurationVerdict.APPROVED,
            score=95,
            summary="Approved",
        )

    def refine(self, code, specification, feedback=None, language=None):
        self.refine_calls += 1
        return RefineResult(refined_code="fixed")


class TestDSparkCore(unittest.TestCase):

    def test_json_extraction_clean(self):
        raw = '{"verdict": "APPROVED", "score": 95, "summary": "Great code."}'
        parsed = _extract_json(raw)
        self.assertEqual(parsed["verdict"], "APPROVED")
        self.assertEqual(parsed["score"], 95)

    def test_json_extraction_markdown_block(self):
        raw = 'Here is the analysis:\n```json\n{"verdict": "NEEDS_REVISION", "score": 60, "summary": "Has edge case."}\n```\nHope it helps!'
        parsed = _extract_json(raw)
        self.assertEqual(parsed["verdict"], "NEEDS_REVISION")
        self.assertEqual(parsed["score"], 60)

    def test_code_block_extraction(self):
        raw = "```python\ndef hello():\n    return 'world'\n```"
        code = _extract_code_blocks(raw)
        self.assertEqual(code, "def hello():\n    return 'world'")

    def test_audit_result_dataclass(self):
        res = AuditResult(
            verdict=CurationVerdict.APPROVED,
            score=95,
            summary="All tests passed",
            edge_cases=[EdgeCase(case="Empty array", risk_level="LOW", handled_properly=True)],
        )
        self.assertTrue(res.is_approved)
        self.assertEqual(len(res.edge_cases), 1)

    def test_pipeline_honors_refine_attempt_limit(self):
        curator = SequencedCurator()
        result = DSparkPipeline(curator=curator).run(
            specification="Fix the implementation",
            draft_code="broken",
            max_refine_attempts=1,
        )

        self.assertEqual(curator.refine_calls, 1)
        self.assertEqual(curator.audited_code, ["broken", "fixed"])
        self.assertEqual(result.final_code, "fixed")
        self.assertTrue(result.audit_result.is_approved)


if __name__ == "__main__":
    unittest.main()
