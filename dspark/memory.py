"""
AgentDeltaMemory: agent-level adaptation of the Kimi Delta Attention (KDA) memory dynamics.

Theory (Kimi Team, 2025 - "Kimi Linear: An Expressive, Efficient Attention Architecture", arXiv:2510.26692).

KDA maintains a fixed-size matrix-valued state S_t updated by a fine-grained gated delta rule::

    S_t = (I - beta_t * k_t * k_t^T) * Diag(alpha_t) * S_{t-1} + beta_t * k_t * v_t^T

Three properties are ported from the token level to agent orchestration:

1. **Delta rule (memory that corrects itself).**  Before writing, the memory looks up its
   current prediction for key k_t and writes only the correction -- a single online gradient
   descent step on the reconstruction loss ``L(S) = 1/2 * ||S^T k_t - v_t||^2``.  In agents:
   knowledge is *corrected*, never blindly appended.  When the correction norm falls below
   eps, the memory has converged: it "already knew" the outcome, which yields a principled
   stop criterion for refinement loops (delta -> 0 theorem).

2. **Per-channel forgetting (Diag(alpha_t)).**  KDA replaces the scalar forget gate of Gated
   DeltaNet with a channel-wise decay so that each feature dimension retains its own
   forgetting rate.  In agents: each memory channel decays at its own rate, so hard
   invariants (alpha ~ 1, never forgotten) coexist with transient facts (alpha << 1, fade).

3. **Key-bound (DPLR) updates.**  KDA binds the DPLR low-rank factors to the key itself
   (a = beta*k, b = k * alpha), making every update rank-1 *along the current key* --
   surgical, with no collateral damage to unrelated memory.  In agents: a refinement guided
   by a counterexample only touches knowledge bound to that counterexample signature.

Evidence strength (beta) maps to trust: sandbox-approved verdicts (1.0) > audit verdicts
(0.7) > draft-level observations (0.6).  The state stays bounded (fixed capacity per
channel), mirroring KDA's fixed-size RNN state that replaces the growing KV cache.

Usage::

    mem = AgentDeltaMemory()
    mem.write("contract:python:abs_val", "result >= 0", channel="invariant", beta=0.95)
    read = mem.lookup("contract:python:abs_val")          # predicted value, outcome
    mem.decay(1)                                          # one agent time-step
"""

from __future__ import annotations

import hashlib
import math
import re
from dataclasses import dataclass, field
from typing import Any, Dict, List, Optional, Tuple

_TOKEN_RE = re.compile(r"[a-z0-9_]+")


def embed(text: str, dim: int = 64) -> List[float]:
    """Deterministic hashing-trick embedding of a text into a unit vector."""
    vec = [0.0] * dim
    for token in _TOKEN_RE.findall(text.lower()):
        digest = hashlib.sha256(token.encode("utf-8")).digest()
        h = int.from_bytes(digest[:8], "big")
        vec[h % dim] += 1.0 if (h >> 63) & 1 else -1.0
    norm = math.sqrt(sum(x * x for x in vec))
    if norm > 0.0:
        vec = [x / norm for x in vec]
    return vec


def _norm(v: List[float]) -> float:
    return math.sqrt(sum(x * x for x in v))


def _dot(a: List[float], b: List[float]) -> float:
    return sum(x * y for x, y in zip(a, b))


def _cosine(a: List[float], b: List[float]) -> float:
    na, nb = _norm(a), _norm(b)
    if na == 0.0 or nb == 0.0:
        return 0.0
    return _dot(a, b) / (na * nb)


@dataclass
class MemoryEntry:
    """A single association written into a channel (agent-level k_t v_t^T)."""

    key: List[float]
    value: List[float]
    strength: float
    label: Optional[str] = None
    hits: int = 0


@dataclass
class MemoryWrite:
    """Result of a delta-rule write."""

    delta_norm: float
    converged: bool
    updated: bool
    entry_count: int
    channel: str


@dataclass
class MemoryRead:
    """Result of a key query (agent-level S_t^T q_t).

    ``outcome`` is the structured label vote of the top-k nearest entries
    (the discrete readout head of the memory), ``value`` the strength-weighted
    mean embedding of the recalled associations.
    """

    value: Optional[List[float]]
    outcome: Optional[str]
    confidence: float
    hits: int
    entries_hit: int


