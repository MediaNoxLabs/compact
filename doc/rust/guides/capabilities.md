# Read Rust capability reports

Evidence checkpoint: 2026-10-07. All twelve adopted DID stateful exports now advertise recorded and observed-call APIs. A fresh report generated from the unchanged DID source with the reviewed5efa91c2 toolchain confirms all twelve as `available`. The earlier unavailable relation example below is explicitly historical; [Capability guide execution receipt — 2026-10-07](../evidence/0.3.0/capability-k1/receipt.json) retains that older report and checker execution.

## Start with the generated report

Generate the crate with the compiler and matching runtime source selected for your project:

```sh
compactc --target rust --skip-zk \
  --rust-runtime-root /absolute/path/to/compact \
  contract.compact generated
```

Read `generated/contract/rust-capabilities.json` alongside `generated/compiler/contract-info.json`. `--skip-zk` skips key generation; it does not mean a circuit no longer requires a proof. The frontend's contract metadata supplies that applicability decision.

The Rust report lists exported **stateful** circuits. Pure functions live in `pure_circuits` and are not listed here. A pure-only passport crate can therefore have an empty `circuits` array while still exposing its pure API. The report is not a complete inventory of every generated function or constructor.

## Choose the execution surface

| Your task | Surface | Evidence you still need |
|---|---|---|
| Compute a pure value | `pure_circuits` | Typed input/output tests |
| Execute locally | Native `ledger_contract` method | Expected state, errors, effects and witness behavior |
| Record and replay a stateful call | Generated `recorded` API, when `recorded` is true | VM replay and comparison with independent behavior observations |
| Prepare a call against observed state | Generated `*_call` API, when `observed_call` is true | Correct observation, inputs and transaction preparation |
| Prove and apply a transaction | Separate proof/ledger integration | Actual proof verification and ledger acceptance under the chosen rules |

Enable the generated crate's **`ledger-transaction` Cargo feature** to compile its observed-call methods. The report describes what the emitter can provide; it does not override Cargo feature selection. See [Version and runtime selection](version-selection.md) for dependency identity and version pairing, and [Testing generated contracts with ContractLab](contract-lab.md) for local execution/replay.

## Understand the fields

| Field | Meaning |
|---|---|
| `name` | Original exported stateful circuit name |
| `source` | Source file/line/column when available |
| `recorded` | The backend can emit a complete recorded method for this circuit |
| `observed_call` | The backend can emit its observation-bound call method |
| `proof_required` | Frontend proof applicability, joined from contract metadata |
| `recording_status` | Combined applicability/API classification |
| `recording_unavailable` | Why recorded lowering is unavailable, when reported |
| `observed_call_unavailable` | Why the call facade is unavailable, when reported |

`recording_status` has three values:

- **`available`**: proof applies, and both recorded and observed-call APIs are available.
- **`unavailable`**: proof applies, but at least one required API is unavailable.
- **`not_applicable`**: frontend metadata says this circuit does not require a proof. This status is not itself a backend defect.

Read both Boolean fields. A recorded method can exist while an observed-call method is suppressed because its generated name collides with another exported circuit. Checking only `recorded` is insufficient when your application needs `*_call`.

## Read an available row

This digest-verification row was produced by the frozen ADR0288 compiler and remains available in the freshly generated report:

```json
{
  "name": "verifySchnorrJubjubDigestSignature",
  "source": {"file": "did.compact", "line": 742, "column": 1},
  "recorded": true,
  "observed_call": true,
  "proof_required": true,
  "recording_status": "available"
}
```

It means the APIs can be generated. It is not a proof receipt. Separate ADR0288 evidence records actual input cases, VM replay and strict proof/application checks for this operation.

## Historical diagnostic: turn an unavailable row into a bug report

At the historical ADR0288 checkpoint (2026-10-07), `setVerificationMethodRelation` had `recorded: false`, `observed_call: false` and `recording_status: "unavailable"`. Its recorded diagnostic identified `StateAction::CircuitCall` at `actions[0].action.action.action.actions[0]`.

The source location identifies the Compact export; the path identifies a node in this backend's typed IR. A first reported gap may hide later unsupported combinations. It does not establish that every local circuit call is unsupported or that removing that node will make the whole contract recordable.

A useful report includes:

1. The unchanged source closure, compiler identity, lockfile and runtime selection.
2. The finalized capability report and frontend contract metadata.
3. A small source reducer retaining the failing primitive combination.
4. Expected behavior from an independent oracle, including errors and ordering.
5. Separate results for compilation, native execution, recording/replay and real proofs.

Avoid editing generated Rust or changing the JSON flags to advertise an unavailable API. Fix and test the emitter/runtime responsibility, then regenerate the crate and report. ADR0294/0295 used this workflow to deliver the final relation API. Current source no longer has that recorded gap.

## Check the APIs your application requires

Save the following as `check_observed_api.py`. It checks explicitly named stateful exports in a compiler-produced schema-3 report. It does not claim all exports were tested, generate keys, enable Cargo features or validate a network deployment.

```python
import json
import sys

path, *required_exports = sys.argv[1:]
if not required_exports:
    raise SystemExit('Specify the stateful exports your application needs.')
with open(path, encoding='utf-8') as handle:
    report = json.load(handle)
if report.get('schema_version') != 3 or not isinstance(report.get('circuits'), list):
    raise SystemExit('Expected a finalized Rust capability schema-3 report.')
rows = report['circuits']
by_name = {row['name']: row for row in rows}
if len(by_name) != len(rows):
    raise SystemExit('Duplicate capability names.')
failures = []
for name in required_exports:
    row = by_name.get(name)
    if row is None:
        failures.append(f'{name}: not a reported stateful export')
    elif row.get('proof_required') is not True:
        failures.append(f'{name}: proof is not applicable; choose the pure/native API')
    elif not (row.get('recorded') is True and row.get('observed_call') is True
              and row.get('recording_status') == 'available'):
        gap = row.get('observed_call_unavailable') or row.get('recording_unavailable') or {}
        failures.append(f"{name}: {gap.get('code', 'unavailable')} at {gap.get('path', '<no path>')}")
if failures:
    raise SystemExit('\n'.join(failures))
print('Available generated APIs: ' + ', '.join(required_exports))
```

```sh
python3 check_observed_api.py generated/contract/rust-capabilities.json \
  verifySchnorrJubjubDigestSignature
```

Historically, the checker passed the digest row and refused the relation row at the frozen ADR0288 checkpoint. The current report has twelve available rows, including that relation; this documentation reconciliation read the fresh report but did not rerun the checker or prove those exports. A missing expected export cannot pass through an empty report. Choose a pure/native test instead when proof applicability is false.

## Keep the version numbers separate

| Number at this checkpoint | Owner and purpose |
|---|---|
| Typed IR schema **20** | This branch's private frontend-to-Rust interchange; checks compiler/backend agreement |
| Capability schema **3** | Finalized exported-stateful API/proof metadata |
| Runtime ABI **50** | Generated Rust/runtime compatibility contract |
| Milestone **0.3.0** | Delivery/release planning label |

These are separate project version domains, not a single Compact language version or a public standard called “schema-20”. The backend library initially creates an unclassified schema-2 capability draft; the CLI joins frontend proof metadata and emits finalized schema3. Application checks should consume the finalized report.

All new guides and ADRs remain in Obsidian until milestone closeout. Historical measurements and capability examples retain their exact source checkpoint.
