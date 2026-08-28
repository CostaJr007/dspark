//! AgentDeltaMemory: agent-level adaptation of the Kimi Delta Attention (KDA) memory dynamics.
//!
//! Theory (Kimi Team, 2025 - "Kimi Linear: An Expressive, Efficient Attention Architecture",
//! arXiv:2510.26692): KDA maintains a fixed-size matrix state S_t updated by a fine-grained
//! gated delta rule:
//!
//! ```text
//! S_t = (I - beta_t k_t k_t^T) Diag(alpha_t) S_{t-1} + beta_t k_t v_t^T
//! ```
//!
//! Ported to agent orchestration:
//! - **delta rule**: before writing, look up the memory's current prediction for key k_t and
//!   write only the correction (online gradient descent on a reconstruction loss). When the
//!   correction norm falls below `eps` the memory has converged ("already knew" the outcome),
//!   a principled stop criterion for refinement loops;
//! - **per-channel forgetting** (`Diag(alpha_t)`): each memory channel decays at its own rate,
//!   so invariants (alpha ~ 1) are retained while transient facts fade;
//! - **key-bound (DPLR) updates**: the DPLR low-rank factors are bound to the key itself
//!   (a = beta*k, b = k*alpha), making every update rank-1 *along the current key* -- surgical,
//!   with no collateral damage to unrelated memory.
//!
//! Evidence strength (`beta`) maps to trust: sandbox-approved verdicts (1.0) > audit verdicts
//! (0.7) > draft-level observations (0.6). The state stays bounded (fixed capacity per channel),
//! mirroring KDA's fixed-size RNN state replacing the growing KV cache.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Embedding dimension of the memory state (analog of d_k).
pub const DIM: usize = 64;
pub type Embedding = [f64; DIM];

/// serde adapter: `[f64; DIM]` serializes as a sequence (serde only implements
/// arrays up to length 32 natively).
mod embedding_serde {
    use serde::{de::Error as _, Deserialize, Deserializer, Serializer};

    use super::{Embedding, DIM};

    pub fn serialize<S: Serializer>(e: &Embedding, s: S) -> Result<S::Ok, S::Error> {
        s.collect_seq(e.iter())
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Embedding, D::Error> {
        let v = Vec::<f64>::deserialize(d)?;
        if v.len() != DIM {
            return Err(D::Error::invalid_length(v.len(), &"length 64"));
        }
        let mut e = [0.0f64; DIM];
        e.copy_from_slice(&v);
        Ok(e)
    }
}

/// serde adapter for `Option<Embedding>`.
mod option_embedding_serde {
    use serde::{Deserialize, Deserializer, Serializer};

    use super::Embedding;

