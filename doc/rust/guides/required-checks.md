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

## Temporary repository workflow profiles

GitHub supports enabling and disabling individual workflows through its API.
[`workflow_control.py`](../../../scripts/workflow_control.py) wraps those controls
with a read-only plan, an explicit repository, a create-only state snapshot and
live state verification. It uses the existing `gh` login and never edits workflow
files, cancels running jobs, merges PRs or changes branch protection.

```sh
python3 scripts/workflow_control.py --repo betsudo/compact list
python3 scripts/workflow_control.py --repo betsudo/compact focus

# Apply the displayed focus plan; retain the snapshot until it is restored.
CI_STATE_DIR="$(mktemp -d)"
python3 scripts/workflow_control.py --repo betsudo/compact focus \
  --apply --snapshot "$CI_STATE_DIR/before.json"

# Enable one workflow when its component needs qualification.
python3 scripts/workflow_control.py --repo betsudo/compact enable ci-specification.yml \
  --apply --snapshot "$CI_STATE_DIR/enable-agda.json"

# Restore only the operations from the original focus plan.
python3 scripts/workflow_control.py --repo betsudo/compact restore "$CI_STATE_DIR/before.json" \
  --apply --snapshot "$CI_STATE_DIR/restore.json"
```

`enable` and `disable` accept exact workflow filenames. Without `--apply`, every
command is a dry run. Apply writes its snapshot before the first remote mutation;
if a later API call fails, the same snapshot can reconcile partial changes.
Restoration checks repository, workflow identity and current state. Unknown
workflows are left unchanged; an explicitly requested unregistered workflow is an
error. The script protects Rust-AST, Forge, compiler-build and security workflows
from disabling. It refuses unexpected states such as inactivity suspension.

The `focus` profile pauses editor, TypeScript debugger/runtime, Agda, unrelated
CLI/release/publishing, upstream contract qualification, documentation sync and
scheduled fuzzing workflows. Rust, compiler-build, security, ledger-line checks,
changelog checks and result reporting remain available. This is a temporary Rust
backend delivery profile: restore or enable the relevant workflows before changing
those components, specification work or full release qualification.

**Workflow state is repository-wide, across all branches.** Disabling a workflow
can leave a required check on another branch pending; inspect that branch's rules
before applying a profile there. Enabling a workflow does not replay missed events.
Use an eligible new PR event or an explicit dispatch where the workflow supports
it. Manual dispatch registration may require the file on GitHub's default branch;
new feature-branch workflows can run from a fresh PR event. A disabled workflow or
an empty check list never counts as a passing gate.

## Header regressions versus inherited header debt

The AST line has pre-existing files that do not meet the repository's full license
header template. Short SPDX notices are **not** accepted as replacements by this
policy. Pull requests run `scripts/check_header_regressions.py` against the frozen
PR base commit: it runs the original full-tree validator, reports all missing
headers, and tolerates a violation only when the file's current bytes equal its
base blob. New, renamed or modified violations fail. The receipt identifies every
inherited path and digest; a passing PR result means **no new violations**, not
full-tree compliance. All newly authored/modified Python files in these PRs carry
the full template.

The original `python3 add_headers.py --validate` remains unchanged and runs strictly
for scheduled/manual Compiler Build qualification. It will still report inherited
debt until that work is remediated; this is an explicit limitation tracked in
[issue #659](https://github.com/betsudo/compact/issues/659).
Retained and generated fixture bytes are not rewritten to make the gate green.
Base objects are read with two batched Git calls rather than one process per file.
