"""
Unit tests for DSpark package components.
"""

import unittest
from dspark.curator import _extract_json, _extract_code_blocks, CurationVerdict, EdgeCase, AuditResult
from dspark.prompts import CURATOR_SYSTEM_PROMPT, ARBITRATOR_SYSTEM_PROMPT


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


if __name__ == "__main__":
    unittest.main()
