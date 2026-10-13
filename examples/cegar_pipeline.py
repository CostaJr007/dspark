"""
Example: Programmatic execution of the CEGAR verification pipeline via Python SDK.
"""

import asyncio
from dspark.pipeline.cegar import CEGARPipeline
from dspark.engines.creator import CreatorEngine
from dspark.engines.curator import CuratorEngine
from dspark.engines.refiner import RefinerEngine


async def main():
    print("=== DSpark CEGAR Pipeline Example ===")

    # Initialize engines with configured models
    pipeline = CEGARPipeline(
        creator=CreatorEngine(model="gpt-4o-mini"),
        curator=CuratorEngine(model="deepseek-chat"),
        refiner=RefinerEngine(model="deepseek-chat"),
        max_iterations=2,
    )

    spec = "Write a function parse_int(s: str) -> int that handles negative numbers and raises ValueError for non-digits."
    final_state = await pipeline.execute(user_spec=spec, language="python")

    print(f"Final Verdict: {final_state.verdict}")
    print(f"Iterations: {final_state.iteration}")
    print("\nVerified Code:\n", final_state.current_draft)


if __name__ == "__main__":
    asyncio.run(main())
