# 🎓 Theoretical Foundations

DSpark synthesizes advancements across multi-agent epistemology, speculative decoding, and formal verification.

---

## 1. The Self-Correction Fallacy

Huang et al. (2023) demonstrated that Large Language Models struggle with intrinsic self-correction when reviewing their own output. When a model re-examines code it generated, its prior probability distribution $\pi_\theta(y|x)$ biases its assessment towards affirming its own hallucinations (*Self-Confirmation Bias*).

**DSpark's Solution:** Epistemic isolation. The **Curator** is an independent, distinct model (e.g. DeepSeek v4 Pro) that receives ONLY the raw code and explicit I/O contracts. It never receives the Creator's reasoning trace or user prompt.

---

## 2. Semi-Autoregressive Speculative Decoding

Inspired by DeepSeek's DSpark (2026), speculative decoding at the agent level generates $N$ diverse execution paths concurrently:

$$\{ \tau_1, \tau_2, \dots, \tau_N \} \sim \text{Creator}(\text{spec})$$

Instead of sequential drafting, concurrency is bounded via token buckets, reducing wall-clock latency while expanding search entropy across the algorithmic solution space.

---

## 3. Probabilistic Pivot Tournament (PPT)

Standard Best-of-$N$ verification scales as $O(N^2)$ all-pairs comparisons:

$$\text{Comparisons}_{\text{all-pairs}} = \frac{N(N-1)}{2}$$

The PPT algorithm (Kwok et al., 2026) reduces this to $O(Nk)$ via a three-phase tournament:
1. **Hamiltonian Ring Pass**: Evaluates candidates $(i \to i+1 \pmod N)$.
2. **Pivot Selection**: Extracts the $k$ highest-scoring trajectories as anchors.
3. **Anchor Tournament**: Evaluates non-anchor candidates strictly against the $k$ pivots.

$$\text{Comparisons}_{\text{PPT}} = N + (N-k)k + \frac{k(k-1)}{2} = O(Nk)$$

---

## 4. Fine-Grained Reward Decomposition

Verification evaluates candidates across three weighted criteria dimensions:
- **Specification & Boundary Coverage**: 35%
- **I/O Contract Safety & Invariants**: 35%
- **Idiomatic Correctness & Performance**: 30%

$$R(x, \tau) = \frac{1}{\sum w_c} \sum_{c=1}^C w_c \cdot \phi_c(x, \tau)$$
