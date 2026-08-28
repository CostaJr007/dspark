# Status da sessão — 2026-08-28 (FINALIZAÇÃO)

## Fase de finalização concluída

### Ajustes aplicados (com base nos achados medidos)
1. **Q3 do bench real agora é fiel ao paper**: o braço "discreto" usa a quantização
   grosseira 1-5 dos scores (ties reais no mesmo bucket) e o "soft" usa Bradley-Terry
   dos 1-20 — granularity scaling do paper na MESMA chamada (custo zero extra).
2. **N_DRAFTS 3→5** (temperaturas 0.1–0.9) para dar sinal ao tournament.
3. **`--judge-model` no bench** + aviso no CLI quando ranking tier == draft tier.
4. **Achado publicável (dados reais)**:
   - juiz do mesmo tier: PPT 70% < first-pass 90% (Q1 −6pts)
   - juiz estritamente mais forte: **PPT 100%** (Q1 +22.5pts)
   - tiered+escalação: **100%** em todas as configurações (48 tasks, ≈$0.05)

### Documentação finalizada
- `README.md`: badges (64 Rust + 37 Python), features novas (KDA memory, soft updates,
  continuous rewards, STS, sequential pass, VOC, K), seção Verification-Scaling A/B com
  tabelas offline+real, achado judge-tier, comandos novos.
- `docs/CHANGELOG.md`: entrada [0.3.0] com todas as features, o fix do bug de colisão
  de chaves da memória, e números medidos (offline + real).

### Validação final (tudo verde)
- Python: 37/37 ✓ | Rust: 64/64 ✓ | clippy limpo ✓

### Arquivos de resultados reais
- `bench/results/results_1787810974.jsonl` (4o-mini, 19 tasks)
- `bench/results/results_1787890859.jsonl` (3.5-turbo, 11 tasks, juiz same-tier)
- `bench/results/results_1787893989.jsonl` (3.5-turbo, 10 tasks, N=5)
- `bench/results/results_1787894337.jsonl` (3.5-turbo + juiz deepseek-chat, 8 tasks)
- `bench/show_results.py` e `bench/compare_vt.py` para análise

### Pendências restantes
- Commit git (perguntar ao usuário).
- Opcional futuro: STS com dados reais de produção; Q3 com logprobs quando o backend expor.

---

# Histórico — continuação 2026-08-28 (manhã)

## Feito nesta continuação

### 1. Sequential Dependency Pass LIGADO no CLI (pendência #1 concluída)
- `crates/dspark-core/src/main.rs`: nova flag `--sequential` (default **ligada**) no `run --speculative`.
- Fluxo: draft paralelo → `sequential_dependency_pass` (re-draft de cada trajetória condicionada no prefixo aceito — análogo do sequential head do DSpark) → confidence → scheduler → PPT → escalação.
- Sanity offline com mock: fluxo [1/5] completo com a injeção rodando.

### 2. Bench real com sinal (pendência #2 parcial)
- `bench/run_real_bench.py --limit 12 --cheap-model gpt-3.5-turbo` (11 tasks, ~$0.01).
- Resultados: `bench/results/results_1787890859.jsonl` + `bench/compare_vt.py` (comparação V vs T vs C).
- Tabela honesta (compare_vt.py):
  - **gpt-4o-mini** (19 tasks): V=94.7%, T=94.7%, C=94.7%, D=100% — sem sinal (tasks fáceis)
  - **gpt-3.5-turbo** (11 tasks): V=**90.9%**, T=**81.8%**, C=81.8%, D=**100%**
    - PPT (binário E soft empatados) errou em HumanEval/10: tinha draft PASS (1/3) e escolheu um FAIL; escalação consertou.
    - Q3 bin-vs-soft: 0 divergências em todos os runs — ver achado estrutural abaixo.

