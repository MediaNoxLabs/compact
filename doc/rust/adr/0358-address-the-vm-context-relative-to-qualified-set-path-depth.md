---
id: RUST-ADR-0358
alias: ADR-0358
source_sha256: 0b03258d50d227f440194d5779935f08967f3aa97d04e4005411327643fdab2b
publication_status: published
publication_date: 2026-10-09
publication_source_revision: f9a496660a4a69f4dd7c7927111767ad5c0c12b2
---
# RUST-ADR-0358 — Address the VM context relative to qualified Set path depth

[ADR register](README.md) · [0.3.0 record register](index-0.3.0.md) · [Reference conventions](references-0.3.0.md)

## Publication disposition

This published historical source record belongs to the milestone 0.3.0 documentation collection. **Original opening status:** delivered; runtime fix and maintained gate independently reviewed. Read dated amendments below; neither this status nor issue closure certifies the whole milestone. No fresh implementation, security, proof or formal acceptance is implied. Original code examples and local evidence locators are historical; machine-local paths are not portable downloads or current setup instructions.



## Original decision and amendments

## ADR0358 — Address the VM context relative to qualified Set path depth

Date: 2026-10-08
Status: delivered; runtime fix and maintained gate independently reviewed
Parents: R03013/#357, R03017/#361, R03007/#351

### Problem and evidence

Independent audit-fix retest at e6807884 observes that qualified_coin_set_insert_program emits Dup4 after Idx with push_path. The VM context depth varies with physical path depth. Root source review finds native SetInsertCoin accepts validated two-level physical paths, so this is potentially reachable in chunked generated storage. Root-only recorded profiles do not establish native safety. No end-to-end execution failure is yet claimed.

Generic Cell writes at empty or >=3depth are separate: the standard renderer rejects these paths; do not extend this slice into generic arbitrary-depth support. This is a concrete audit follow-up, not renewed specification/conformance expansion.

### Before / proposed after

Before (runtime helper, root-only offset):
```rust
Op::Idx { push_path: true, /* field path */ },
Op::Dup { n: 4 },
```
Proposed after, only if the pinned VM and a real positive/refusal reproducer validate the offset:
```rust
let context_depth = checked_context_depth(path.len())?;
Op::Idx { push_path: true, /* field path */ },
Op::Dup { n: context_depth },
```
The proposed context offset is 2*path.len()+2, checked for arithmetic overflow and the pinned Dup nibble bound before lookup/query. The root case must emit identical Dup4 and preserve exact compiler/TS program semantics. No silent fallback or guessed coin allocation. Preserve public signatures and existing path representability rejection.

### Ownership and validation

Use a small two-level state/Set with a real allocated coin commitment, and compare root/native/recorded results only where that API is actually admitted. Reproduce existing failure before editing. Add a small generated chunked qualified-Set contract if necessary to prove compiler reachability, with output/state/program assertions and focused reuse of existing oracle patterns. Do not claim a full proof or formal parity from runtime tests.

If confirmed, change only the qualified Set program owner plus focused tests; no ABI/IR/version/pin bump, broad runtime rewrite, or compiler admission expansion. Check root wire sequence, depth2 execution, first invalid computed Dup depth, and no mutation/record adoption on error. Seek independent source review; rerun only affected tests/Clippy and instrumented delta. Record actual outcome, including disproven premises.

### Remaining limits

Byte-admission versus decoded heap/CPU acceptance remains a separate pending owner question. Existing three audit fixes stay delivered. Broader milestone audit cannot ignore a newly confirmed supported native failure; determine reachability and severity before parent acceptance. Unrelated 0.4.0 document edits in the shared checkout remain untouched.

Issue: https://github.com/MediaNoxLabs/compact/issues/493 (rust-backend-v0.3.0).


### Draft identifier correction

This unpublished draft was initially allocated0354. A concurrent0.4.0 planning session reserved0354–0357 in the repository before publication coordination. The draft moves to0358 with issue493 unchanged. Historical draft references remain traceable through this amendment; no published ADR is renumbered.


### Reproduction and local delivery

The allocated root control passed while depth2 [1,14] with actual index11 returned LedgerQueryRejected(Execution(Decode(InvalidBuiltinDecode("Fr")))). This was a runtime error, not a product panic; the old failing test unwrapped it. The helper now uses checked2*depth+2, preserving rootDup4 and rejecting depth7 before lookup. Runtime positives include emptyroot, depth2 and depth6 with real commitment indices.

Real source qualified-coin-set-chunked.compact compiles with the pinned compiler and produces path[1,14]. An external consumer of the unedited generated crate passes insertion/membership with index11, absence of wrongindex7, and untouched padding-field assertions. This qualifies generated native execution, not generated recorded support, which remains unavailable for that fixture. Direct runtime RecordingFrame/replay tests are separately labeled. The existing root qualifiedSet TS-oracle and strict scoped Clippy pass.

Signed GPG/DCO commit ef352bcd7062dcf139478c91b64384b0459c33f3 is pushed. Independent sibling source review found no actionable defect. Exact clean-commit external retest and fresh runtime-only coverage are underway. No compiler admission, ABI, IR, pin or package-version change.


### External retest D1 and maintained gate remediation

The exact ef352bcd external retest accepted the runtime fix but raised D1medium: the new Compact fixture had no maintained repository gate consumer. Its one-off generatedconsumer receipt proved this execution, but did not guard future compiler reachability. Root accepted the finding.

Commit231559f1 adds compilation and actual nested slot/nativeaction/capability assertions to check_compactc_target.py --adt-set-qualified, then requires that command in local_parity_gate.py --full. The exact branch passes with the qualified compiler/Scheme pair;19orchestration tests confirm the mandatory invocation. This is not a rerun of the fullproof suite. The emitted chunked insert_coin still reports no recorded/observed API; contains remainsrecorded. Existing rootqualifiedSet recorded support is unaffected.

This gate-only change does not alter runtime/emitter Rust production lines, so ef352bcd runtime97.83% and e680 backend95.11% coverage remain separately qualified through source equality. A narrowly scoped external D1 sourceconfirmation is running on clean231559f1. Preserve the original medium finding and its remediation rather than rewriting the earlier review as clean.


### Final independent D1 closure

Exact clean231559f1 external source confirmation completed38.85seconds, exit0: D1closed, no remaining defect in scoped gate follow-up. Independent sibling review agrees. Selected compiler gate and19orchestration tests passed; runtime/codegen production source unchanged since respective coverage receipts. Issue493 is complete. Full milestone assurance/resource decision and finalqualification remain open.

Final evidence [ADR0358 — Maintained gate and D1 closure at231559f1.zip](references-0.3.0.md#note-135), SHA256 `42ff1fe3393b5fc55025ef6813403e09d4eab51f290444f357510d4a7535e5f8`. Earlier external medium finding is preserved in [Qualified Set external retest at ef352bcd](references-0.3.0.md#note-159).
