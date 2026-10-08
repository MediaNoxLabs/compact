# Snippet and evidence matrix

**ADR0324/#449 external tutorial validation. SDK staging, two fresh generations and three focused external tests passed. No new rustdoc, proof or network run is claimed by this receipt.** Current local compatibility policy is accepted at `5efa91c280c059e375dcde0998c64dd88f615fb6`; see [candidate qualification](candidate-qualification.md) for the current overall status. Files and source identities are in [guide-manifest.json](guide-manifest.json) and [historical reference manifest](../evidence/0.3.0/tutorials-adr0324/reference-manifest.json).

| ID | Material | Classification now | Validation required |
|---|---|---|---|
| W1 | `examples/witnessed-cell/witnesses_oracle.compact` | Byte-identical maintained source; freshly generated | Source hash retained in external receipt |
| W2 | `examples/witnessed-cell/{Cargo.toml,tests/witnessed.rs}` | One focused external test passed | Qualified lock, unique runtime/path isolation, formatting and strict Clippy recorded |
| D1 | `examples/did-lifecycle/source/**` and `data/unit-composition.json` | Byte-identical source/independent capture; freshly generated | Source/capture hashes verified in receipt |
| D2 | `examples/did-lifecycle/{Cargo.toml,tests/lifecycle.rs,support/*.rs}` | Both external lifecycle tests passed | Qualified lock, unchanged source generation, path isolation, formatting and strict Clippy recorded |
| P1 | [Complete passport consumer](examples/passport/README.md) | Historical executed two-test source copied unchanged; generated dependencies must be prepared | Reuse only after actual file/lock/source identity comparison; no fresh result claimed |
| C1 | Existing vault Counter ContractLab consumer | Historical executed external application sharing absolute source checkout | Do not describe it as all-packages self-contained; new staged SDK layout needs its own receipt |
| K1 | Existing capability checker | Historical executed available digest / unavailable relation rows | Keep historical unavailable example labeled; current DID relation is available. Recheck the current report if promoted as current |
| M1 | ADR0293/0304 migration controls | Historical executed ABI49/non-exhaustive refusal and ABI50/migrated positive controls | Link exact receipts and new accepted compatibility policy; no new runs implied |
| S1 | `prepare-sdk.py` and tutorial shell fences | SDK copying executed; shell recipes updated to qualified source-lock seeding | Local package paths and runtime identity verified |
| R1 | Witness method signature and proof-stage fences | Explanatory excerpts with named prerequisites | No “copy and run” claim; compare names/parameters with current generated/runtime API |
| RD1 | Public `WitnessScript` rustdoc | Executed successfully in ADR0326 at signed `9012a86a` | Owned clone independence, matching consumption and mismatch preservation; separate receipt from tutorial validation |
| CF1 | Documented static constraints / negative consumers | Mapped in the companion static-constraint guide | Historical actual migration failures are distinct from maintained controls not freshly run; no dedicated byte-length/witness-signature negative receipt claimed |

## Companion correction history (completed 2026-10-07)

All corrections below are now applied to companion guides. Prior pages are preserved in the before-reconciliation archive; historical measured receipts remain unchanged.

- Remove ACC deferred/future-adoption language from current Start/Jubjub guidance; ACC is removed, generic Jubjub remains, ledger8 time issue #442 is independent.
- Fix passport troubleshooting's current runtime0.1.0 label to0.2.0. Keep default generated app/testkit0.1.0 separate.
- Convert version-selection memo/history into the current policy; preserve ADR0261/0293 historical values only in explicitly dated sections.
- Current capability guide should state all12 DID recorded APIs available. The old unavailable relation row can remain only as a dated diagnostic example.
- The [ownership map](ownership.md) identifies the delivered component owners and accepted model disposition.
- ContractLab “DID recording in delivery” is stale. Keep existing accurate gas/error/rollback explanations.
- The current runtime ABI is 50; earlier ABI additions remain historical. Use [the task index](index.md) for current runtime, backend and testkit workflows.

## Validation scope and remaining requirements

W2/D2 generation, focused external tests and metadata checks ran with an owned warm target and the reviewed source lock. No need to repeat unchanged full proof suites for a documentation packaging adaptation. Record test/toolchain/features/host and explicitly retain which independent oracles are compared: D2 checks actual TS capture; W2 compares native/recorded behavior while its original maintained scenario owns separate TS evidence.

The external tutorial results cover their recorded examples. Separate rustdoc and static-constraint evidence has its own limits; see [candidate qualification](candidate-qualification.md) for the combined validation and documentation status.

## ADR0360 local proof fixture

`local-proof-tests.md` contains an API excerpt and a repository test command, not a new standalone consumer tutorial. The complete `ProofLab` rustdoc example compiled at17433b40; its prove path was not executed. Four new maintained tests exercise actual official-provider checks/refusals; no successful proof receipt is claimed. Existing generated ContractLab scenarios passed with proof enabled and with default features.