#: Default retention classes.  alpha ~ 1 retains, alpha << 1 forgets.
DEFAULT_CHANNELS: Dict[str, Dict[str, float]] = {
    "invariant": {"alpha": 0.999, "beta_max": 1.0, "capacity": 128.0, "key_similarity": 0.45},
    "decision": {"alpha": 0.97, "beta_max": 1.0, "capacity": 256.0, "key_similarity": 0.45},
    "transient": {"alpha": 0.80, "beta_max": 0.6, "capacity": 512.0, "key_similarity": 0.45},
}


class MemoryChannel:
    """Per-channel retention class with its own decay rate (Diag(alpha_t))."""

    def __init__(
        self,
        name: str,
        alpha: float = 0.97,
        beta_max: float = 1.0,
        capacity: int = 256,
        key_similarity: float = 0.45,
        dim: int = 64,
        min_strength: float = 0.01,
    ):
        self.name = name
        self.alpha = alpha
        self.beta_max = beta_max
        self.capacity = capacity
        self.key_similarity = key_similarity
        self.dim = dim
        self.min_strength = min_strength
        self.entries: List[MemoryEntry] = []

    def nearest(self, k: List[float]) -> Tuple[Optional[MemoryEntry], float]:
        best, best_sim = None, -1.0
        for entry in self.entries:
            sim = _cosine(k, entry.key)
            if sim > best_sim:
                best, best_sim = entry, sim
        return best, best_sim

    def add(self, entry: MemoryEntry) -> None:
        self.entries.append(entry)
        self.entries.sort(key=lambda e: e.strength, reverse=True)
        del self.entries[self.capacity:]

    def apply_decay(self, steps: int = 1) -> None:
        decay = self.alpha ** steps
        survivors = []
        for entry in self.entries:
            entry.strength *= decay
            if entry.strength >= self.min_strength:
                survivors.append(entry)
        self.entries = survivors
        self.entries.sort(key=lambda e: e.strength, reverse=True)
        del self.entries[self.capacity:]

    @property
    def total_strength(self) -> float:
        return sum(e.strength for e in self.entries)

    @property
    def total_hits(self) -> int:
        return sum(e.hits for e in self.entries)


