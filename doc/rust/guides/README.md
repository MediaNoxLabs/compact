# Rust developer guides

Start at [the task index](index.md) to compile a contract, implement witnesses, run local tests and explore ledger integration. The [guide manifest](guide-manifest.json) identifies source provenance; [retained evidence](../evidence/0.3.0/index.md) records the commands and limits behind the examples.

W2/D2 source bodies, manifests, includes and `prepare-sdk.py` are byte-identical. Passport and named-record handwritten source/locks/captures are also byte-identical; their READMEs state which generated dependencies must be prepared. Tutorial command placeholders describe caller-selected paths, not machine-local navigation links. Old absolute paths inside frozen receipts remain historical provenance.

See [candidate qualification](candidate-qualification.md) for the current validation and documentation status. Companion pages retain some named historical vault references as plain text because their full evidence archives are outside this bounded package. Those references are not promoted into new/current acceptance claims.

## Recording-effect branch

[Checked recording and source diagnostics](checked-recording.md) describes the current architecture and extension workflow. The [capability and retirement book](../recording-effect-algebra/capability-and-retirement-book.md) separates fresh, reused and pending evidence. The examples and receipts below retain the 0.3.0 source epoch; current primitive support is determined by checked operations and composition.

## Retained 0.3.0 qualification boundary

Encoded-byte admission is owner-approved for0.3.0 with an opt-in64MiB preset; aggregate decoded heap/object/CPU containment remains deferred. Optional [ProofLab](local-proof-tests.md) uses official traits; its successful-prove rustdoc call is compile-checked only, not an executed proof.

The [candidate follow-up receipts](../evidence/0.3.0/index.md#candidate-qualification-follow-ups) separately retain bounded CI, W2 consumers, API migration, changed-component coverage, portable packaging and conformance baseline maintenance. Each receipt retains its original source and execution scope. See [candidate qualification](candidate-qualification.md) for the overall status.

## Current candidate pointer

[Candidate qualification](candidate-qualification.md) distinguishes bounded CI, full local validation and documentation publication.

The [capability and retirement book](../recording-effect-algebra/capability-and-retirement-book.md) and [original 84-row ledger](../recording-effect-algebra/retirement-ledger.json) preserve boundary accounting and distinguish current implementation from retained qualification epochs. [Retained conformance inputs](retained-conformance-baseline.md) explains the explicit archive required by the one historical baseline case.

[Portable Python fixtures](retained-python-fixtures.md) describes the checked-in
historical baseline and explicitly selected manual graph controls.

[Recording-effect closure](../recording-effect-algebra/closure-report.md) records the final local source, stage-separated qualification, original completion criteria and follow-ups. The candidate-qualification page above retains its 0.3.0 epoch.

## Public regression and CI checks

- [Required Rust-AST checks](required-checks.md): stable aggregate, core/MSRV and
  public Python controls, with explicit branch-rule activation after merge.
- [Qualification evidence inventory](../recording-effect-algebra/qualification-evidence-summary.md):
  public source providers, bounded derivative acceptance summaries and withheld
  historical constituents.
- [Issue closeout map](../recording-effect-algebra/issue-closeout.md): delivered
  work, separate follow-ups and conditions that remain open.
