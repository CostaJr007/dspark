<div align="center">

# 🚀 DSpark

**Orquestração Especulativa de Agentes e Geração de Código com Verificação Formal Dual-Engine**

[![Build Status](https://img.shields.io/badge/build-passing-brightgreen)](https://github.com/CostaJr007/dspark/actions)
[![Tests](https://img.shields.io/badge/testes-37%20Rust%20%2B%2016%20Python-brightgreen)](https://github.com/CostaJr007/dspark/actions)
[![Piloto Real](https://img.shields.io/badge/piloto_real-91%2C1%25%20pass%401%20%240%2C108-blue)](#-piloto-real--modelos-reais-gasto-real)
[![License](https://img.shields.io/badge/license-MIT-blue)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.75%2B-orange)](https://rust-lang.org)
[![Python](https://img.shields.io/badge/python-3.10%2B-blue)](https://python.org)

[📚 Documentação](docs/) • [🚀 Início Rápido](#-início-rápido) • [📊 Benchmarks](#-benchmarks) • [🤝 Contribuição](docs/CONTRIBUTING.md) • [🇺🇸 English](README.md)

</div>

---

## 🎯 O que é o DSpark?

O **DSpark** é uma plataforma de engenharia de software com IA de nível enterprise que implementa **Orquestração Especulativa de Agentes**, combinando:

- 🧠 **Arquitetura Dual-Engine (CEGAR)**: Isola epistemologicamente a geração de código (Criador) da verificação adversarial (Curador) para superar a *Falácia da Auto-Correção* (Huang et al., 2023).
- ⚡ **Decodificação Especulativa Semi-Autorregressiva**: Gera $N$ trajetórias de código em paralelo com controle de concorrência e validação sintática via AST.
- 🔍 **Torneio Probabilístico de Pivôs (PPT)**: Avalia e ranqueia trajetórias com complexidade $O(Nk)$ em vez do custo quadrático $O(N^2)$ de comparações par-a-par.
- 📊 **Pruning Agendado por Confiança**: Calcula a entropia ciclomática local em CPU para podar 40–70% de chamadas de API desnecessárias.
- 🔌 **Servidor FastMCP Nativo**: Integração direta via protocolo MCP com IDEs modernas (Cursor, Windsurf, Claude Code, Roo Code, Antigravity).

> **Fundamentação Teórica**: [DSpark (DeepSeek & Peking University, 2026)](https://arxiv.org/abs/2607.05147) e [LLM-as-a-Verifier (Kwok et al., 2026)](https://arxiv.org/abs/2607.05391).

### 🏆 Resultados Chave e Benchmarks

| Métrica | DSpark Especulativo | Round-Robin Tradicional | Ganho |
|---|---|---|---|
| **Comparações de API ($N=100$, $k=3$)** | 394 | 4.950 | **92,0% de economia** ✅ reproduzível |
| **Comparações de API ($N=50$, $k=3$)** | 194 | 1.225 | **84,2% de economia** ✅ reproduzível |
| **Comparações de API ($N=20$, $k=3$)** | 74 | 190 | **61,1% de economia** ✅ reproduzível |
| **Poda Local de Verificações** | 60–98% podadas (com teto de orçamento) | 0% | teto rígido de custo ✅ reproduzível |
| **Taxa de Sucesso (tiered vs flagship-only)** | simulada: 96,0% vs 90,5% a ~58% do custo † | — | hipótese, valide no seu workload |
| **Cobertura de Testes** | 37 testes Rust + 16 Python verdes no CI | - | ✅ |

✅ = assegurado no CI por `tests/tournament_scaling_test.rs` e
`tests/pruning_reproducibility_test.rs` (metodologia: [docs/BENCHMARKS.md](docs/BENCHMARKS.md)).
† = saída da simulação de premissas declaradas em `examples/cost_quality_harness.rs`;
não é um resultado medido de acurácia de modelo — não cite como tal.

> O PPT compensa para **N ≥ 10**; abaixo disso o overhead do ring-pass supera o all-pairs.

### 🔬 Piloto Real — Modelos Reais, Gasto Real

Uma execução ponta a ponta do pipeline tiered completo contra **APIs reais**
(56 tarefas: 50 HumanEval + 6 de criação de código aberta, avaliadas em
sandbox, 22/08/2026, gasto total **US$ 0,108**):

| Configuração | pass@1 | Notas |
|---|---|---|
| Só modelo barato (`gpt-4o-mini`) | 89,3% | baseline B |
| Só flagship (`deepseek-v4-flash`) | 83,9% | baseline A |
| Melhor de 3 ao acaso | 89,3% | contrafactual dos mesmos rascunhos |
| Verificar todos (grátis) | 89,3% | só avaliação local em sandbox |
| **Escolha do PPT (sem escalada)** | 89,3% | seleção por torneio |
| **Tiered completo (PPT + escalada)** | **91,1%** | flagship refina os casos difíceis |

Leitura honesta deste piloto:

- **P1 — o torneio agrega qualidade?** Aqui: **+0,0 pts**. Em 0 de 56 tarefas
  os três rascunhos divergiram (passavam todos juntos ou falhavam todos
  juntos), então não havia nada para o torneio separar. Isso mede o *regime
  do benchmark*, não um defeito do PPT.
- **P2 — a escalada ao flagship agrega?** **+1,8 pts**: a política de escalada
  mirou exatamente as 6 tarefas que falhavam (precisão de 100%) e corrigiu 1.
  Direção positiva; n=56 é pequeno demais para significância estatística.
- As vitórias estruturais (contagem de comparações, poda, tetos de custo) são
  onde a economia do DSpark já está provada.

Reproduza: `python bench/run_real_bench.py` (requer `OPENAI_API_KEY` e
`DEEPSEEK_API_KEY`; logs JSONL por tarefa em `bench/results/`).

---

## 🚀 Início Rápido

### Pré-requisitos
- **Rust**: 1.75+ ([instalação](https://rustup.rs/))
- **Python**: 3.10+ (para o SDK Python e o sandbox runner)
- **Chaves de API**: DeepSeek API Key, OpenAI API Key ou Google Gemini API Key

### Instalação

```bash
# Clone o repositório
git clone https://github.com/CostaJr007/dspark.git
cd dspark

# Instalar o CLI em Rust (backend Regex AST padrão)
cargo install --path crates/dspark-core --force

# OU instalar com suporte a Tree-Sitter AST
cargo install --path crates/dspark-core --features tree-sitter-ast --force

# Instalar o SDK Python
pip install -e .
```

### Configuração

```bash
# Configurar variáveis de ambiente
export DEEPSEEK_API_KEY="sua-chave-deepseek"
export OPENAI_API_KEY="sua-chave-openai"
export GEMINI_API_KEY="sua-chave-gemini"

# Configurar par Criador/Curador
dspark pair --creator gpt-4o-mini --curator deepseek-chat
```

### Execução em Modo Especulativo

```bash
# Executar orquestração especulativa com N=4 trajetórias e k=2 pivôs
dspark run "Implemente um LRU Cache thread-safe com expiração TTL em Python" \
           --speculative \
           --trajectories 4 \
           --pivots 2 \
           --ranking-model deepseek-chat \
           --out lru_cache.py
```

Roteamento em camadas: a **camada barata** redige as trajetórias E roda as
comparações do torneio (`--ranking-model`, por padrão o modelo creator); o
`--curator` flagship só é acionado quando a política de escalonamento detecta
um caso residual difícil (empate no torneio, blocos de alto risco não
verificados, baixa confiança do vencedor, AST inválida).

### Servidor FastMCP

```bash
# Iniciar servidor MCP para Cursor / Claude Code
dspark-mcp
```

---

## 📖 Índice da Documentação

| Documento | Descrição |
|---|---|
| [🏛️ Arquitetura](docs/ARCHITECTURE.md) | Especificação técnica do pipeline de 5 estágios |
| [🚀 Primeiros Passos](docs/GETTING_STARTED.md) | Guia detalhado de instalação, configuração e uso |
| [🔌 Referência de API](docs/API_REFERENCE.md) | Documentação completa das APIs Rust e Python |
| [⌨️ Referência da CLI](docs/CLI_REFERENCE.md) | Lista de comandos, opções e variáveis de ambiente |
| [📊 Benchmarks](docs/BENCHMARKS.md) | Metodologia e resultados de benchmarks com Criterion |
| [🎓 Teoria](docs/THEORY.md) | Fundamentos teóricos (CEGAR, DSpark, LLM-as-a-Verifier) |
| [🤝 Contribuição](docs/CONTRIBUTING.md) | Guia para desenvolvedores e padrões de código |
| [📜 Histórico de Versões](docs/CHANGELOG.md) | Registro de lançamentos e notas de versão |

---

## 📊 Benchmarks

Suite Criterion completa:

```bash
./scripts/bench_all.sh
```

### Escalonamento do Torneio ($k=3$ pivôs)
```
N=10:  O(Nk)=34 comparações  vs all-pairs=45   (24,4% de economia; PPT só vale a partir de N>=10)
N=20:  O(Nk)=74 comparações  vs all-pairs=190  (61,1% de economia)
N=50:  O(Nk)=194 comparações vs all-pairs=1225 (84,2% de economia)
N=100: O(Nk)=394 comparações vs all-pairs=4950 (92,0% de economia)
```

### Piloto Real (56 tarefas, US$ 0,108)
```
só barato gpt-4o-mini         : 89,3% pass@1
só flagship deepseek-v4-flash : 83,9%
tiered (PPT + escalada)       : 91,1%   (+1,8 pts vindos da escalada)
torneio vs acaso              : +0,0    (rascunhos perfeitamente correlacionados neste regime)
precisão da escalada          : 6/6 casos-alvo realmente falhos, 1 corrigido
```

Metodologia completa e instruções de reprodução:
[docs/BENCHMARKS.md](docs/BENCHMARKS.md).

---

## 🧪 Testes

```bash
# Rodar todos os testes em Rust (37 testes)
cargo test -p dspark-core

# Rodar todos os testes em Python (16 testes)
python -m unittest discover tests
pytest -v
```

---

## 📜 Licença

Distribuído sob a licença **MIT**. Consulte `LICENSE` para mais detalhes.