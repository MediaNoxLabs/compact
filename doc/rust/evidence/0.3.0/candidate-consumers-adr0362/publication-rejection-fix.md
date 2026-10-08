# ADR0362 / #499 — final gate publication rejection harness correction

## Cause

The stopped full gate reported two failures in `397-compiler-rejections.log`. They share one stale expectation: after manually renaming a complete output to an interrupted backup, the harness selected an invalid runtime root and expected output recovery.

`check_output_publication` gained that expectation at `e4984cb1`. Later runtime compatibility selection (`987b56b1`) intentionally moved `compatibility::validate_root` before `StagedOutput::new` (`compactc.rs:687–697`). Invalid explicit runtime must be refused before staging/recovery, with no fallback. The second unsafe-output assertion saw the still-absent output and failed as a consequence. Production ordering is correct; no runtime/CLI change is warranted.

## Narrow change

Only `tools/compact-rust-backend/check_rejections.py` changed.

- Existing fresh/existing-output invalid-runtime protections remain.
- After renaming to the interrupted backup, invalid runtime selection must refuse with the runtime diagnostic and leave output absent, the backup directory present, all file hashes and immediate directory entries unchanged. Recovery is not expected for this early failure.
- A valid runtime plus a real invalid Boolean source reaches `StagedOutput::new`, restores the previous output, then fails in the frontend. The test requires the exact Boolean diagnostic and source line/column, complete preserved output bytes, no remaining backup and no leaked staging directories.
- Existing file/symlink protection and successful replacement checks remain unchanged. The obsolete “post-render failure” description was corrected to reflect early runtime validation.

No safety assertion was removed to make the gate pass. Failure order and failed-rebuild cleanup now have independent positive evidence. The invalid source is a tiny handwritten Compact input, not a forged frontend failure or arbitrary nonzero process.

## Verification

The exact maintained `check_rejections.py` gate ran with the candidate Rust CLI and freshly built candidate Scheme snapshot. It passed: **4 source rejections, 6 source contexts, output publication, proof capabilities and shared Boolean typing; 0 failures**. Terminal exit0. No Cargo/build, broad full-gate rerun or production-source mutation by this agent. Scoped `git diff --check` passed.

Raw passing log: `publication-rejection-fixed.log`. Original failing log remains untouched in `full-gate/logs/397-compiler-rejections.log`. `publication-rejection-fix.json` binds source, binary hashes, exact command and both logs. Root owns ADR/issue, signed commit and final-candidate continuation.

## Final review correction

Root requested strict diagnostic equality. The new frontend-refusal assertion now compares the complete stripped stderr to the expected filename, line2/char10 and Boolean-vs-Field diagnostic. The exact maintained gate was rerun after this change and again passed with0 failures/exit0. Final source SHA-256: `b2eccc1e4a5c9d81910ca2f3def28fe5208dad54ee8083306f74658411063185`. The original failing log is also copied to `publication-rejection-before.log`; the earlier passing version remains `publication-rejection-initial-pass.log`. `publication-rejection-fixed.log` and the JSON receipt refer to final source.
