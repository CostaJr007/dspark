# DSpark: Plataforma Unificada de Código com Dual-Engine

[![Rust](https://img.shields.io/badge/rust-2024%20%2F%202021-orange.svg)](https://www.rust-lang.org)
[![Python](https://img.shields.io/badge/python-3.10%2B-blue.svg)](https://www.python.org)
[![MCP](https://img.shields.io/badge/protocol-MCP-purple.svg)](https://modelcontextprotocol.io)
[![License: MIT / Apache 2.0](https://img.shields.io/badge/license-MIT%20%2F%20Apache--2.0-green.svg)](LICENSE)

O **DSpark** é uma arquitetura unificada de IA para geração e verificação de código. Ele integra a alta taxa de transferência e geração criativa (**Creator**) com a arbitragem e verificação rigorosa e independente de contratos de I/O (**Curator / LLM-as-a-Verifier**).

Sistemas baseados em um único LLM que geram e revisam o próprio código sofrem do **viés de auto-correção** (*Self-Correction Fallacy*). O DSpark elimina esse problema separando as tarefas entre famílias distintas de modelos com refinamento guiado por contra-exemplos (estilo CEGAR).

---

## 🔬 Fundamentação Teórica e Científica

### 1. A Falácia da Autocorreção (*Self-Correction Fallacy*)
Pesquisas acadêmicas demonstraram formalmente que **LLMs não conseguem se autocorrigir de forma confiável no mesmo contexto autoregressivo** (*"Large Language Models Cannot Self-Correct Reasoning Yet"*, Huang et al., 2023; Stechly et al., 2024).
* Quando o mesmo modelo gera um código e é perguntado *"Tem certeza de que está certo?"*, ele sofre de **viés de confirmação**. O modelo tende a justificar as próprias alucinações e reafirmar premissas errôneas porque a geração e a revisão compartilham o mesmo espaço latente e a mesma distribuição de probabilidade de tokens.

### 2. Diversidade de Famílias e *Priors* Indutivos
O DSpark quebra essa câmara de eco separando o processo entre duas famílias de modelos independentes:
* **Criador (ex.: Google Gemini / Anthropic Claude):** Otimizado para contexto amplo de repositório, alta vazão e modificações rápidas de arquivos.
* **Curador (ex.: DeepSeek v4 Pro / DeepSeek-R1):** Otimizado para raciocínio lógico profundo (*Chain-of-Thought*) e falsificação rigorosa.
* O que passa despercebido pelos pontos cegos da Família A é imediatamente capturado e confrontado pela Família B.

### 3. Princípio CEGAR e Contratos Formais de I/O
Em vez de pedir "revisões genéricas", o DSpark adota a metodologia de verificação formal **CEGAR** (*Counterexample-Guided Abstraction Refinement*):
* O Curador audita o código contra **contratos estritos de Entrada/Saída (I/O)**, invariantes e condições de limite.
* Se um contrato falhar, o Curador sintetiza um **contra-exemplo concreto** (`counter_examples`) que guia a refatoração determinística do código.

---

## 🌟 Vantagens Práticas para o Desenvolvedor

| Vantagem | Modelo Único Tradicional | DSpark Dual-Engine |
|---|---|---|
| **Detecção de Casos de Borda** | ❌ Deixa passar erros sutis de limites e concorrência | ✅ **Detectado deterministicamente** pelo Curador independente |
| **Fim dos Loops de Alucinação** | ❌ Fica gerando o mesmo erro repetidamente | ✅ **Quebrado na hora** com contra-exemplos concretos |
| **Economia de Custos e Tokens** | ❌ Usa modelos caros de raciocínio para tarefas simples de IO | ✅ **Criador rápido/barato** + **Curador de raciocínio cirúrgico** |
| **Autonomia sem Fadiga** | ❌ Exige que o desenvolvedor revise cada linha manualmente | ✅ **Curadoria 100% automática** via hooks MCP em segundo plano |
| **Independência de Fornecedor** | ❌ Preso ao ecossistema de uma única empresa | ✅ **Universal**: Google, Anthropic, DeepSeek, OpenAI e Modelos Locais |

---

## 🏛️ Arquitetura do Sistema

```mermaid
graph TD
    User([Desenvolvedor / IDE / Agente]) -->|Prompt / Especificação| Creator[Creator: Gemini 3.7 / LLM Rápido]
    Creator -->|Código Inicial & AST| Engine[Motor de Arbitragem DSpark]
    Engine -->|Código + Contrato I/O| Curator[Curator: DeepSeek v4 Pro / Verificador]
    Curator -->|Auditoria / Contra-exemplos| Evaluator{Veredito: APPROVED?}
    Evaluator -->|Sim| Output([Código Verificado e Aprovado])
    Evaluator -->|Não: critical_issues| Refiner[Refinador DSpark]
    Refiner -->|Código Refinado| Engine
```

### Papéis do Dual-Engine

| Papel | Motor Principal | Finalidade | Objetivo |
|---|---|---|---|
| **Creator** | **Gemini 3.7 Flash** | Escreve implementações, lê contextos amplos de repositórios e edita arquivos. | Maximizar vazão, contexto e velocidade. |
| **Curator** | **DeepSeek v4 Pro** | LLM-as-a-Verifier independente. Audita pré-condições, pós-condições e casos de borda. | Falsear erros, sintetizar contra-exemplos e validar contratos de I/O. |

---

## 📦 Estrutura do Monorepo Unificado

```text
dspark/
├── crates/
│   ├── dspark-core/          # Núcleo Rust, CLI, REPL e Servidor MCP Verifier
│   ├── codegen/              # Crates da TUI em tela cheia (PTY, worktrees, chat-state, etc.)
│   │   ├── xai-grok-pager-bin/   # Binário executável da TUI (dspark-cli)
│   │   ├── xai-codebase-graph/   # Gerador de grafo de código via tree-sitter
│   │   ├── xai-fast-worktree/    # Virtualização de worktrees Git com CoW
│   │   └── ...                   # Crates modulares do ecossistema
│   └── common/               # Crates compartilhados (tracing, circuit breaker, tool-runtime)
├── dspark/                   # Pacote Python SDK (pipelines assíncronos, geradores, curadores)
├── skills/                   # Skills do Antigravity e hooks (dspark-curate, automações)
├── examples/                 # Exemplos em Rust e Python (arbitragem, LeetCode, contratos)
├── tests/                    # Suítes de testes em Rust e Python
├── pyproject.toml            # Configuração do pacote Python
├── Cargo.toml                # Definição do Workspace raiz Cargo
└── README.md                 # Documentação técnica do projeto
```

---

## 🚀 Componentes Principais

1. **TUI Fullscreen (`dspark-cli`):** Interface de terminal completa com curadoria automática contínua em segundo plano.
2. **Servidor MCP (`dspark mcp`):** Expõe ferramentas padronizadas (`dspark_audit_code`, `dspark_refine_code`) para Antigravity, Cursor, Claude Code, Windsurf e Roo Code.
3. **CLI & Engine Core (`dspark-core`):** Executável autônomo para auditoria rápida, REPL interativo e arbitragem.
4. **Python SDK (`dspark`):** Biblioteca Python para integração direta em pipelines e fluxos de dados.

---

## 🧪 Testes e Validação

```bash
# Testes do núcleo Rust
cargo test -p dspark-core

# Testes do pacote Python
python -m unittest discover tests

# Validação dos crates do workspace
cargo check -p dspark-core -p xai-grok-tools -p xai-grok-pager-bin
```