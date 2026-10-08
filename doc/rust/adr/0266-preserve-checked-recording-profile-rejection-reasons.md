---
id: RUST-ADR-0266
alias: ADR-0266
source_sha256: 14b131e4cb773ad5b27c08254167576f7fe174e2ff3a34b17ff3f0fe038e1a8f
publication_status: published
publication_date: 2026-10-09
publication_source_revision: f9a496660a4a69f4dd7c7927111767ad5c0c12b2
---
# RUST-ADR-0266 — Preserve checked recording-profile rejection reasons

[ADR register](README.md) · [0.3.0 record register](index-0.3.0.md) · [Reference conventions](references-0.3.0.md)

## Publication disposition

This published historical source record belongs to the milestone 0.3.0 documentation collection. **Original opening status:** not separately declared. Read dated amendments below; neither this status nor issue closure certifies the whole milestone. No fresh implementation, security, proof or formal acceptance is implied. Original code examples and local evidence locators are historical; machine-local paths are not portable downloads or current setup instructions.



## Original decision and amendments

## ADR0266 — Preserve checked recording-profile rejection reasons

Read-only review. No production changes, ADR or issue were made. The mutable checkout includes concurrent work; this note refers to the source paths/line regions below and pinned scratch inputs.

### Current boundary

- `recorded.rs:175-190` exposes only `RecordingOutcome::Supported` and `Unsupported(gap)` at the final renderer boundary.
- `recorded/typed_plan.rs:2922-2990` (`lower_shielded_send`) returns `Option<TypedPlan>`. `None` conflates a circuit that is unrelated to this profile with a circuit of the send-result/action shape whose whole-body audit found an unsupported nested value.
- `recorded.rs:7380-7520` selects many `Option<TypedPlan>` profiles with `or_else`; the first admitted plan wins. If none admits, `render_recorded_item` falls through to its generic action/return handling (`recorded.rs:7548 onward`, final return around `recorded.rs:8430`). The final gap therefore need not describe the typed profile's actual failed leaf.
- Native validation/emission runs before capability recording in `lib.rs:3320-3380`; a malformed typed/native declaration still yields `RenderError` first. `StateReturn::Effectful` has an explicit early full-plan rejection around `recorded.rs:1635-1653` and should remain unchanged.

### Concrete repro and actual outcomes

Inputs are `/tmp/rust030-domain-probe/send-base.json` and `/tmp/rust030-domain-probe/send-unused-size.json`, derived from unmodified `tools/compact-rust-backend/tests/shielded-send-schema20-ir.json` (schema 20). The latter changes only `stateful_circuits[1].return_value.value` to `Expr::Let { bindings:[hidden: Uint64 = Expr::SetSize(coins,0)], body:<original send expression> }` and adds a declared Set slot. This is a valid native expression, with the query in an unused binding. The existing closed send test at `typed_plan.rs:4338-4390` already asserts `lower_shielded_send` rejects this family.

Run: `CARGO_TARGET_DIR=/Users/ysh/.codex/worktrees/unassessed-source-bridge/compact/target/adr169 CARGO_INCREMENTAL=0 cargo +1.99.0 run --offline --quiet --manifest-path /tmp/rust030-domain-probe/Cargo.toml -- /tmp/rust030-domain-probe/send-base.json /tmp/rust030-domain-probe/send-unused-size.json`.

Actual public `render_with_capabilities` result: baseline `send_to_self`, `send_to_user`, `send_to_contract`, `send_forward` all recorded=true. Mutant renders successfully; `send_to_self` recorded=false, other three remain true. Its final `RecordingGap` is `UnsupportedReturn`, `StateReturn::Expression`, `path=return_value`, detail `StateReturn::Expression is not supported by recorded Rust lowering`. The disqualifying node is actually `return_value.value.bindings[0].value` (`Expr::SetSize`). No claim of unsafe admission: the result fails closed.

A second check uses unmodified `shielded-receive-schema20-ir.json`; adding an unused `CounterLessThan` binding to the called helper keeps native render valid, makes both receive exports recorded=false, but yields a generic `StateAction::Let` gap at `callee[receiveShielded].actions[0]`. This corroborates the same `None` diagnostic loss, but the send case alone is sufficient for a first slice.

### Smallest checked model if accepted

Introduce a private `ProfileAttempt<T> { NotApplicable, Rejected(RecordingGap), Admitted(T) }` only at the bounded send profile/router boundary. `Rejected` means *in this selected closed domain*, never invalid typed IR. Keep `RecordingOutcome` and capability JSON schema unchanged. Classify unrelated signatures/return forms as `NotApplicable`; after the structural send-result/root-expression gate, audit the entire return expression and helper closure in source order, including unused bindings and both branches. Yield `Rejected` with the first exact unsupported nested node path if that audit fails; yield `Admitted` only after the same typed Plan/effect-count checks used today.

