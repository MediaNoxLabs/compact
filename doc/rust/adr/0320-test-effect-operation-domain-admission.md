---
id: RUST-ADR-0320
alias: ADR-0320
source_sha256: 56e2d12522eb80e8fa956e2b56b9ffcb36f0de35dee466980e071822b4c5016c
publication_status: published
publication_date: 2026-10-09
publication_source_revision: f9a496660a4a69f4dd7c7927111767ad5c0c12b2
---
# RUST-ADR-0320 — Test effect-operation domain admission

[ADR register](README.md) · [0.3.0 record register](index-0.3.0.md) · [Reference conventions](references-0.3.0.md)

## Publication disposition

This published historical source record belongs to the milestone 0.3.0 documentation collection. **Original opening status:** delivered bounded test slice · 2026-10-07 · R030-07/#351 · milestone 0.3.0. Read dated amendments below; neither this status nor issue closure certifies the whole milestone. No fresh implementation, security, proof or formal acceptance is implied. Original code examples and local evidence locators are historical; machine-local paths are not portable downloads or current setup instructions.



## Original decision and amendments

## ADR0320 — Test effect-operation domain admission

Status: delivered bounded test slice · 2026-10-07 · R030-07/#351 · milestone 0.3.0

### Problem
The real stateful expression dispatcher lacks focused positive/negative checks for nominal ContractAddress shape, kernel claim digest widths, mint amount width, and input/output coin and recipient types. A valid generated fixture does not by itself establish exact refusal and absence of premature effect emission.

### Before / after
```text
Before: valid effect fixtures; incomplete dispatcher refusal assertions
After: paired valid/refused Self, claim, mint and coin operations
       with exact expected types and no query effect for refused operands
```
Example: KernelSelf requires ContractAddress { bytes: Bytes<32> }; a same-layout differently named record must fail. Mint accepts Uint<64>, not Field or a differently bounded Uint. Input requires qualified coin data; output requires coin data plus the declared recipient sum record.

### Ownership
Add one cohesive effect-domain unit-test module using the existing Declarations dispatcher harness. Check generated method selection for valid cases and absence of effect calls for invalid operands. Operand bindings may precede a later error; do not claim general rollback of emitted temporary statements. Emitter, runtime, generated implementations, IR and ABI remain unchanged. This is emission/admission evidence, not execution, cryptographic or proof assurance. Existing stopped ADR0285 review is not in scope.

### Verification
Focused dispatcher tests, backend library strict Clippy and formatting. Fresh bounded instrumented library cohort, preserve prior executable/profile identities and union only source-identical positive production line hits on the frozen mapping. Exclude new tests explicitly. Numeric coverage alone cannot close #351; separate agents reconcile existing constructor/oracle evidence. Conventional signed DCO commit, local only.

Delivery issue: https://github.com/MediaNoxLabs/compact/issues/445


### 2026-10-07 — Effect domain admission delivered (ADR0320/#445)

Signed GPG/DCO `49898aeda5c694fcfbd249bb84162340977b1b26` adds four real-dispatcher tests for nominal ContractAddress shape, claim digest width/routing, exact mint Uint64 amount, qualified input coins and output recipients. Accepted and rejected cases check types, exact diagnostics, method routing and absence of premature context calls; invalid first operands win before unresolved later operands. This is emitted AST admission evidence, not new ledger/proof execution. Production source and generated libraries remain unchanged.

24 focused dispatcher and201 fresh instrumented backend library methods pass, as do strict library/test/all-feature Clippy and whole-workspace formatting. Independent subagent review found no material findings. Source-identical Boolean hit union adds4changed lines: **8602/9060=94.94%**, whole mapped backend22156/24020=92.24%. The95% floor remains unmet by5hits; no rounding-up acceptance. Tests are excluded and the frozen production denominator is unchanged. Other owner percentages retain earlier cohort scopes.

Archive [ADR0320 — Effect domain tests and coverage.zip](references-0.3.0.md#note-102), SHA256 `5710684f3b522877d8fd67ab629b993e7593790d0244bec3e62e80fb29b89e10`. #445 completes this slice; #351 and semantic/evidence obligations remain open; milestone remains9/19. Source/receipt/compiled-object/profiles preserved. No push/remoteCI. User doc preserved.
