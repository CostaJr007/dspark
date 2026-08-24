# 🏛️ DSpark Architecture Specification

## 1. Architectural Overview

DSpark is structured around two foundational principles:
1. **CEGAR Dual-Engine Epistemic Isolation**: Code generation and adversarial verification are decoupled into discrete agents with strict information boundaries to prevent self-confirmation biases.
2. **Speculative Orchestration & Tournament Selection**: Multiple code variants are generated in parallel ($N$ trajectories), topologically sorted by syntax dependency graphs, pruned locally via CPU entropy checks, and ranked through an $O(Nk)$ Probabilistic Pivot Tournament before executing surgical CEGAR repairs on failures.

```mermaid
flowchart TD
    subgraph S1["Stage 1: Speculative Drafting"]
        A[User Specification & Contracts] --> B[Concurrent Async Drafter]
        B --> C1["Trajectory 1 (T=0.20)"]
        B --> C2["Trajectory 2 (T=0.35)"]
        B --> C3["Trajectory 3 (T=0.50)"]
        B --> Cn["Trajectory N (T=0.2+0.15*N)"]
    end

    subgraph S2["Stage 2: AST Dependency Resolution"]
        C1 & C2 & C3 & Cn --> D[AST Parser: Tree-Sitter / Regex]
        D --> E[Directed Acyclic Graph PetGraph]
        E --> F[Topological Invariant Sorter]
    end

    subgraph S3["Stage 3 & 4: Confidence & Cost Scheduling"]
        F --> G[Confidence Head: Cyclomatic Entropy on CPU]
        G --> H{"Risk Tier Assessment"}
        H -->|"Low Risk (Score > 0.88)"| I["Local Zero-Cost Approval\n(60-98% Pruning)"]
        H -->|"High Risk / Ambiguity"| J["Cost Scheduler\n(Budget Cap & Allocation)"]
    end

    subgraph S5["Stage 5: PPT Tournament Ranking"]
        J --> K["Hamiltonian Ring Pass (i → i+1 mod N)"]
        K --> L["Pivot Anchor Selection (Top-k)"]
        L --> M["Anchor Tournament: Non-Pivots vs Pivots O(Nk)"]
        M --> N["Selected Candidate Trajectory"]
    end

    subgraph S6["CEGAR Dual-Engine Refinement"]
        N --> O["Isolated Subprocess Sandbox Runner"]
        O --> P{"Deterministic Contract Result"}
        P -->|"PASS"| Q["🎉 Production Verified Code"]
        P -->|"FAIL"| R["Extract Counterexample (failure_tail)"]
        R --> S["Epistemically Isolated Curator (DeepSeek Flagship)"]
        S -->|"1-Shot Surgical Refine"| O
    end
```

---

## 2. The 5 Pipeline Stages

### Stage 1: Speculative Drafting (`crates/dspark-core/src/engine/speculative_drafter.rs`)
- **Mechanism**: Spawns $N$ concurrent generation tasks bounded by an asynchronous `tokio::sync::Semaphore`.
- **Temperature Scaling**: Uses diversified sampling temperature $T_i = 0.2 + (i \times 0.15)$ to balance precision and algorithmic exploration across the solution space.
- **Fail-Fast Filter**: Drops malformed or truncated responses early before downstream analysis.

### Stage 2: AST Dependency Resolution (`crates/dspark-core/src/utils/ast_resolver.rs`)
- **Mechanism**: Analyzes function definitions, imports, and call expressions.
- **DAG Construction**: Constructs a Directed Acyclic Graph using `petgraph::graph::DiGraph`.
- **Topological Sorting**: Enforces dependency-first ordering ($f_{\text{callee}} \to f_{\text{caller}}$). Detects and flags cyclic dependencies.
- **Pluggable Backends**:
  - `RegexResolver`: Instant linear regex scanner (~1ms/100 blocks).
  - `TreeSitterResolver`: Exact syntax tree parser supporting Rust and Python grammars (~10ms/100 blocks).