## ACHADO ESTRUTURAL IMPORTANTE (leitura honesta)
**Q3 (binário vs soft) no bench real é estruturalmente fraco**: as duas políticas consomem a
MESMA resposta do juiz (winner + scores), e um juiz consistente produz o mesmo pick.
A divergência bin-vs-soft só aparece quando: (a) o winner contradiz os scores, ou
(b) a escala discreta colapsa em ties — e o nosso parser binário não colapsa (usa o campo winner).
A demonstração fiel do paper (E2 offline: discreto 84%/tie 6% vs contínuo 100%/tie 0%) é a
comparação argmax-discreto vs expectativa-contínua sobre os MESMOS logits — não duas
leituras do mesmo JSON. Para o bench real discriminar, o juiz deveria dar só o winner
(escala discreta) num braço e scores 1-20 no outro (2 chamadas), ou usar logprobs.

**O valor REAL medido no bench é o D (100% em ambos): a escalação flagship é quem resolve**
**o que o tournament erra. O tournament ainda não provou valor vs V (first-pass scan) em
tasks fáceis — e perdeu para V no gpt-3.5 (81.8% vs 90.9%, n=11).**

## Pendências (próximos passos)
1. **Melhorar a comparação PPT vs V no bench real**: mais tasks + drafts (N=5) para o
   tournament ter o que discriminar; avaliar o prompt do juiz (truncado em 2200 chars).
2. **Q3 fiel ao paper**: 2 chamadas por par (juiz discreto vs juiz 1-20) OU usar logprobs
   quando o backend expõe.
3. **STS em produção**: coletar (confiança → aceite) reais para calibrar o ConfidenceHead.
4. **Commit git** (não feito).
5. Paper: seção com recorrência KDA + teorema δ→0 + não-antecipação + resultados.

## Validação
- Python 37 ✓ | Rust 64 ✓ + clippy limpo (reconfirmado após wiring do CLI).
- Bench real acumulado: 19 tasks (4o-mini) + 11 tasks (3.5-turbo) ≈ $0.025 gastos.

---

# Histórico — 2026-08-27 (sessão anterior, resumo)


Estado atual do trabalho de pesquisa/implementação em `C:\Users\adeil\dspark`.
Tudo validado no estado descrito abaixo. Nada pendente de "salvar" além de commit git (não foi feito — opcional amanhã).

## O que foi feito nesta sessão

### 1. Papers baixados e estudados (PDFs em `docs/papers/`)
- `dspark_deepseek_2607.05147.pdf` — DSpark: Confidence-Scheduled Speculative Decoding (DeepSeek-AI + Peking Univ.)
- `llm_as_verifier_2607.05391.pdf` — LLM-as-a-Verifier (Kwok et al., Stanford/Berkeley/NVIDIA)
- KDA/Kimi Linear (arXiv:2510.26692) estudado via HTML (não baixado em PDF).

### 2. Memória de agente KDA-derivada (implementado + integrado)
- `dspark/memory.py` — `AgentDeltaMemory`: delta rule, decay per-channel (invariant/decision/transient), updates key-bound (rank-1), convergência δ→0, readout por voto de label.
- Integrado no `dspark/pipeline/cegar.py`: invariantes (contratos), decisões (vereditos), transients (contraexemplos), **early-stop por convergência da memória** (`memory_stable`).
- Rust: `crates/dspark-core/src/engine/agent_memory.rs` (espelho completo + serde custom para `[f64;64]`).
- **Bug crítico corrigido**: chaves `ce:{lang}:{fn}:{digest}` colidiam entre contraexemplos distintos (cosine 0.75 > 0.45) → early-stop indevido. Fix: digest em 3-4 chunks dominando a chave (0.40/0.33 < 0.45). Testes de regressão adicionados.

