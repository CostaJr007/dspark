"""
Pipeline package orchestrating CEGAR Dual-Engine lifecycle and legacy pipelines.
"""

from .cegar import CEGARPipeline, run_cegar_pipeline
from .legacy import DSparkPipeline, PipelineResult

__all__ = [
    "CEGARPipeline",
    "run_cegar_pipeline",
    "DSparkPipeline",
    "PipelineResult",
]
