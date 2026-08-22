# DSpark

Código **dual-engine**: um **criador** rascunha, um **curador** de outra família de modelo audita contratos de I/O. Papéis, não fornecedores.

Par padrão:

| Papel | Modelo |
|---|---|
| criador | `gpt-4o-mini` |
| curador | `deepseek-v4-pro` |

Arquivo: `~/.dspark/pair.toml`. Você escolhe os dois modelos. Curação é o default — não precisa pedir.

[![Licença: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-stable-orange.svg)](https://www.rust-lang.org/)

[English](README.md)

---

## O que rodar

| Comando | Uso |
|---|---|
| **`dspark-cli`** | Dia a dia. TUI no repositório. O criador escreve; o curador roda sozinho depois de cada edição. |
| **`dspark`** | Este crate. Pipeline e biblioteca: `run`, `audit`, `refine`, `arbitrate`, REPL, MCP. |

Fonte do TUI: [CostaJr007/dspark-app](https://github.com/CostaJr007/dspark-app).

```powershell
cd seu-projeto
dspark-cli
```

`/pair` só troca os modelos dos dois papéis.

---

## Este repositório (`dspark`)

```
spec
  → rascunho do criador
  → auditoria de I/O do curador
  → refine se precisar
  → reauditoria
```

```bash
git clone https://github.com/CostaJr007/dspark.git
cd dspark
cargo install --path . --force
```

Instala **só `dspark`**. Não sobrescreve o `dspark-cli`.

```bash
dspark pair
dspark run "LRU cache limitado em Rust" --lang rust --no-research --out lru.rs
dspark audit lru.rs --spec "get/put O(1), capacidade limitada" --lang rust
```

```bash
export OPENAI_API_KEY="..."
export DEEPSEEK_API_KEY="..."
```

Copie [dspark.toml.example](dspark.toml.example) para `~/.dspark/pair.toml`.
