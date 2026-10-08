---
id: RUST-ADR-0274
alias: ADR-0274
source_sha256: d8812cb458a9ee2c077fe219db36035d1aefb618b3598a66c55c0547362645ca
publication_status: published
publication_date: 2026-10-09
publication_source_revision: f9a496660a4a69f4dd7c7927111767ad5c0c12b2
---
# RUST-ADR-0274 — Preserve default worker stack in typed Map lowering

[ADR register](README.md) · [0.3.0 record register](index-0.3.0.md) · [Reference conventions](references-0.3.0.md)

## Publication disposition

This published historical source record belongs to the milestone 0.3.0 documentation collection. **Original opening status:** locally delivered at signed commit `a536360ab002149afc7f6b33465c6d3071eb1d12`, 2026-10-07. Historical diagnosis and proposal below are preserved. Parent R030-05/#349 and R030-18/#362; blocks child #393. Root review caught a default-worker failure in the final evidence despite a passing64MiB diagnostic run. Read dated amendments below; neither this status nor issue closure certifies the whole milestone. No fresh implementation, security, proof or formal acceptance is implied. Original code examples and local evidence locators are historical; machine-local paths are not portable downloads or current setup instructions.



## Original decision and amendments

## ADR0274 — Preserve default worker stack in typed Map lowering

Status: locally delivered at signed commit `a536360ab002149afc7f6b33465c6d3071eb1d12`, 2026-10-07. Historical diagnosis and proposal below are preserved. Parent R030-05/#349 and R030-18/#362; blocks child #393. Root review caught a default-worker failure in the final evidence despite a passing64MiB diagnostic run.

### Reproduction

Same current backend source/toolchain/profile, with only typed_plan.rs and composition.rs restored to the committed pre-Map baseline, passes reset_payout::tests::original_cash_out_preserves_send_before_reset_and_saved_result on the ordinary Rust test worker. Current Map implementation deterministically stack-overflows that same isolated test. The larger-stack full run is retained only as diagnostic evidence, not acceptance. No user stack environment or public compiler switch will be changed.

### Problem and decision

Adding unrelated Map lowering must not consume the default debug worker's stack budget on an existing shielded payout circuit. Determine whether large recursive Plan expression/action stack frames grew due to new inline match-arm locals. Prefer a small private Map-lowering component/method that owns typed membership/insertion/removal lowering, reuses the same Plan scope and upstream slots, and shortens the existing recursive frame. Do not widen recording admission, box all IR nodes, change ABI/schema, reorder operands, change stack sizes, or annotate tests to hide this regression.

Before (shape): recursive Plan::action contains the complete MapInsert checking and AST construction inline. After (candidate): that match arm delegates to a small private map_insert(...) method with the same explicit operands/scope/steps; actual choice must follow measured before/after default-stack behavior. Use ordinary functions, no macro. No public generated API change.

### Acceptance

- Retain exact same-profile baseline/current red logs and source hashes; record the actual default-stack setup (RUST_MIN_STACK absent).
- The isolated original reset-payout test and full backend cohort pass without increasing test/worker stack limits. Existing explicitly scoped historical tests retain their original policy.
- New Map admission/refusal tests and original DID fixture source/capabilities stay unchanged; complete immutable before/after corpus comparison must pass apart from the already intended two DID capabilities of ADR0269.
- Rebuild immutable compiler after extraction; compare generated source/capabilities with the already proved Service output. Re-run semantic proof only if generated program/IR or runtime behavior changes; source-only identity receipts must state the refactor honestly.
- Strict Clippy and relevant formatting pass. Document module ownership/maintenance benefit, no generalized performance claim from one stack regression.

Root postponed a separate recorded.rs profile extraction until this regression is resolved. Do not conflate that prospective cleanup with this demonstrated resource defect.

Issue: https://github.com/MediaNoxLabs/compact/issues/398

### Local delivery — 2026-10-07

The measured culprit was the inline recursive `MapMember` lowering in `Plan::expression`, rather than the illustrative `MapInsert` arm. A small private `composition_map_member` helper retains the same complete audit, typed scope, Map declaration, key checks, metered query, and result. Same-profile controls show committed pre-Map and no-MapMember variants pass the original reset-payout test on an ordinary worker, inline MapMember overflows, and extracted MapMember passes. Final isolated ADR0269 90 library tests and six Map tests pass without a stack override; full backend and strict Clippy pass. The generated original DID output is byte-identical before/after extraction; 183-source differential proves only the two intended Service capabilities changed from the committed post-ADR0265 baseline.

Evidence: [ADR0274 — Default worker stack regression and correction](references-0.3.0.md#note-045), [ADR0269 — Service Map delivery receipt](references-0.3.0.md#note-039), [ADR0269 — 183-source immutable renderer differential](references-0.3.0.md#note-038). The original strict Service proof predates this output-identical function extraction, and was not rerun solely for it. No general performance improvement is claimed.


### Issue closeout

[MediaNoxLabs/compact#398](https://github.com/MediaNoxLabs/compact/issues/398) closed as completed after the paired default-worker evidence was saved; delivery comment: https://github.com/MediaNoxLabs/compact/issues/398#issuecomment-6027055851.
