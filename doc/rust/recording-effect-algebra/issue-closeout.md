# Recording-effect issue closeout map

Prepared 2026-10-17 after PR651 merged. This is a reviewable map for reconciling
GitHub issue status; an open ticket alone does not prove an implementation gap.
The [availability policy](evidence-availability.md) applies to historical results.
The whole 0.4.0 milestone is not closed by this map.

| Issue | Delivered work and public anchor | Closeout disposition |
| --- | --- | --- |
| [#508](https://github.com/betsudo/compact/issues/508) | [Core PR641](https://github.com/betsudo/compact/pull/641), residual PR645–PR651 and [closure crosswalk](closure-report.md#original-eleven-completion-conditions) | Local EA00–EA13/DoD accepted; publication merged. Reconcile the stale EA checklist and close the epic after owner review of this map. |
| [#636](https://github.com/betsudo/compact/issues/636) | PR641 restores independent reviewed emission snapshots; [fixture output checker](../../../tools/compact-rust-backend/check_fixture_outputs.py) retains explicit update/refusal behavior | Delivered; reconcile against original scoped acceptance, not full-spec equivalence. |
| [#639](https://github.com/betsudo/compact/issues/639) | PR641 metadata-only retained oracle provenance; [PR648](https://github.com/betsudo/compact/pull/648) separately qualifies the current wide-add capture | Delivered portable metadata successor; no new historical capture/proof claim. |
| [#640](https://github.com/betsudo/compact/issues/640) | PR641; [backend Cargo manifest](../../../tools/compact-rust-backend/Cargo.toml) explicitly includes both encoding-583/resource-diagnostics-597 fixtures | Delivered packaging correction; preserve original payloads/assertions. |
| [#635](https://github.com/betsudo/compact/issues/635) | [PR642](https://github.com/betsudo/compact/pull/642) keeps Forge an explicit local opt-in | Delivered original optionality scope; newer Forge qualification belongs to #655. |
| [#632](https://github.com/betsudo/compact/issues/632) | [PR645](https://github.com/betsudo/compact/pull/645) publishes the optional sharder; authenticated historical dispatcher/views remain separately retained | Do not close from sharder publication alone. Public portability/reproduction remains bounded by #653. |
| [#652](https://github.com/betsudo/compact/issues/652) | New public file-closure/lifetime tests, fixed retained authenticator and explicit CI collection | Local candidate; close only with reviewed delivery of those controls. Genuine graph qualification remains separate. |
| [#653](https://github.com/betsudo/compact/issues/653) | [Constituent inventory](qualification-evidence-summary.md), original identities and explicit withheld materials | Inventory route prepared; full public 974-case reproduction remains unmet. Keep that limitation visible rather than closing all criteria as done. |
| [#654](https://github.com/betsudo/compact/issues/654) | [Required checks](../guides/required-checks.md), public Python runner and stable aggregate | Workflow prepared locally; administrator must enable the required check and observe hosted execution after merge. |
| [#655](https://github.com/betsudo/compact/issues/655) | Separate Forge acceleration candidate | Separate PR; preserve uncached qualification and real proof/verification/assertion execution. |

No remote issue closures, checklist edits, milestone closure or repository ruleset
changes are performed merely by committing this map. PR descriptions should
reference remaining operational/reproduction conditions honestly.
