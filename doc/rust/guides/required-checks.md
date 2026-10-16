# Required Rust-AST checks

The `Rust Backend Checks` workflow defines **Rust AST / required** as the stable
branch-protection check for `codex/rust-backend-ast`. It runs on every PR targeting
that branch and every push to it, including documentation-only changes. There
are no path filters on this required workflow.

The aggregate requires successful completion of both matrix lanes and the public
Python job. A failed, cancelled, skipped or missing prerequisite cannot pass.
The core receipt also rejects incomplete checks or changed Cargo locks.

## What runs

| Lane | Checks |
| --- | --- |
| Rust core owners | Formatting; locked/offline all-feature tests and strict all-target Clippy for backend, runtime, macros and testkit |
| Rust 1.89 | Backend and generated Jubjub consumer; separately packaged backend; unchanged locks |
| Public Python controls | Ordinary backend Python discovery plus all eight explicit historical-sharder unit methods; exact manual-graph skip policy |

The public Python job runs:

```sh
python3 -B tools/compact-rust-backend/ci_python_checks.py --receipt /tmp/public-python.json
```

Leave `COMPACT_GUARDED_POINT_ROOT` and `COMPACT_CONFORMANCE_BASELINE_ROOT` unset.
The gate refuses environment overrides, empty/duplicate discovery, unrun cases,
unknown skips and changes to the expected manual skip population. Nineteen genuine
graph-dependent parent methods currently produce twenty skip events because one
parent contains two subtests. These are explicitly reported, not public coverage.
The new file-authentication population executes without skips or private setup;
see [retained Python fixtures](retained-python-fixtures.md).

The original authentic graph checks remain available through their explicit
qualification path. Synthetic unit inputs do not qualify historical graphs,
source-to-ZKIR equivalence, proofs or the original 974-case registry. Neither this
bounded workflow nor a green aggregate replaces compiler regeneration, full
consumer/adoption qualification or proof/application gates.

## Enable enforcement after merge

A workflow definition alone does not require passing checks before merge. Configure
the repository branch rule/ruleset for `codex/rust-backend-ast` to require the exact
check name **Rust AST / required**, then confirm its first actual hosted run and
merge-queue policy if one is used. Do not treat an empty check list or a skipped
workflow as qualification. Use ordinary commit messages without `[skip ci]` when
requesting that hosted run.

The local-only delivery prepares the workflow and tests; it does not mutate
GitHub rulesets or dispatch CI. A repository administrator must enable enforcement
when the workflow is merged. Issue #654 records that remaining operational step.

## Forge separation

This required gate has no Forge dependency and uses ordinary tests. The separate
Forge acceleration delivery must qualify compatible keys and preserve actual
proofs, verification, assertions and an uncached key-generation lane. Build caches
are not cached test results. Do not add a key-cache setup cost to lanes that
perform no key generation.
