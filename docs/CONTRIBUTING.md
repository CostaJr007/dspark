# 🤝 Contributing to DSpark

We welcome contributions from the community! Follow these steps to set up your development environment.

---

## Development Workflow

1. **Fork and Clone**:
   ```bash
   git clone https://github.com/CostaJr007/dspark.git
   cd dspark
   ```

2. **Verify Tests**:
   ```bash
   # Rust core tests
   cargo test -p dspark-core

   # Python SDK tests
   python -m unittest discover tests
   ```

3. **Check Linter Rules**:
   ```bash
   cargo clippy -p dspark-core
   ```

4. **Commit Messages**:
   Follow [Conventional Commits](https://www.conventionalcommits.org/):
   - `feat(...)`: New features
   - `fix(...)`: Bug fixes
   - `docs(...)`: Documentation updates
   - `bench(...)`: Performance benchmarks
