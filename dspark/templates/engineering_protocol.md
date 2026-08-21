# DSpark Metacognitive Engineering Protocol

This protocol is natively enforced across the **DSpark** framework, CLI, and Antigravity agents.

When requesting modifications, refactors, feature additions, or bug fixes, all DSpark agents follow this deterministic process:

---

### Mandatory Metacognitive Reasoning Process (Before Code Generation):
1. **Mental Simulation & Static Analysis**: Comprehensive analysis of proposed changes across the entire system.
2. **I/O Contracts & Invariant Formulation**: Explicit definition of Preconditions, Postconditions, and Type guarantees.
3. **Adversarial Simulation**: Proactively synthesize counter-examples and edge cases that could break the code.
4. **Performance & Lifecycle Impact**: Evaluate asymptotic complexity ($O(Time)$, $O(Space)$) and potential regression vectors.

---

### Mandatory Response Structure:

```markdown
### 1. Análise e Raciocínio
- **Onde (Where)**: Exact files, classes, functions, or modules modified.
- **Como (How)**: Technical approach, algorithms, and data structures chosen.
- **Por que (Why)**: Justification of decision and rejection of alternatives.
- **Contrato de I/O e Invariantes**: Pre-conditions, Post-conditions, and Type guarantees.
- **Impacto Específico**: Immediate direct consequences.
- **Impacto Inespecífico / Efeitos Colaterais**: Indirect, long-term architectural effects.

### 2. Testes Mentais e Estáticos Realizados
- Validations, edge cases tested, and simulated adversarial counter-examples resolved.

### 3. Mudanças Propostas (Commented Diff)
```diff
diff --git a/path/to/file.py b/path/to/file.py
--- a/path/to/file.py
+++ b/path/to/file.py
@@ -10,6 +10,12 @@
     existing_code()
+
+    # Rationale: [explanation of change]
+    new_production_code()
+
     remaining_code()
```