class AgentDeltaMemory:
    """Fixed-size, delta-rule agent memory with per-channel forgetting.

    Faithful to KDA's update::

        S_t = (I - beta * k k^T) Diag(alpha) S_{t-1} + beta * k v^T

    Implemented discretely: each write resolves to the *nearest* entry by key cosine
    (the rank-1 "key-bound" direction) and updates only the correction term
    ``beta * (v - pred)``; a zero correction means the memory has converged for that key.
    Structured ``label`` (e.g. an APPROVED/REJECTED verdict) is stored per entry and
    recalled by weighted vote -- the discrete readout head of the memory.
    """

    def __init__(
        self,
        dim: int = 64,
        eps: float = 1e-3,
        top_k: int = 3,
        outcome_threshold: float = 0.5,
        channels: Optional[Dict[str, Dict[str, Any]]] = None,
    ):
        self.dim = dim
        self.eps = eps
        self.top_k = top_k
        self.outcome_threshold = outcome_threshold
        self.writes = 0
        self.converged_writes = 0
        self.channels: Dict[str, MemoryChannel] = {}
        for name, cfg in (channels or DEFAULT_CHANNELS).items():
            self.channels[name] = MemoryChannel(
                name=name,
                alpha=float(cfg.get("alpha", 0.97)),
                beta_max=float(cfg.get("beta_max", 1.0)),
                capacity=int(cfg.get("capacity", 256)),
                key_similarity=float(cfg.get("key_similarity", 0.45)),
                dim=dim,
            )

    def _channel(self, name: str) -> MemoryChannel:
        try:
            return self.channels[name]
        except KeyError:
            raise KeyError(f"unknown memory channel: {name!r}") from None

    @staticmethod
    def _merge_label(current: Optional[str], incoming: Optional[str]) -> Optional[str]:
        """Majority label semantics: conflicting labels void the entry's vote."""
        if current is None or incoming is None:
            return incoming if incoming is not None else current
        return current if current == incoming else None

    def write(
        self,
        key: str,
        value: str,
        channel: str = "decision",
        beta: Optional[float] = None,
        label: Optional[str] = None,
    ) -> MemoryWrite:
        """Delta-rule write: correct the nearest key-bound entry, or insert.

        Returns ``converged=True`` when the correction norm is below eps -- the memory
        already predicted this value, so the surrounding pipeline stopped learning.
        """
        ch = self._channel(channel)
        k = embed(key, self.dim)
        v = embed(value, self.dim)
        beta = min(beta if beta is not None else ch.beta_max, ch.beta_max)

        nearest, sim = ch.nearest(k)
        if nearest is None or sim < ch.key_similarity:
            ch.add(MemoryEntry(key=k, value=v, strength=beta, label=label))
            self.writes += 1
            delta_norm = _norm(v)
            return MemoryWrite(
                delta_norm=delta_norm,
                converged=delta_norm < self.eps,
                updated=True,
                entry_count=len(ch.entries),
                channel=ch.name,
            )

        pred = nearest.value
        delta = [vi - pi for vi, pi in zip(v, pred)]
        delta_norm = _norm(delta)
        nearest.label = self._merge_label(nearest.label, label)
        if delta_norm < self.eps:
            self.writes += 1
            self.converged_writes += 1
            nearest.hits += 1
            return MemoryWrite(
                delta_norm=delta_norm,
                converged=True,
                updated=False,
                entry_count=len(ch.entries),
                channel=ch.name,
            )

        # Delta rule: write only the correction along the key direction (rank-1).
        nearest.value = [pi + beta * d for pi, d in zip(pred, delta)]
        nearest.strength = min(nearest.strength + beta, 1.0)
        nearest.hits += 1
        self.writes += 1
        return MemoryWrite(
            delta_norm=delta_norm,
            converged=False,
            updated=True,
            entry_count=len(ch.entries),
            channel=ch.name,
        )

    def lookup(
        self,
        key: str,
        channel: Optional[str] = None,
    ) -> MemoryRead:
        """Query the memory (agent-level read ``S^T q``).

        The read is a strength-weighted mean of the top-k nearest entries; ``outcome``
        is the weighted label vote of those entries, reported when its weight fraction
        clears ``outcome_threshold``.
        """
        k = embed(key, self.dim)
        candidates: List[Tuple[float, MemoryEntry]] = []
        channels = [self.channels[channel]] if channel else list(self.channels.values())
        for ch in channels:
            for entry in ch.entries:
                sim = _cosine(k, entry.key)
                if sim > 0.0:
                    candidates.append((sim * entry.strength, entry))
        candidates.sort(key=lambda t: t[0], reverse=True)
        top = candidates[: self.top_k]
        if not top:
            return MemoryRead(value=None, outcome=None, confidence=0.0, hits=0, entries_hit=0)

        weight_sum = sum(w for w, _ in top)
        value = [0.0] * self.dim
        for w, entry in top:
            for i in range(self.dim):
                value[i] += w * entry.value[i]
        value = [x / weight_sum for x in value]

        tally: Dict[str, float] = {}
        for w, entry in top:
            if entry.label is not None:
                tally[entry.label] = tally.get(entry.label, 0.0) + w
        outcome: Optional[str] = None
        confidence = 0.0
        if tally:
            label, votes = max(tally.items(), key=lambda kv: kv[1])
            confidence = votes / weight_sum
            if confidence >= self.outcome_threshold:
                outcome = label

        return MemoryRead(
            value=value,
            outcome=outcome,
            confidence=confidence,
            hits=sum(e.hits for _, e in top),
            entries_hit=len(top),
        )

    def predict_outcome(self, key: str) -> Optional[Tuple[str, float]]:
        """Convenience: predicted verdict for a key, or None below threshold."""
        read = self.lookup(key)
        if read.outcome is None:
            return None
        return read.outcome, read.confidence

    def decay(self, steps: int = 1) -> None:
        """Advance the agent time-step: per-channel exponential forgetting + eviction."""
        for ch in self.channels.values():
            ch.apply_decay(steps)

    def stats(self) -> Dict[str, Any]:
        return {
            "dim": self.dim,
            "eps": self.eps,
            "writes": self.writes,
            "converged_writes": self.converged_writes,
            "channels": {
                name: {
                    "entries": len(ch.entries),
                    "alpha": ch.alpha,
                    "strength": round(ch.total_strength, 4),
                    "hits": ch.total_hits,
                }
                for name, ch in self.channels.items()
            },
        }


__all__ = [
    "AgentDeltaMemory",
    "MemoryChannel",
    "MemoryEntry",
    "MemoryRead",
    "MemoryWrite",
    "DEFAULT_CHANNELS",
    "embed",
]
