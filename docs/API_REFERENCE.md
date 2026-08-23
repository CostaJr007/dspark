# 🔌 DSpark API Reference

This document provides a comprehensive API reference for both the **Rust Crate (`dspark-core`)** and the **Python SDK (`dspark-ai`)**.

---

## 1. Rust API (`dspark-core`)

### `dspark::engine::SpeculativeDrafter`
```rust
pub struct SpeculativeDrafter;

impl SpeculativeDrafter {
    pub fn new(client: ModelClient, n_trajectories: usize) -> Self;
    pub fn with_resolver(client: ModelClient, n_trajectories: usize, resolver: Box<dyn DependencyResolver>) -> Self;
    pub fn with_model(model_name: &str, n_trajectories: usize) -> Result<Self, ClientError>;
    pub async fn generate_trajectories(&self, prompt: &str) -> Vec<DraftTrajectory>;
    pub fn apply_sequential_module(&self, trajectories: Vec<DraftTrajectory>) -> Vec<DraftTrajectory>;
}
```

### `dspark::engine::PivotTournament`
```rust
pub struct PivotTournament;

impl PivotTournament {
    pub fn new(client: ModelClient, n_pivots: usize) -> Self;
    pub fn with_model(model_name: &str, n_pivots: usize) -> Result<Self, ClientError>;
    pub async fn run_tournament(&self, trajectories: &[DraftTrajectory], criteria: &str) -> TournamentResult;
}
```

### `dspark::engine::ConfidenceHead`
```rust
pub struct ConfidenceHead {
    pub verification_threshold: f64,
}

impl ConfidenceHead {
    pub fn new(verification_threshold: f64) -> Self;
    pub fn estimate_confidence(&self, trajectory: &DraftTrajectory) -> Vec<BlockConfidence>;
}
```

### `dspark::engine::CostScheduler`
```rust
pub struct CostScheduler {
    pub max_api_calls: usize,
    pub cost_per_verification: f64,
}

impl CostScheduler {
    pub fn new(max_api_calls: usize, cost_per_verification: f64) -> Self;
    pub fn schedule_verification(&self, confidences: &[BlockConfidence]) -> VerificationPlan;
}
```

### `dspark::utils::DependencyResolver`
```rust
pub trait DependencyResolver: Send + Sync {
    fn resolve(&self, code_blocks: &[CodeBlock], language: &str) -> (DependencyGraph, bool);
    fn split_into_blocks(&self, source: &str) -> Vec<CodeBlock>;
    fn name(&self) -> &'static str;
}
```

---

## 2. Python SDK API (`dspark-ai`)

### `dspark.pipeline.cegar.CEGARPipeline`
```python
class CEGARPipeline:
    def __init__(
        self,
        creator: CreatorEngine | None = None,
        curator: CuratorEngine | None = None,
        refiner: RefinerEngine | None = None,
        max_iterations: int = 3,
    ) -> None: ...

    async def execute(
        self,
        user_spec: str,
        initial_code: str | None = None,
        language: str = "python",
    ) -> DualEngineState: ...
```

### `dspark.engines.curator.CuratorEngine`
```python
class CuratorEngine:
    def __init__(
        self,
        model: str | None = None,
        temperature: float | None = None,
        sandbox: SandboxRunner | None = None,
    ) -> None: ...

    async def audit_and_verify(
        self,
        source_code: str,
        contracts: list[IOContract],
    ) -> AuditResult: ...
```

### `dspark.sandbox.runner.SandboxRunner`
```python
class SandboxRunner:
    def __init__(self, timeout_seconds: int = 15) -> None: ...

    def run_tests(
        self,
        source_code: str,
        test_code: str,
        extra_files: dict[str, str] | None = None,
    ) -> SandboxExecutionResult: ...
```