    pub fn serialize<S: Serializer>(opt: &Option<Embedding>, s: S) -> Result<S::Ok, S::Error> {
        match opt {
            Some(e) => super::embedding_serde::serialize(e, s),
            None => s.serialize_none(),
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Option<Embedding>, D::Error> {
        Option::<Vec<f64>>::deserialize(d).map(|v| v.map(|vec| {
            let mut e = [0.0f64; super::DIM];
            let n = vec.len().min(super::DIM);
            e[..n].copy_from_slice(&vec[..n]);
            e
        }))
    }
}

#[derive(Debug, Clone, thiserror::Error, PartialEq)]
pub enum MemoryError {
    #[error("unknown memory channel: {0}")]
    UnknownChannel(String),
}

/// FNV-1a 64-bit hash for the deterministic hashing-trick embedding.
fn fnv1a(text: &str) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in text.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

/// Deterministic hashing-trick embedding of a text into a unit vector.
pub fn embed(text: &str) -> Embedding {
    let mut vec = [0.0f64; DIM];
    let mut token = String::new();
    for c in text.chars() {
        if c.is_ascii_alphanumeric() || c == '_' {
            token.push(c.to_ascii_lowercase());
        } else if !token.is_empty() {
            let h = fnv1a(&token);
            let idx = (h % DIM as u64) as usize;
            vec[idx] += if (h >> 63) & 1 == 1 { 1.0 } else { -1.0 };
            token.clear();
        }
    }
    if !token.is_empty() {
        let h = fnv1a(&token);
        let idx = (h % DIM as u64) as usize;
        vec[idx] += if (h >> 63) & 1 == 1 { 1.0 } else { -1.0 };
    }
    let norm = vec.iter().map(|x| x * x).sum::<f64>().sqrt();
    if norm > 0.0 {
        for x in &mut vec {
            *x /= norm;
        }
    }
    vec
}

fn norm(v: &[f64]) -> f64 {
    v.iter().map(|x| x * x).sum::<f64>().sqrt()
}

fn cosine(a: &[f64], b: &[f64]) -> f64 {
    let na = norm(a);
    let nb = norm(b);
    if na == 0.0 || nb == 0.0 {
        return 0.0;
    }
    a.iter().zip(b).map(|(x, y)| x * y).sum::<f64>() / (na * nb)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    #[serde(with = "embedding_serde")]
    pub key: Embedding,
    #[serde(with = "embedding_serde")]
    pub value: Embedding,
    pub strength: f64,
    pub label: Option<String>,
    pub hits: u64,
}

#[derive(Debug, Clone)]
pub struct MemoryChannel {
    pub name: String,
    pub alpha: f64,
    pub beta_max: f64,
    pub capacity: usize,
    pub key_similarity: f64,
    pub min_strength: f64,
    pub entries: Vec<MemoryEntry>,
}

impl MemoryChannel {
    pub fn new(
        name: &str,
        alpha: f64,
        beta_max: f64,
        capacity: usize,
        key_similarity: f64,
    ) -> Self {
        Self {
            name: name.to_string(),
            alpha,
            beta_max,
            capacity,
            key_similarity,
            min_strength: 0.01,
            entries: Vec::new(),
        }
    }

    /// Nearest entry by key cosine; returns (index, similarity).
    fn nearest(&self, k: &[f64]) -> Option<(usize, f64)> {
        let mut best: Option<(usize, f64)> = None;
        for (idx, entry) in self.entries.iter().enumerate() {
            let sim = cosine(k, &entry.key);
            if best.is_none_or(|(_, best_sim)| sim > best_sim) {
                best = Some((idx, sim));
            }
        }
        best
    }

    fn add(&mut self, entry: MemoryEntry) {
        self.entries.push(entry);
        self.entries.sort_by(|a, b| b.strength.partial_cmp(&a.strength).unwrap_or(std::cmp::Ordering::Equal));
        self.entries.truncate(self.capacity);
    }

    fn apply_decay(&mut self, steps: usize) {
        let decay = self.alpha.powi(steps as i32);
        self.entries.retain_mut(|entry| {
            entry.strength *= decay;
            entry.strength >= self.min_strength
        });
        self.entries.sort_by(|a, b| b.strength.partial_cmp(&a.strength).unwrap_or(std::cmp::Ordering::Equal));
        self.entries.truncate(self.capacity);
    }

    pub fn total_strength(&self) -> f64 {
        self.entries.iter().map(|e| e.strength).sum()
    }

    pub fn total_hits(&self) -> u64 {
        self.entries.iter().map(|e| e.hits).sum()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryWrite {
    pub delta_norm: f64,
    pub converged: bool,
    pub updated: bool,
    pub entry_count: usize,
    pub channel: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryRead {
    #[serde(with = "option_embedding_serde")]
    pub value: Option<Embedding>,
    pub outcome: Option<String>,
    pub confidence: f64,
    pub hits: u64,
    pub entries_hit: usize,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MemoryStats {
    pub writes: u64,
    pub converged_writes: u64,
    pub entries: usize,
    pub channel_entries: HashMap<String, usize>,
    pub total_strength: f64,
}

/// Fixed-size, delta-rule agent memory with per-channel forgetting (KDA-derived).
pub struct AgentDeltaMemory {
    pub eps: f64,
    pub top_k: usize,
    pub outcome_threshold: f64,
    pub channels: HashMap<String, MemoryChannel>,
    writes: u64,
    converged_writes: u64,
}

impl Default for AgentDeltaMemory {
    fn default() -> Self {
        Self::new()
    }
}

impl AgentDeltaMemory {
    pub fn new() -> Self {
        let mut channels = HashMap::new();
        channels.insert(
            "invariant".to_string(),
            MemoryChannel::new("invariant", 0.999, 1.0, 128, 0.45),
        );
        channels.insert(
            "decision".to_string(),
            MemoryChannel::new("decision", 0.97, 1.0, 256, 0.45),
        );
        channels.insert(
            "transient".to_string(),
            MemoryChannel::new("transient", 0.80, 0.6, 512, 0.45),
        );
        Self {
            eps: 1e-3,
            top_k: 3,
            outcome_threshold: 0.5,
            channels,
            writes: 0,
            converged_writes: 0,
        }
    }

    /// Majority label semantics: conflicting labels void the entry's vote.
    fn merge_label(current: Option<&str>, incoming: Option<&str>) -> Option<String> {
        match (current, incoming) {
            (None, None) => None,
            (Some(c), None) | (None, Some(c)) => Some(c.to_string()),
            (Some(c), Some(i)) if c == i => Some(c.to_string()),
            _ => None,
        }
    }

    /// Delta-rule write: correct the nearest key-bound entry, or insert.
    ///
    /// Returns `converged = true` when the correction norm is below `eps`: the memory
    /// already predicted this value, so the surrounding pipeline stopped learning.
    pub fn write(
        &mut self,
        key: &str,
        value: &str,
        channel: &str,
        beta: Option<f64>,
        label: Option<&str>,
    ) -> Result<MemoryWrite, MemoryError> {
        let ch = self
            .channels
            .get_mut(channel)
            .ok_or_else(|| MemoryError::UnknownChannel(channel.to_string()))?;
        let k = embed(key);
        let v = embed(value);
        let beta = beta.unwrap_or(ch.beta_max).min(ch.beta_max);

        if let Some((idx, sim)) = ch.nearest(&k) {
            if sim >= ch.key_similarity {
                let pred = ch.entries[idx].value;
                let mut delta = [0.0f64; DIM];
                for (i, (p, x)) in pred.iter().zip(v).enumerate() {
                    delta[i] = x - p;
                }
                let delta_norm = norm(&delta);
                ch.entries[idx].label = Self::merge_label(ch.entries[idx].label.as_deref(), label);
                if delta_norm < self.eps {
                    self.writes += 1;
                    self.converged_writes += 1;
                    ch.entries[idx].hits += 1;
                    return Ok(MemoryWrite {
                        delta_norm,
                        converged: true,
                        updated: false,
                        entry_count: ch.entries.len(),
                        channel: channel.to_string(),
                    });
                }
                // Delta rule: write only the correction along the key direction (rank-1).
                for (i, x) in ch.entries[idx].value.iter_mut().enumerate() {
                    *x = pred[i] + beta * delta[i];
                }
                ch.entries[idx].strength = (ch.entries[idx].strength + beta).min(1.0);
                ch.entries[idx].hits += 1;
                self.writes += 1;
                return Ok(MemoryWrite {
                    delta_norm,
                    converged: false,
                    updated: true,
                    entry_count: ch.entries.len(),
                    channel: channel.to_string(),
                });
            }
        }

        ch.add(MemoryEntry {
            key: k,
            value: v,
            strength: beta,
            label: label.map(str::to_string),
            hits: 1,
        });
        self.writes += 1;
        let delta_norm = norm(&v);
        Ok(MemoryWrite {
            delta_norm,
            converged: delta_norm < self.eps,
            updated: true,
            entry_count: ch.entries.len(),
            channel: channel.to_string(),
        })
    }

    /// Query the memory (read `S^T q`).
    ///
    /// ``outcome`` is the weighted label vote of the top-k nearest entries, reported
    /// when its weight fraction clears ``outcome_threshold``.
    pub fn lookup(&self, key: &str, channel: Option<&str>) -> MemoryRead {
        let k = embed(key);
        let mut candidates: Vec<(f64, &MemoryEntry)> = Vec::new();
        let channels: Vec<&MemoryChannel> = match channel {
            Some(name) => self.channels.get(name).into_iter().collect(),
            None => self.channels.values().collect(),
        };
        for ch in channels {
            for entry in &ch.entries {
                let sim = cosine(&k, &entry.key);
                if sim > 0.0 {
                    candidates.push((sim * entry.strength, entry));
                }
            }
        }
        candidates.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        candidates.truncate(self.top_k);
        if candidates.is_empty() {
            return MemoryRead {
                value: None,
                outcome: None,
                confidence: 0.0,
                hits: 0,
                entries_hit: 0,
            };
        }

        let weight_sum: f64 = candidates.iter().map(|(w, _)| w).sum();
        let mut value = [0.0f64; DIM];
        for (w, entry) in &candidates {
            for (i, x) in value.iter_mut().enumerate() {
                *x += w * entry.value[i];
            }
        }
        for x in &mut value {
            *x /= weight_sum;
        }

        let mut tally: HashMap<&str, f64> = HashMap::new();
        for (w, entry) in &candidates {
            if let Some(label) = entry.label.as_deref() {
                *tally.entry(label).or_insert(0.0) += w;
            }
        }
        let mut outcome = None;
        let mut confidence = 0.0;
        if let Some((label, votes)) = tally.iter().max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal)) {
            confidence = votes / weight_sum;
            if confidence >= self.outcome_threshold {
                outcome = Some(label.to_string());
            }
        }

        MemoryRead {
            value: Some(value),
            outcome,
            confidence,
            hits: candidates.iter().map(|(_, e)| e.hits).sum(),
            entries_hit: candidates.len(),
        }
    }

    /// Convenience: predicted verdict for a key, or None below threshold.
    pub fn predict_outcome(&self, key: &str) -> Option<(String, f64)> {
        let read = self.lookup(key, None);
        read.outcome.map(|outcome| (outcome, read.confidence))
    }

    /// Advance the agent time-step: per-channel exponential forgetting + eviction.
    pub fn decay(&mut self, steps: usize) {
        for ch in self.channels.values_mut() {
            ch.apply_decay(steps);
        }
    }

    pub fn stats(&self) -> MemoryStats {
        let entries: usize = self.channels.values().map(|c| c.entries.len()).sum();
        let total_strength: f64 = self.channels.values().map(|c| c.total_strength()).sum();
        MemoryStats {
            writes: self.writes,
            converged_writes: self.converged_writes,
            entries,
            channel_entries: self
                .channels
                .iter()
                .map(|(name, ch)| (name.clone(), ch.entries.len()))
                .collect(),
            total_strength,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_then_lookup_returns_value() {
        let mut mem = AgentDeltaMemory::new();
        let write = mem
            .write("contract:python:abs_val", "result >= 0", "invariant", Some(0.95), None)
            .unwrap();
        assert!(write.updated);
        assert!(!write.converged);

        let read = mem.lookup("contract:python:abs_val", Some("invariant"));
        assert!(read.value.is_some());
        assert_eq!(read.entries_hit, 1);
    }

    #[test]
    fn repeated_write_converges() {
        let mut mem = AgentDeltaMemory::new();
        let key = "ce:python:abs_val:abc123";
        let first = mem.write(key, "REJECTED", "transient", Some(0.6), Some("REJECTED")).unwrap();
        assert!(!first.converged);

        let second = mem.write(key, "REJECTED", "transient", Some(0.6), Some("REJECTED")).unwrap();
        assert!(second.converged, "delta rule must converge for identical outcomes");
        assert!(second.delta_norm < mem.eps);

        let third = mem.write(key, "REJECTED", "transient", Some(0.6), Some("REJECTED")).unwrap();
        assert!(third.converged);
        assert_eq!(mem.stats().converged_writes, 2);
    }

    #[test]
    fn per_channel_decay_invariant_survives_transient_evicted() {
        let mut mem = AgentDeltaMemory::new();
        mem.write("inv", "a = 1", "invariant", Some(1.0), None).unwrap();
        mem.write("trn", "b = 2", "transient", Some(0.6), None).unwrap();

        for _ in 0..200 {
            mem.decay(1);
        }

        let inv = mem.lookup("inv", Some("invariant"));
        let trn = mem.lookup("trn", Some("transient"));
        assert!(inv.value.is_some(), "invariants must survive decay (alpha ~ 1)");
        assert!(
            trn.value.is_none(),
            "transient entries must be evicted after many time-steps (alpha << 1)"
        );
    }

    #[test]
    fn key_bound_updates_are_surgical() {
        let mut mem = AgentDeltaMemory::new();
        mem.write("k1", "value_one", "decision", Some(0.5), None).unwrap();
        let before = mem.lookup("k1", Some("decision")).value.unwrap();

        // Unrelated key must not perturb the first entry (rank-1, key-bound update).
        mem.write("k2_completely_different", "value_two", "decision", Some(0.5), None).unwrap();
        let after = mem.lookup("k1", Some("decision")).value.unwrap();

        let diff = before
            .iter()
            .zip(after)
            .map(|(a, b)| (a - b).abs())
            .fold(0.0f64, |acc, x| acc + x);
        assert!(diff < 1e-9, "unrelated write mutated bound entry (diff={diff})");
    }

    #[test]
    fn outcome_projection_distinguishes_approved_and_rejected() {
        let mut mem = AgentDeltaMemory::new();
        mem.write("t1", "APPROVED score=100", "decision", Some(1.0), Some("APPROVED")).unwrap();
        mem.write("t2", "REJECTED score=40", "decision", Some(1.0), Some("REJECTED")).unwrap();

        assert_eq!(
            mem.predict_outcome("t1").map(|(o, _)| o),
            Some("APPROVED".to_string())
        );
        assert_eq!(
            mem.predict_outcome("t2").map(|(o, _)| o),
            Some("REJECTED".to_string())
        );
    }

    #[test]
    fn conflicting_labels_void_the_vote() {
        let mut mem = AgentDeltaMemory::new();
        mem.write("k", "ok", "decision", Some(0.5), Some("APPROVED")).unwrap();
        mem.write("k", "nope", "decision", Some(0.5), Some("REJECTED")).unwrap();
        let read = mem.lookup("k", Some("decision"));
        assert!(read.outcome.is_none(), "conflicting labels must not produce a verdict");
    }

    #[test]
    fn capacity_eviction_keeps_strongest() {
        let mut mem = AgentDeltaMemory::new();
        for i in 0..20 {
            mem.write(&format!("key-{i}"), "x", "decision", Some(0.1), None).unwrap();
        }
        let stats = mem.stats();
        assert!(stats.entries <= 256);
    }

    #[test]
    fn unknown_channel_errors() {
        let mut mem = AgentDeltaMemory::new();
        let err = mem.write("k", "v", "nope", None, None).unwrap_err();
        assert_eq!(err, MemoryError::UnknownChannel("nope".to_string()));
    }
}
