# DSpark

Motor nativo em Rust **dual-engine**: um **criador** rascunha, um **curador** de outra família de modelo audita contratos de I/O. Papéis, não fornecedores.

Par padrão (o que este repo usa com chaves OpenAI + DeepSeek):

- **criador** `gpt-4o-mini`
- **curador** `deepseek-v4-pro`

Configuração em `~/.dspark/pair.toml`. Este projeto não substitui nem altera o `grok.exe`.

[![Licença: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-stable-orange.svg)](https://www.rust-lang.org/)
[![MCP](https://img.shields.io/badge/MCP-Compatible-green.svg)](https://modelcontextprotocol.io/)

[English](README.md)

---

## Binários (não misture)

| Binário | O que é |
|---|---|
| `dspark-cli` | TUI tela cheia (rebrand do Grok Build, Apache-2.0). Instalado à parte. Dual-engine é o fluxo padrão (`/pair`, ferramenta `dspark_curate`). |
| `dspark` | Este crate: CLI do pipeline (`run`, `audit`, `refine`, `arbitrate`, REPL). |
| `dspark-bench` | Runner opcional de HumanEval / HumanEval+. |

`cargo install --path .` **deste** repo instala só `dspark` e `dspark-bench`. **Não** sobrescreve o `dspark-cli`.

---

## Como funciona

```
especificação / tarefa
    → pesquisa na web (opcional)
    → rascunho do criador
    → auditoria de I/O do curador (spec, contratos, oráculo de doctest — não a autoavaliação do agente)
    → refine se precisar
    → reauditoria
```

A busca é ao vivo: Tavily se `TAVILY_API_KEY` existir; senão HTML do DuckDuckGo. Resultado vazio permanece vazio.

---

## Instalação (motor)

```bash
git clone https://github.com/CostaJr007/dspark.git
cd dspark
cargo install --path . --force
```

```bash
dspark --help
dspark pair
```

### Ambiente

```bash
export OPENAI_API_KEY="sk-..."
export DEEPSEEK_API_KEY="sk-..."
export DSPARK_CREATOR="gpt-4o-mini"
export DSPARK_CURATOR="deepseek-v4-pro"
```

PowerShell:

```powershell
$env:OPENAI_API_KEY="sk-..."
$env:DEEPSEEK_API_KEY="sk-..."
```

### Arquivo de par

Copie [dspark.toml.example](dspark.toml.example) para `%USERPROFILE%\.dspark\pair.toml`:

```toml
creator = "gpt-4o-mini"
curator = "deepseek-v4-pro"
research = true
```

`dspark pair` mostra o par ativo e avisa se os dois papéis usam o mesmo modelo.

---

## Comandos

```bash
dspark
dspark run "LeetCode 1 Two Sum em Python" --lang python --no-research --out two_sum.py
dspark audit two_sum.py --spec "Retorne os índices de dois números que somam o alvo" --lang python
dspark refine two_sum.py --spec "Duplicatas; hashmap O(n)" --in-place --lang python
```

---

## TUI (`dspark-cli`)

A UI tela cheia é o uso do dia a dia. O modelo default é o criador; `/pair` grava criador + curador. Depois de editar, o agente é lembrado de chamar `dspark_curate`.

Não faça `cargo install` deste crate por cima de um `dspark-cli.exe` TUI (~300 MB).