The router must keep existing profile order. It records a rejection but continues trying subsequent profiles; a later admitted profile still wins. If none admits, prefer the profile's exact nested gap only when the generic fallback offers a coarser `UnsupportedReturn` for that same return. Preserve existing first definite generic action/type gaps (for example `render.rs:7208-7375`), source-location wrapping, native error precedence, and the explicit whole-body `Effectful` path. This changes diagnostics, not admitted plans, observed calls, output Rust, local scopes, or runtime ABI.

Before: `lower_shielded_send(...) -> None; generic return -> UnsupportedReturn(path=return_value)`.
After: `lower_shielded_send_checked(...) -> Rejected(UnsupportedExpression(path=return_value.value.bindings[0].value)); later profiles still run; no admission -> exact gap`.

Tests: baseline send 4/4 recorded and generated output byte-identical; hidden SetSize in unused binding, untaken branch, nested argument all refused with exact paths; an unrelated same-result expression classified `NotApplicable`; another eligible profile's successful fallback remains admitted; malformed native identifier/type still returns the original `RenderError` before profile probing. No new profile labels in generated APIs.

### Representative controls and recommendation

- Pure-only closure (`/tmp/rust030-domain-probe/pure-only.json`, derived from receive fixture with no stateful exports): renderer OK; no stateful capability rows. No profile model needed here.
- Read-only assertions (`tests/stateful-assert-schema20-ir.json`): `checked` and `unit_result` recorded=true. Existing `render.rs:11051-11150` checks invalid pure/stateful types and read-only negative cases.
- Witnessed Coracle guess (`tests/coracle-guess-schema20-ir.json`): `guess` recorded=true.
- Funded Coracle start (`tests/coracle-start-schema20-ir.json`): `start` recorded=true.
- Shielded send baseline/mutant above: no admission widening; only a concrete, coarse gap.

Recommendation: accept a diagnostic-only, one-profile tri-state slice if R030-01 requires machine-usable failure attribution. Defer a universal conversion of all `Option<TypedPlan>` profiles: no current probe demonstrates unsafe fallback admission, and changing 20-plus selectors at once would add risk without a measured acceptance gain. The existing fail-closed result is production-safe; this is an engineering/debuggability improvement, not a blocking soundness fix.

### Evidence identities

- Scratch public renderer code: `/tmp/rust030-domain-probe/src/main.rs` SHA256 `a729982fdb1b15bb8802084fdb77a1c1db06fbd882ab34b78164475b08db72d1`.
- Primary mutant: `/tmp/rust030-domain-probe/send-unused-size.json` SHA256 `2d68c8857e7acfac3d34879fc0fa8b95b56b12937537b4aac379d601381c54ab`.
- Output: `/tmp/rust030-domain-probe/send.log` SHA256 `9d9f684a9797631cbaeadd47c08d82df35e44ffc7ad127002d53d2e5d4fafc2a`.
- Other controls: `/tmp/rust030-domain-probe/probe.log` SHA256 `cf77e73790fc30e6179f109ae5824c64f1294931761262add580867986224abd`, `/tmp/rust030-domain-probe/pure.log` SHA256 `686274e5aaba2e81934444518dcf18d9800b7512a98f4c26611ef540f140d445`.

### Accepted bounded decision

Implement the one-profile diagnostic result with an explicit distinction between unrelated shape, rejected closed-domain attempt, and admitted typed plan. Reuse/refactor the existing shielded-send audit to retain source paths; do not add a second semantic evaluator or duplicate permissive whitelist. Later existing profiles retain their order and ability to admit. Report a retained nested rejection only where the existing final generic return gap is coarser for the same return. No admission, generated Rust, runtime, ABI or capability-schema change. All actual corpus capability differences must be reviewed as intentional diagnostic precision; unchanged accepted source output is mandatory. Coverage owner currently measuring frozen backend: begin edits only after its source snapshot/run no longer needs mutation exclusion.

### 2026-10-07 — Delivered

Commit `9949b01340f5801c9ef2f65e805d4427930c2778`; good GPG signature and DCO, local only. 248 isolated tests and strict Clippy pass; all 44 corpus artifacts remain byte-identical. The final implementation uses typed rejection codes without inspecting diagnostic path strings for control flow. Full source snapshots and validation scope: [ADR0266 — Checked recording-profile diagnostic receipt](references-0.3.0.md#note-033). No admission bug, ABI/schema change or full domain-model completion is claimed.