### Stage 3: Local Confidence Head (`crates/dspark-core/src/engine/confidence_head.rs`)
- **Metric Formula**:
  $$\text{Confidence} = 1.0 - (0.40 \cdot H_{\text{cyclomatic}}) - (0.25 \cdot \mathbf{1}_{\text{complex}}) - (0.20 \cdot \mathbf{1}_{\text{mutating}})$$
- **Risk Tiers**:
  - **Low Risk** ($\text{Confidence} > 0.88$): Skip remote LLM verification.
  - **Medium Risk** ($0.65 < \text{Confidence} \le 0.88$): Optional/budgeted verification.
  - **High Risk** ($\text{Confidence} \le 0.65$): Mandatory tournament verification.

### Stage 4: Cost-Aware Scheduling (`crates/dspark-core/src/engine/cost_scheduler.rs`)
- **Budgeting**: Ranks blocks by risk score and selects the top-$k$ critical items within `max_api_calls`.
- **Savings**: Prunes between 60% and 98% of candidate code blocks, enforcing a hard spend ceiling per task.

### Stage 5: Probabilistic Pivot Tournament (`crates/dspark-core/src/engine/pivot_tournament.rs`)
The implemented algorithm performs exactly:
$$\text{Comparisons}(N, k) = N + (N-k)k + \binom{k}{2} = O(Nk)$$

1. **Hamiltonian Ring Pass**: Evaluates candidates $(i \to i+1 \pmod N)$.
2. **Pivot Selection**: Extracts the $k$ highest-scoring trajectories as anchors.
3. **Anchor Tournament**: Evaluates non-anchor candidates strictly against the $k$ pivots.

---

## 3. CEGAR Loop & Epistemic Isolation

```mermaid
sequenceDiagram
    autonumber
    actor Dev as Developer / IDE Agent
    participant DSpark as DSpark Orchestrator
    participant Creator as Cheap Drafter (gpt-4o-mini / local)
    participant Sandbox as OS Subprocess Sandbox
    participant Curator as Epistemic Curator (DeepSeek Flagship)

    Dev->>DSpark: Submit Task & I/O Contracts
    DSpark->>Creator: Generate N Speculative Trajectories
    Creator-->>DSpark: N Code Drafts
    DSpark->>DSpark: AST Sort + Confidence Pruning + PPT Tournament
    DSpark->>Sandbox: Execute Top Candidate vs Pytest / Cargo Contracts
    
    alt All Tests Pass
        Sandbox-->>DSpark: 100% PASS (Zero errors)
        DSpark-->>Dev: Return Verified Production Code
    else Test Fails (Contract Violation)
        Sandbox-->>DSpark: FAIL + Traceback (failure_tail)
        Note over DSpark,Curator: Epistemic Isolation: Curator receives ONLY code + error (no creator CoT)
        DSpark->>Curator: Refine Code with Concrete Counterexample
        Curator-->>DSpark: Corrected Full Implementation
        DSpark->>Sandbox: Re-run Sandbox Validation
        Sandbox-->>DSpark: 100% PASS
        DSpark-->>Dev: Return Verified Production Code
    end
```

---

## 4. MCP (Model Context Protocol) Architecture

DSpark exposes its core engine to external IDEs via a standardized FastMCP server:

```mermaid
graph LR
    subgraph Clients["Supported IDEs & Clients"]
        Cursor["Cursor IDE"]
        Claude["Claude Desktop / Code"]
        Windsurf["Windsurf IDE"]
        AGY["Antigravity CLI"]
    end

    subgraph MCP["FastMCP Protocol Server (dspark.mcp.server)"]
        AuditTool["dspark_audit"]
        RefineTool["dspark_refine"]
        PipelineTool["dspark_verify_pipeline"]
    end

    subgraph Core["DSpark Dual Engine"]
        CEGAR["CEGAR Pipeline"]
        ASTCore["dspark-core (Rust)"]
        DSClient["DeepSeek v4 Pro / Flash Client"]
    end

    Cursor & Claude & Windsurf & AGY <-->|"JSON-RPC / stdio"| MCP
    MCP <--> Core
```