### 3. Melhorias dos papers (todas implementadas e testadas)
Rust (`crates/dspark-core/src/engine/`):
- `pivot_tournament.rs` — **soft updates Bradley-Terry** (`p = σ(R_a−R_b)`, scores 1-20 parseados, fallback binário, EQUAL→0.5). `comparison_preference` agora `pub`.
- `cost_scheduler.rs` — greedy admission por risco com **early-stop por ganho marginal** (`with_early_stop`), campo `expected_accepted`, invariante de **não-antecipação** documentado/testado.
- `sts_calibration.rs` (NOVO) — **Sequential Temperature Scaling** (grid de temperatura por posição, ECE do produto cumulativo, transform logit `σ(logit(p)/T)`).
- `logprob_extractor.rs` — **recompensa contínua Eq 3.1** (`continuous_reward`, vocabulário A–T/1–20) + `two_stage_continuous_reward` (workaround B.6).
- `speculative_drafter.rs` — **`sequential_dependency_pass`** (análogo do sequential head: re-draft condicionado no prefixo aceito). **NÃO LIGADO no CLI ainda.**
- `utils/prompt_optimizer.rs` — prompt de comparação agora pede `score_A/score_B` 1-20.

Python:
- `config.py` — `memory_*`, `curator_repetitions` (K), `voc_stagnation_*`.
- `state.py` — `AuditResult.criteria_scores`, `DualEngineState.voc/voc_stagnated/memory_stats/memory_stable`.
- `engines/curator.py` — **criteria decomposition** (Specification/Output/Errors) no prompt + `criteria_scores`.
- `pipeline/cegar.py` — **repetição K** (`_run_audits`, veredito conservador, dedup de CEs, média de critérios), **VOC** (Spearman iteração×score) + **early-stop por estagnação**.

### 4. Harnesses de A/B (offline, determinísticos, com asserts que quebram o CI)
- Rust: `crates/dspark-core/tests/verification_scaling_test.rs` — E1..E4:
  - E1 PPT soft 67% vs bin 65%, ties 1289→0
  - E2 contínua 100% vs discreta 84%, tie 6%→0
  - E3 STS ECE 0.358→0.055 (−85%)
  - E4 scheduler −22% de calls, melhor falhas/call
- Python: `bench/compare_cegar_improvements.py` — 30 tasks sintéticos:
  - memória+VOC: 189→79 iterações (−58%), paridade de desfecho PASSED
  - K=3: MAE 9.67→3.33 (−66%)
- `tests/test_bench_claims.py` — roda o bench no CI.

### 5. Bench real (APIs reais, ~$0.014 gastos)
- `bench/run_real_bench.py` — PPT agora dual-policy (binário + soft na MESMA chamada) com colunas `T_soft_pick_pass` e Q3.
- `bench/show_results.py` — exibe resultados acumulados.
- Resultado atual: `bench/results/results_1787810974.jsonl` (19 tasks HumanEval 0-18):
  - T bin 94.7% = T soft 94.7%, 0 divergências (tasks fáceis demais — sem sinal)
  - D full 100% (1 escalação, 1 conserto)

## Estado de validação (tudo verde)
- Python: **37 testes** ✓ (`python -m pytest tests -q`)
- Rust: **64 testes** ✓ + clippy limpo (`cargo test -p dspark-core`, `cargo clippy -p dspark-core --all-targets`)

## Pendências para amanhã (priorizadas)
1. **Ligar `sequential_dependency_pass` na fase 1 do CLI** (`main.rs` linha ~353: hoje só `apply_sequential_module`). Fluxo: draft paralelo → PPT → re-draft do vencedor condicionado no prefixo.
2. **Comparativo real com sinal**: rodar bench com `--cheap-model gpt-3.5-turbo` (README mostra 41.7% — drafts falham de verdade) e/ou continuar `--resume` para HumanEval 20+.
3. **STS em produção**: coletar (confiança → aceite) reais para calibrar o ConfidenceHead.
4. **Commit git** do estado atual (opcional, não foi feito).
5. Se quiser: seção de paper com a recorrência da memória KDA + teorema de parada δ→0 + não-antecipação.

## Comandos úteis
```bash
python -m pytest tests -q
cargo test -p dspark-core
cargo clippy -p dspark-core --all-targets
cargo test -p dspark-core --test verification_scaling_test -- --nocapture   # tabela E1-E4
python bench/compare_cegar_improvements.py                                   # tabela B1-B3
python bench/run_real_bench.py --smoke                                       # bench real 3 tasks (~$0.01)
python bench/show_results.py bench\results\results_1787810974.jsonl          # resultados acumulados
```
