---
id: RUST-ADR-0242
alias: ADR-0242
source_sha256: e09d6c093a649e7dc46c349ecfa89d4ea188f30cf15f809a4a6286ce74121660
publication_status: published
publication_date: 2026-10-09
publication_source_revision: f9a496660a4a69f4dd7c7927111767ad5c0c12b2
---
# RUST-ADR-0242 — Separate capability coin-shape and recording-facade ownership

[ADR register](README.md) · [0.3.0 record register](index-0.3.0.md) · [Reference conventions](references-0.3.0.md)

## Publication disposition

This published historical source record belongs to the milestone 0.3.0 documentation collection. **Original opening status:** accepted. Read dated amendments below; neither this status nor issue closure certifies the whole milestone. No fresh implementation, security, proof or formal acceptance is implied. Original code examples and local evidence locators are historical; machine-local paths are not portable downloads or current setup instructions.



## Original decision and amendments

```yaml
status: accepted
date: 2026-10-07
milestone: "0.3.0"
issue: https://github.com/MediaNoxLabs/compact/issues/365
```

## ADR-0242 — Separate capability, coin-shape and recording-facade ownership

### Problem statement

At `939b7aaa`, three emitter files contain 4,098 (`lib.rs`), 4,056 (`stateful.rs`) and 9,451 (`recorded.rs`) lines. More concretely, root assembly owns capability metadata, shared coin IR type construction belongs to native lowering even though recorded lowering uses it, and public recording facade generation sits alongside admission profiles and VM emission. These responsibilities have small, identifiable dependency surfaces and distinct engineering vocabulary.

### Decision and component model

Extract three private, cohesive modules in one mechanical first slice:

| Component | Owns | Does not own |
|---|---|---|
| `capabilities` | Report data and atomic frontend proof-applicability join | Whether a profile is admitted or emitted |
| `coin_shapes` | Exact Compact IR field shapes/order for shielded coin and recipient types | Ledger cryptography, runtime values, coin policy |
| `recorded::facade` | Typed generated recording and observed-call methods, borrowing and argument packaging | Admission, witness execution, ledger instructions |

Keep public capability types/constants available at the crate root. Keep recorded facade entrypoints available through internal reexports. Import shared coin shapes directly from their owner in both native and recorded emitters. Use explicit imports. Co-locate existing relevant tests; retain broader integration tests.

### Before / after examples

```rust
// Before: metadata lives in crate assembly; shared shapes appear native-owned.
use crate::stateful::qualified_coin_type;
// lib.rs declares RustCapabilityReport and its proof metadata join.
// recorded.rs constructs public recording methods beside admission profiles.
```

```rust
// After: each concept has a searchable component owner.
use crate::coin_shapes::qualified_coin_type;
// lib.rs preserves consumer paths:
pub use capabilities::{RustCapabilityReport, RustCircuitCapability,
    RecordingStatus, RUST_CAPABILITY_SCHEMA_VERSION};
// recorded.rs preserves internal call sites:
pub(crate) use facade::{render_recorded_contract_method,
    render_borrowed_recorded_contract_method, render_observed_call_method};
```

Generated consumer code before and after must be byte-identical. A change to displayed/generated API is a separate researched decision, not part of this extraction.

### Emitter and runtime changes

Move existing implementations without changing their AST construction. Native/recorded emitters depend directly on shared IR shape constructors. Runtime/ledger/zk code, runtime ABI 49, private IR schema 20 and capability schema 3 remain unchanged. No new macro, DSL or generated-code level option is introduced.

### Alternatives and tradeoffs

Keeping everything in the large files preserves short-term navigation but hides ownership. A generic `utils` module would hide it again. Rewriting admission during extraction broadens the risk and makes output differences harder to explain. More files are useful only when names and interfaces reduce coupling and clarify responsibility; line counts are diagnostic, not the success criterion.

### Verification plan

Use existing proof-applicability, borrowed-facade hygiene, input-retention, lexical-return and coin-shape integration coverage. Run the emitter package regression suite and strict Clippy; check generated fixture freshness against the updated backend. No expensive proof rerun is required if this is verified to be an exact-output-preserving move. If output changes, investigate before acceptance and broaden checks for the actual cause.

### Scope and delivery

Parent #349 remains open. This first slice does not complete all decomposition, checked domain modeling or safety improvements. Follow-up boundaries (admission/diagnostics, expression lowering, context/invariant ownership) require their own evidence. Nonblocking abstraction-level probes remain #346.

Implementation and local results: pending; append exact commit and receipts after validation.

### Delivery amendment — 2026-10-07

Locally delivered at `537fd64180b9f9f2f7df6fc947944652690f4a27` (GPG verified, DCO, conventional commit; not pushed). 228 backend tests passed (62 library, 13 CLI, 153 rendering), strict package Clippy and formatting passed; frozen updated compiler checked 176 fixtures with 0 stale and 0 failed. Capability invariants passed. Existing implementation bodies, public paths and tests were independently reviewed. Full command logs, file/compiler hashes and limits are in [ADR0242 — Component extraction local receipt](references-0.3.0.md#note-011). No proof suite rerun or remote CI was needed for this output-preserving extraction.

Large-file sizes: lib.rs 4098→3949, recorded.rs 9451→9308, stateful.rs 4056→3984. The useful result is explicit ownership and removal of native-emitter coupling for shared coin shapes; these numbers do not establish broad architectural completion. Parent #349 remains open.
