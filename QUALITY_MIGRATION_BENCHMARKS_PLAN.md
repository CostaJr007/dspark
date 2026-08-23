# 🏗️ DSpark: Plano de Qualidade, Migração e Benchmarks
## Complemento ao MASTER_PLAN_SPECULATIVE_ORCHESTRATION.md

Este documento contém **3 planos de execução completos** para elevar o DSpark ao nível de produção enterprise-grade. Siga na ordem apresentada.

---

# 📋 PARTE 1: PLANO DE TESTES UNITÁRIOS COMPLETO

## 1.1 Cobertura Alvo por Módulo

| Módulo | Linhas | Cobertura Alvo | Prioridade | Complexidade |
|---|---|---|---|---|
| `pivot_tournament.rs` | 226 | 95% | 🔴 P0 | Alta |
| `speculative_drafter.rs` | 91 | 90% | 🔴 P0 | Alta |
| `logprob_extractor.rs` | 136 | 90% | 🟡 P1 | Média |
| `cost_scheduler.rs` | 78 | 85% | 🟡 P1 | Baixa |
| `confidence_head.rs` | 118 | 85% | 🟡 P1 | Baixa |

## 1.2 Estrutura de Arquivos de Teste

```
crates/dspark-core/
├── src/
│   ├── engine/
│   │   ├── pivot_tournament.rs
│   │   ├── speculative_drafter.rs
│   │   ├── logprob_extractor.rs
│   │   ├── cost_scheduler.rs
│   │   └── confidence_head.rs
│   └── ...
└── tests/
    ├── engine/
    │   ├── mod.rs
    │   ├── pivot_tournament_test.rs
    │   ├── speculative_drafter_test.rs
    │   ├── logprob_extractor_test.rs
    │   ├── cost_scheduler_test.rs
    │   └── confidence_head_test.rs
    ├── utils/
    │   ├── mod.rs
    │   └── ast_resolver_test.rs
    └── mocks/
        ├── mod.rs
        └── mock_client.rs
```

---

# 🌲 PARTE 2: MIGRAÇÃO REGEX → TREE-SITTER

## 2.1 Justificativa Técnica
- Suporte a AST preciso, detecção de dependências transitivas e eliminação de falsos positivos em strings/comentários.
- Feature flags: `default = ["regex-ast"]` para build rápido, `tree-sitter-ast` para verificação rigorosa.

---

# 📊 PARTE 3: BENCHMARKS COM CRITERION

## 3.1 Objetivos dos Benchmarks
1. Provar O(Nk) vs O(N²) do Pivot Tournament.
2. Medir ganho de pruning local do Confidence Head (40-70% economia).
3. Comparar Regex vs Tree-sitter (latência e throughput).
4. Medir o ganho de KV cache do Prompt Optimizer.
