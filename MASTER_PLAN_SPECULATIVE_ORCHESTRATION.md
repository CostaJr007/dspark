# 🚀 DSpark Master Plan: Agent-Level Speculative Orchestration

## Implementando Speculative Decoding (DeepSeek) + LLM-as-a-Verifier no dspark-core

---

## 📋 1. VISÃO GERAL E OBJETIVOS

### 1.1 Contexto
Este documento especifica a evolução do `dspark-core` de uma ferramenta de verificação dual-engine para um **Sistema de Orquestração Especulativa de Agentes**, implementando as metodologias de:
- **DSpark** (DeepSeek & Peking University, 2026): *Confidence-Scheduled Speculative Decoding*
- **LLM-as-a-Verifier** (Kwok et al., 2026): *Probabilistic Pivot Tournament* e *Fine-Grained Reward Estimation*

### 1.2 Objetivo Final
Transformar o `dspark run` em um pipeline que:
1. ✅ Gera múltiplas trajetórias de código em paralelo (*Semi-Autoregressive Drafting*)
2. ✅ Valida dependências via AST antes de verificar (*Sequential Dependency Injection*)
3. ✅ Calcula confiança localmente para evitar chamadas desnecessárias à API (*Confidence-Scheduled Verification*)
4. ✅ Rankeia as melhores soluções via torneio probabilístico O(Nk) (*Probabilistic Pivot Tournament*)
5. ✅ Extrai logprobs para feedback denso (*Fine-Grained Reward via Logprobs*)
6. ✅ Otimiza prompts para maximizar cache hit (*Prefix-Cache Optimization*)

### 1.3 Benefícios Esperados
- **Redução de 60-80%** no custo de API (DeepSeek) via pruning de verificação
- **Redução de 50%** na latência total via speculative execution
- **Aumento de 40%** na qualidade do código final via best-of-N eficiente

---

## 🏗️ 2. ARQUITETURA PROPOSTA

### 2.1 Fluxo de Dados do Novo Pipeline

```
[Task Prompt + I/O Contracts]
            |
            v
[STAGE 1: Speculative Drafter] --(N=3-5 trajectories)--> [Pool of Drafts]
            |                                                    |
            | [Tree-sitter Sequential Module]                    |
            v                                                    v
[STAGE 2: Dependency Resolver] --(valid drafts only)--> [Valid Draft Pool]
            |                                                    |
            | [Local Confidence Head]                            |
            v                                                    v
[STAGE 3: Cost Scheduler] --(high-risk blocks only)--> [Verification Batch]
            |                                                    |
            v                                                    v
[STAGE 4: Pivot Tournament] --(O(Nk) comparisons)--> [Best Trajectory]
            |
            v
[STAGE 5: Logprob-Aware Refiner] --(dense feedback)--> [Verified Code]
```

### 2.2 Estrutura de Diretórios Atualizada

```
crates/dspark-core/src/
├── lib.rs                          # Entry point
├── cli/                            # Comandos CLI
│   ├── mod.rs
│   ├── run.rs                      # 🆕 NOVO: Modo especulativo
│   ├── audit.rs
│   ├── refine.rs
│   └── pair.rs
├── engine/                         # 🆕 NOVO MÓDULO: Orquestrador
│   ├── mod.rs
│   ├── speculative_drafter.rs      # Geração paralela + Tree-sitter
│   ├── confidence_head.rs          # Estimativa local de confiança
│   ├── cost_scheduler.rs           # Hardware/Cost-Aware Scheduler
│   ├── pivot_tournament.rs         # Algoritmo PPT O(Nk)
│   └── logprob_extractor.rs        # Extração e processamento de logprobs
├── verifier/                       # Verificador existente
│   ├── mod.rs
│   ├── contracts.rs
│   └── deepseek_client.rs          # ⚠️ ATUALIZAR: Suporte a logprobs
├── creator/                        # Gerador existente
│   ├── mod.rs
│   └── gemini_client.rs
└── utils/
    ├── prompt_optimizer.rs         # 🆕 NOVO: Prefix-cache friendly prompts
    └── ast_resolver.rs             # 🆕 NOVO: Tree-sitter dependency injection
```

---

## 📦 3. DEPENDÊNCIAS E TOOLCHAIN

### 3.1 Atualizações no `Cargo.toml`

Adicionar ao `crates/dspark-core/Cargo.toml`:

```toml
[dependencies]
tokio = { version = "1.0", features = ["full", "sync"] }
reqwest = { version = "0.11", features = ["json", "blocking"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# 🆕 NOVAS DEPENDÊNCIAS
tree-sitter = "0.22"                    # AST parsing para dependency injection
tree-sitter-rust = "0.21"               # Grammar para Rust
tree-sitter-python = "0.21"             # Grammar para Python
rayon = "1.8"                           # Parallel iterators para speculative drafting
petgraph = "0.6"                        # Dependency graph analysis
statrs = "0.16"                         # Statistical functions para logprobs
ordered-float = "4.2"                   # Float comparisons precisas
```

---

## 📅 4. PLANO DE EXECUÇÃO E FASES

- **Fase 1: Fundação & Drafting**: `Cargo.toml`, `utils/ast_resolver.rs`, `engine/speculative_drafter.rs`.
- **Fase 2: Confidence Head & Cost Scheduler**: `engine/confidence_head.rs`, `engine/cost_scheduler.rs`.
- **Fase 3: Probabilistic Pivot Tournament**: `engine/pivot_tournament.rs` (O(Nk) vs O(N²)).
- **Fase 4: Logprobs & Prefix Cache**: `engine/logprob_extractor.rs`, `utils/prompt_optimizer.rs`, DeepSeek client logprobs.
- **Fase 5: CLI & Pipeline E2E**: `cli/run.rs` (`--speculative`), testes e documentação.

---
