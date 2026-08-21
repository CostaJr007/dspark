# DSpark

> **Motor de Arbitragem Especulativa Dual-LLM** (Rust)
> *Geração de alto desempenho (Gemini) + curadoria e raciocínio de I/O profundo (DeepSeek)*

[![Licença: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-stable-orange.svg)](https://www.rust-lang.org/)
[![MCP Ready](https://img.shields.io/badge/MCP-Compatible-green.svg)](https://modelcontextprotocol.io/)
[![Google Antigravity](https://img.shields.io/badge/Google_Antigravity-Integrado-4285F4.svg)](https://antigravity.google)

---

## Visão geral

O **DSpark** é um framework agentic inspirado em *Speculative Decoding* e no padrão *Generator-Critic*. O runtime, a CLI, o servidor MCP, o pipeline, o motor de busca e o curador estão em **Rust**. O pacote Python permanece só como SDK opcional.

* **Geradores (Gemini / GPT / modelos locais):** velocidade, contexto grande e edição de vários arquivos.
* **Curadores (DeepSeek Reasoner / V4 Pro):** raciocínio profundo, contratos de I/O e auditoria de *edge cases*.

O DSpark junta os dois: o gerador produz o rascunho rápido; o DeepSeek atua como arquiteto-chefe, valida contratos e só então o código segue.

---

## Instalação

```bash
git clone https://github.com/CostaJr007/dspark.git
cd dspark
cargo install --path .
```

```bash
export DEEPSEEK_API_KEY="sua-chave-deepseek"
```

No PowerShell:

```powershell
$env:DEEPSEEK_API_KEY="sua-chave-deepseek"
```

---

## Uso da CLI

```bash
dspark
dspark search "FastAPI background tasks best practices"
dspark audit src/busca.rs --spec "Busca binária O(log N) tratando listas vazias"
dspark refine src/algoritmo.rs --spec "Otimizar para O(1) de memória auxiliar" --in-place
dspark arbitrate candidato_a.rs candidato_b.rs --spec "Fila concorrente sem lock"
dspark run "Implementar LRU cache limitado em Rust" --lang rust --out lru.rs
dspark local
dspark "Refatore auth.rs para usar bcrypt e valide todos os edge cases"
```

Comandos da sessão interativa: `/search`, `/fetch`, `/files`, `/read`, `/sh`, `/audit`, `/refine`, `/local`, `/models`, `/theme`.

---

## Servidor MCP

```json
{
  "mcpServers": {
    "dspark": {
      "command": "dspark",
      "args": ["mcp"],
      "env": { "DEEPSEEK_API_KEY": "sua-chave" }
    }
  }
}
```

---

## SDK Python (opcional)

O CLI principal é o binário Rust. O pacote Python (`pip install -e .`) não instala mais o comando `dspark`, para não sombrear o binário. Use `python -m dspark.cli` só se quiser a CLI legado.

---

## Licença

MIT — criado por [Adeilson Costa](https://github.com/CostaJr007).
