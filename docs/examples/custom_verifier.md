# 🔍 Example: Custom Verifiers & Local LLMs

DSpark allows configuring local open-weights LLMs (Ollama / vLLM / llama.cpp) as Creators or Curators.

---

## 1. Connecting to Local Ollama Instance

Configure DSpark to use a local Qwen or DeepSeek model running at `http://127.0.0.1:11434/v1`:

```bash
dspark pair --creator local:qwen2.5-coder:7b --curator deepseek-chat
```

---

## 2. Programmatic Python Configuration

```python
import asyncio
from dspark.pipeline.cegar import CEGARPipeline
from dspark.engines.creator import CreatorEngine
from dspark.engines.curator import CuratorEngine

pipeline = CEGARPipeline(
    creator=CreatorEngine(model="ollama/qwen2.5-coder:7b"),
    curator=CuratorEngine(model="deepseek/deepseek-chat"),
    max_iterations=3,
)

asyncio.run(pipeline.execute("Implement quicksort with 3-way partitioning"))
```
