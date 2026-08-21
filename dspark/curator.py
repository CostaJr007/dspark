"""
DeepSeek Curator & Arbitrator Engine for DSpark.
"""

from dataclasses import dataclass, field
from enum import Enum
import json
import re
from typing import Any, Dict, List, Optional, Union

from .client import DeepSeekClient
from .prompts import CURATOR_SYSTEM_PROMPT, ARBITRATOR_SYSTEM_PROMPT, REFINER_SYSTEM_PROMPT


class CurationVerdict(str, Enum):
    APPROVED = "APPROVED"
    NEEDS_REVISION = "NEEDS_REVISION"
    REJECTED = "REJECTED"


@dataclass
class EdgeCase:
    case: str
    risk_level: str
    handled_properly: bool
    remedy: str = ""


@dataclass
class CounterExample:
    failing_input: str
    expected_behavior: str
    actual_behavior: str
    severity: str = "HIGH"


@dataclass
class AuditResult:
    verdict: CurationVerdict
    score: int
    summary: str
    criteria_scores: Dict[str, int] = field(default_factory=dict)
    counter_examples: List[CounterExample] = field(default_factory=list)
    io_contract_analysis: Dict[str, Any] = field(default_factory=dict)
    edge_cases: List[EdgeCase] = field(default_factory=list)
    complexity: Dict[str, Any] = field(default_factory=dict)
    critical_issues: List[str] = field(default_factory=list)
    suggested_improvements: List[str] = field(default_factory=list)
    refined_code: Optional[str] = None
    raw_response: str = ""

    @property
    def is_approved(self) -> bool:
        return self.verdict == CurationVerdict.APPROVED


@dataclass
class RefineResult:
    refined_code: str
    summary_of_changes: List[str] = field(default_factory=list)
    raw_response: str = ""


@dataclass
class ArbitrationResult:
    winner_index: int
    rationale: str
    comparison_matrix: Dict[str, Any] = field(default_factory=dict)
    synthesized_code: str = ""
    raw_response: str = ""


def _extract_json(text: str) -> Dict[str, Any]:
    """Helper to extract JSON object from markdown blocks or free text."""
    text = text.strip()
    
    # Strip <think>...</think> tags if present
    text = re.sub(r"<think>.*?</think>", "", text, flags=re.DOTALL).strip()

    # 1. Try direct parse with strict=False
    try:
        return json.loads(text, strict=False)
    except Exception:
        pass

    # 2. Try extracting inside ```json ... ```
    match = re.search(r"```(?:json)?\s*(\{.*?\})\s*```", text, re.DOTALL)
    if match:
        try:
            return json.loads(match.group(1), strict=False)
        except Exception:
            pass

    # 3. Try finding outermost { and }
    first_brace = text.find("{")
    last_brace = text.rfind("}")
    if first_brace != -1 and last_brace != -1 and last_brace > first_brace:
        candidate = text[first_brace:last_brace + 1]
        try:
            return json.loads(candidate, strict=False)
        except Exception:
            pass

    # 4. Fallback: repair unescaped newlines inside strings
    def _clean_newlines(m):
        return m.group(0).replace("\n", "\\n").replace("\r", "\\r")
    
    try:
        cleaned = re.sub(r'"([^"\\]*(\\.[^"\\]*)*)"', _clean_newlines, text)
        first_b = cleaned.find("{")
        last_b = cleaned.rfind("}")
        if first_b != -1 and last_b != -1:
            return json.loads(cleaned[first_b:last_b + 1], strict=False)
    except Exception:
        pass

    raise ValueError(f"Failed to parse valid JSON from model response:\n{text[:400]}...")


def _extract_code_blocks(text: str) -> str:
    """Extract code from triple backtick blocks if present."""
    match = re.search(r"```(?:\w+)?\n(.*?)```", text, re.DOTALL)
    if match:
        return match.group(1).strip()
    return text.strip()


