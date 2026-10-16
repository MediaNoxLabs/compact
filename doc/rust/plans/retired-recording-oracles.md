# Milestone proposal: retire historical recording recognizers

Status: planned follow-up to checked recording (#508); this document does not create a GitHub milestone or remove existing tests.

## Outcome

Remove retired whole-circuit recognizers from the current test implementation after independent composition laws and small source examples cover their useful assertions. Preserve historical replay evidence and the original84-boundary ledger. Production recording already uses checked effect composition; this milestone concerns test-only references.

## Delivery slices

1. Inventory each `#[cfg(test)]` recognizer, its callers and the semantic facts it still supplies. Separate old shape admission from type/layout/resource/encoding obligations. Record dependencies on historical replay inputs.
2. Map assertions to explicit native, recording, replay, preparation and applicable proof laws. Cover operand/witness order, same-type wrong-slot writes, dropped effects, public encoding, failures and resource refusals. Type-correctness alone is insufficient.
3. Replace one family at a time with readable source snippets, exact expected observations and mutation/refusal controls. Keep independent expectations; calling the production implementation twice is not a replacement oracle. Preserve useful negative cases and their provenance.
4. Delete a recognizer only after its callers use the replacement and independent review accepts the coverage map. Archive immutable historical inputs under the existing authentication/privacy policy; do not rewrite their authorities or remove live mutation checks.
5. Qualify the final candidate with affected Rust/consumer gates and required historical qualification when its execution machinery changes. Reuse prior proof/application evidence only under complete unchanged relevant premises.

## Exit criteria

- No current test depends on a retired whole-body recognizer; semantic integration constraints remain explicit.
- Every removed assertion has a named replacement or a reviewed redundant-assertion disposition.
- Original84-boundary accounting, historical inputs, mutation/refusal controls and stage-specific proof scope are preserved.
- Original expected values are not regenerated merely to accept changed output.
- Independent coding-agent review accepts the replacements and final qualification. No formal semantic-equivalence or universal language-coverage claim is implied.

## Current release boundary

The present PR retains these oracles to preserve regression evidence. This follow-up must not delay fixes for ordinary test discovery, MSRV configuration, release metadata or structured lowering refusals.
