# DSpark

DSpark é um sistema de código **dual-engine**: um modelo **cria**, outro **cura**.

Um LLM que escreve e depois “se revisa” tem viés de confirmação. O DSpark separa o trabalho em dois **papéis** (não fornecedores):

| Papel | Função |
|---|---|
| **Criador** | Rascunha a implementação a partir da spec. Pode ser rápido/barato. |
| **Curador** | **LLM-as-a-Verifier** independente. Pontua contratos de I/O (pré/pós-condições, edge cases), monta contraexemplos e pode reescrever o rascunho. Tem de ser de **outra família** que o criador. |

Você só escolhe *quais* dois modelos ocupam esses papéis. Curação é o fluxo padrão — não se pede.

Par padrão em `~/.dspark/pair.toml`:

```toml
creator = "gpt-4o-mini"
curator = "deepseek-v4-pro"
```

[English](README.md) · Licença: [MIT](LICENSE)

---

## Dois repositórios (não misture)

| Repo | Binário | O que é |
|---|---|---|
| **[CostaJr007/dspark-cli](https://github.com/CostaJr007/dspark-cli)** | `dspark-cli` | **Uso do dia a dia.** TUI no repositório. O criador escreve; depois de cada edição o curador roda sozinho e pode aplicar o refine. |
| **Este repo (`dspark`)** | `dspark` | **Motor.** CLI de pipeline, biblioteca Rust e servidor MCP usados pelo TUI, pelo Agy e por scripts. |

Trabalhar num projeto:

```powershell
cd seu-projeto
dspark-cli
```

Peça a feature. Não diga “cura”. `/pair` só troca os dois modelos.

---

## O que o motor faz

```
especificação (o que o código tem de fazer)
        │
        ▼
   rascunho do criador     ← GPT / Gemini / local / o que você configurar
        │
        ▼
   auditoria do curador    ← DeepSeek (ou outra família)
        │                    veredito: APPROVED | NEEDS_REVISION | REJECTED
        │                    nota 0–100, issues, contraexemplos
        ▼
   refine se precisar      ← o mesmo curador reescreve com a spec + feedback
        │
        ▼
   reauditoria             ← não se entrega no auto-score do criador
```

O curador **não** confia em “os testes passaram” dito pelo criador. Ele checa:

- **Especificação** — o que foi pedido está implementado
- **Contrato de I/O** — vazio/nulo, tipos, erros documentados, exemplos `>>>`, roundtrip encode/decode quando os dois helpers existem
- **Erros** — o contrato não é engolido em silêncio

`APPROVED` 100/100 é proibido se esses exemplos não foram checados.

---

## Instalar este motor

```bash
git clone https://github.com/CostaJr007/dspark.git
cd dspark
cargo install --path . --force
```

Instala **só `dspark`**. Não sobrescreve o `dspark-cli`.

```bash
export OPENAI_API_KEY="..."
export DEEPSEEK_API_KEY="..."     # curador (obrigatório para audit/refine/MCP)
export DSPARK_CURATOR="deepseek-v4-pro"
```

Copie [dspark.toml.example](dspark.toml.example) para `~/.dspark/pair.toml`.

---

## Comandos

```bash
dspark pair
dspark run "LRU limitado em Rust" --lang rust --no-research --out lru.rs
dspark audit lru.rs --spec "get/put O(1), capacidade limitada" --lang rust
dspark refine lru.rs --spec "get/put O(1)" --in-place --lang rust
```

`dspark run` é o pipeline completo. `audit` / `refine` são o curador sozinho.

---

## MCP (Agy e outros agentes)

Ferramentas: `dspark_audit_code`, `dspark_refine_code`, `dspark_arbitrate`.

No Agy deste PC o criador é Gemini Flash; o curador é DeepSeek v4 Pro via esse MCP. Para curador mais barato: `curator = "deepseek-v4-flash"` no `pair.toml`.