class DeepSeekCurator:
    """
    Curator and Arbitrator engine powered by DeepSeek Reasoning, OpenAI, or Local LLMs.
    """

    def __init__(self, client: Optional[Any] = None, model: Optional[str] = None):
        if client:
            self.client = client
            self.model = model or getattr(client, "default_model", None)
        elif model:
            from .client import create_model_client
            self.client = create_model_client(model)
            self.model = getattr(self.client, "default_model", model)
        else:
            self.client = DeepSeekClient()
            self.model = self.client.default_model

    def audit(
        self,
        code: str,
        specification: str,
        language: Optional[str] = None,
        temperature: float = 0.1,
    ) -> AuditResult:
        """
        Audit draft code against a specification, validating I/O contracts and edge cases.
        """
        lang_str = f"Language: {language}\n" if language else ""
        user_prompt = (
            f"### SPECIFICATION / REQUIREMENTS:\n{specification}\n\n"
            f"### CANDIDATE IMPLEMENTATION TO AUDIT:\n{lang_str}```{language or ''}\n{code}\n```\n\n"
            f"Perform strict reasoning audit, I/O verification, edge case simulation, and return the required JSON."
        )

        raw_resp = self.client.complete(
            prompt=user_prompt,
            system_prompt=CURATOR_SYSTEM_PROMPT,
            model=self.model,
            temperature=temperature,
            response_format={"type": "json_object"},
        )

        data = _extract_json(raw_resp)
        verdict_str = str(data.get("verdict", "NEEDS_REVISION")).upper()
        verdict = CurationVerdict(verdict_str) if verdict_str in CurationVerdict.__members__ else CurationVerdict.NEEDS_REVISION

        edge_cases = []
        for ec in data.get("edge_cases_identified", []):
            if isinstance(ec, dict):
                edge_cases.append(
                    EdgeCase(
                        case=ec.get("case", ""),
                        risk_level=ec.get("risk_level", "MEDIUM"),
                        handled_properly=bool(ec.get("handled_properly", False)),
                        remedy=ec.get("remedy", ""),
                    )
                )

        counter_examples = []
        for ce in data.get("counter_examples", []):
            if isinstance(ce, dict):
                counter_examples.append(
                    CounterExample(
                        failing_input=str(ce.get("failing_input", "")),
                        expected_behavior=str(ce.get("expected_behavior", "")),
                        actual_behavior=str(ce.get("actual_behavior", "")),
                        severity=str(ce.get("severity", "HIGH")),
                    )
                )

        refined = data.get("refined_code", "").strip() or None

        return AuditResult(
            verdict=verdict,
            score=int(data.get("score", 70)),
            summary=data.get("summary", ""),
            criteria_scores=data.get("criteria_scores", {}),
            counter_examples=counter_examples,
            io_contract_analysis=data.get("io_contract_analysis", {}),
            edge_cases=edge_cases,
            complexity=data.get("complexity", {}),
            critical_issues=data.get("critical_issues", []),
            suggested_improvements=data.get("suggested_improvements", []),
            refined_code=refined,
            raw_response=raw_resp,
        )

    def refine(
        self,
        code: str,
        specification: str,
        feedback: Optional[str] = None,
        language: Optional[str] = None,
        temperature: float = 0.2,
    ) -> RefineResult:
        """
        Produce a refined, production-grade version of the code.
        """
        feedback_section = f"### AUDIT FEEDBACK / ISSUES TO FIX:\n{feedback}\n\n" if feedback else ""
        lang_str = f"Language: {language}\n" if language else ""
        user_prompt = (
            f"### SPECIFICATION:\n{specification}\n\n"
            f"{feedback_section}"
            f"### DRAFT CODE:\n{lang_str}```{language or ''}\n{code}\n```\n\n"
            f"Refine the code to 100% production readiness. Fix all potential edge cases and enforce strict I/O typing."
        )

        raw_resp = self.client.complete(
            prompt=user_prompt,
            system_prompt=REFINER_SYSTEM_PROMPT,
            model=self.model,
            temperature=temperature,
        )

        refined_code = _extract_code_blocks(raw_resp)
        
        # Extract bullets if present
        changes = []
        for line in raw_resp.splitlines():
            line = line.strip()
            if line.startswith(("- ", "* ", "• ")) and not line.startswith("```"):
                changes.append(line[2:].strip())

        return RefineResult(
            refined_code=refined_code,
            summary_of_changes=changes,
            raw_response=raw_resp,
        )

    def arbitrate(
        self,
        candidates: List[str],
        specification: str,
        language: Optional[str] = None,
        temperature: float = 0.1,
    ) -> ArbitrationResult:
        """
        Arbitrate between two or more candidate code implementations.
        """
        if len(candidates) < 2:
            raise ValueError("Arbitration requires at least 2 candidate implementations.")

        candidates_formatted = []
        for idx, cand in enumerate(candidates):
            candidates_formatted.append(
                f"### CANDIDATE #{idx}:\n```{language or ''}\n{cand}\n```"
            )

        user_prompt = (
            f"### SPECIFICATION:\n{specification}\n\n"
            + "\n\n".join(candidates_formatted)
            + "\n\nCompare the candidates thoroughly, choose the winner and synthesize the optimal code in the specified JSON format."
        )

        raw_resp = self.client.complete(
            prompt=user_prompt,
            system_prompt=ARBITRATOR_SYSTEM_PROMPT,
            model=self.model,
            temperature=temperature,
            response_format={"type": "json_object"},
        )

        data = _extract_json(raw_resp)
        return ArbitrationResult(
            winner_index=int(data.get("winner_index", 0)),
            rationale=data.get("rationale", ""),
            comparison_matrix=data.get("comparison_matrix", {}),
            synthesized_code=data.get("synthesized_code", ""),
            raw_response=raw_resp,
        )
