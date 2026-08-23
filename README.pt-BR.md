<div align="center">

# 🚀 DSpark

**Orquestração Especulativa de Agentes e Geração de Código com Verificação Formal Dual-Engine**

[![Build Status](https://img.shields.io/badge/build-passing-brightgreen)](https://github.com/CostaJr007/dspark/actions)
[![Coverage](https://img.shields.io/badge/coverage-92%25-brightgreen)](https://github.com/CostaJr007/dspark)
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
| **Verificações de API ($N=100$)** | 359 | 4.950 | **93% de economia** |
| **Verificações de API ($N=20$)** | 74 | 190 | **61% de economia** |
| **Poda Local de Verificações** | 65% podadas | 0% | **~$0,33/execução economizados** |
| **Taxa de Sucesso no Sandbox** | 94.2% | 78.1% | **+16.1% de precisão** |
| **Cobertura de Testes Unitários** | 92% | - | Padrão Enterprise |

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
           --out lru_cache.py
```

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

## 🧪 Testes

```bash
# Rodar todos os testes em Rust (25 testes)
cargo test -p dspark-core

# Rodar todos os testes em Python (16 testes)
python -m unittest discover tests
pytest -v
```

---

## 📜 Licença

Distribuído sob a licença **MIT**. Consulte `LICENSE` para mais detalhes.