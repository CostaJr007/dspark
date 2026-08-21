# DSpark CLI

CLI nativa em Rust para trabalho **dual-engine**: um **criador** rascunha, um **curador** de outra família de modelo audita contratos de I/O. O binário se chama `dspark-cli`.

Criador e curador são **papéis**, não fornecedores. Os dois são escolhíveis. O par típico é um rascunho classe Gemini com um curador classe DeepSeek, para o revisor não ser da mesma família que escreveu o código (autoavaliação do mesmo modelo tem viés de confirmação).

Configuração em `~/.dspark/config.toml`. Este projeto não substitui nem altera o `grok` da sua máquina.

[![Licença: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-stable-orange.svg)](https://www.rust-lang.org/)
[![MCP](https://img.shields.io/badge/MCP-Compatible-green.svg)](https://modelcontextprotocol.io/)

[English](README.md)

---

## Como funciona

```
especificação / tarefa
    → pesquisa na web (opcional)
    → rascunho do criador
    → auditoria de I/O do curador (spec, contratos, erros — não a autoavaliação do agente)
    → refine se precisar
    → reauditoria
```

O curador é LLM-as-a-Verifier: pontua pré-condições, pós-condições, edge cases, complexidade e segurança de recurso. Não deve confiar na nota que o criador dá a si mesmo. Depois do refine, audita de novo.

A busca é ao vivo: Tavily se `TAVILY_API_KEY` existir; senão HTML do DuckDuckGo. Resultado vazio permanece vazio — sem hits inventados.

---

## Instalação

```bash
git clone https://github.com/CostaJr007/dspark.git
cd dspark
cargo install --path .
```

Isso instala `dspark-cli` (em geral `%USERPROFILE%\.cargo\bin` no Windows, `~/.cargo/bin` nos demais).

```bash
dspark-cli --help
dspark-cli pair
```

### Ambiente

```bash
export DEEPSEEK_API_KEY="sk-..."
export GEMINI_API_KEY="..."
export OPENAI_API_KEY="..."
export TAVILY_API_KEY="..."
export DSPARK_CREATOR="gemini-3.7-flash"
export DSPARK_CURATOR="deepseek-v4-pro"
```

PowerShell:

```powershell
$env:DEEPSEEK_API_KEY="sk-..."
```

### Par criador / curador

Copie [dspark.toml.example](dspark.toml.example) para `%USERPROFILE%\.dspark\config.toml` (ou `~/.dspark/config.toml`):

```toml
creator = "gemini-3.7-flash"
curator = "deepseek-v4-pro"
research = true
```

`dspark-cli pair` mostra o par ativo e avisa se os dois papéis usam o mesmo modelo.

---

## Comandos

Sem subcomando, abre o REPL com o par salvo:

```bash
dspark-cli
dspark-cli interactive --generator gemini-3.7-flash --curator deepseek-v4-pro --theme bloomberg
```

Utilitários:

```bash
dspark-cli pair
dspark-cli search "tokio spawn_blocking vs spawn"
dspark-cli search --deep "tarefas em background FastAPI"
dspark-cli fetch https://docs.rs/tokio
dspark-cli audit src/search.rs --spec "Busca binária, O(log N), lista vazia é válida"
dspark-cli refine src/lru.rs --spec "get/put O(1), capacidade limitada" --in-place
dspark-cli arbitrate a.rs b.rs --spec "Fila lock-free"
dspark-cli run "LRU cache limitado em Rust" --lang rust --out lru.rs
dspark-cli local
dspark-cli "Refatore auth.rs para bcrypt e cubra senha vazia"
```

Comandos do REPL: `/search`, `/fetch`, `/files`, `/read`, `/sh`, `/audit`, `/refine`, `/local`, `/models`, `/theme bloomberg|cyan|matrix`.

---

## MCP

```json
{
  "mcpServers": {
    "dspark": {
      "command": "dspark-cli",
      "args": ["mcp"],
      "env": {
        "DEEPSEEK_API_KEY": "sua-chave"
      }
    }
  }
}
```

Ferramentas: `dspark_audit_code`, `dspark_refine_code`, `dspark_arbitrate`.

---

## Biblioteca Rust

```rust
use dspark::DeepSeekCurator;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let curator = DeepSeekCurator::new()?;
    let verdict = curator
        .audit(
            "fn divide_chunks(l: &[u8], n: usize) {}",
            "Particionar em lotes de tamanho n. Tratar n == 0.",
            Some("rust"),
        )
        .await?;
    println!("Veredito: {} (Score: {}/100)", verdict.verdict, verdict.score);
    Ok(())
}
```

---

## SDK Python (opcional)

O pacote `dspark/` é SDK de compatibilidade (`pip install -e .`). Ele **não** instala o comando `dspark`, para não sombrear o `dspark-cli`. Use `python -m dspark.cli` só se quiser o entrypoint legado.

---

## Licença

MIT — [Adeilson Costa](https://github.com/CostaJr007).
