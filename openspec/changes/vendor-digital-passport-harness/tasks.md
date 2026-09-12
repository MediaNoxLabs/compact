# Tasks: vendor-digital-passport-harness

No compiler change; request the maintainer-applied `skip-changelog` label. All work inside `nix develop`.

## 1. Vendor the Compact sources

- [ ] 1.1 Clone upstream at rev `cdeb860b` and copy only `packages/midnight-verifiable-credential-digital-passport/src/**.compact` → `examples/dogfood/digital-passport-credential/src/` (6 files: entry + five modules), verbatim with upstream headers intact. Vendor **no** `.ts` or other files. Verify: per-file `cmp` against the upstream clone; the vendored `src/**/*.compact` file list equals upstream's; `find examples/dogfood -name '*.ts' | wc -l` is `0`.
- [ ] 1.2 Stage the core without pnpm: `curl -L $(npm view @midnight-ntwrk/credential-compact@0.1.0-rc3 dist.tarball) | tar xz`, copy `dist/credentials.compact` + `dist/credentials/` → `examples/dogfood/digital-passport-credential/core-compact-staging/` (15 `.compact` files). Verify: file count is 15 and bytes match the tarball.
- [ ] 1.3 Write `examples/dogfood/digital-passport-credential/PROVENANCE.md`: upstream URL, rev `cdeb860b`, license, npm core package+version, staging procedure (incl. the no-pnpm variant), refresh policy (explicit manual re-sync; rev updated in the same commit), and the compact-only / no-TypeScript statement. Verify: a reader can re-sync from PROVENANCE alone.
- [ ] 1.4 Run `git status --ignored` over the vendored tree and confirm every file is tracked. Verify: `git ls-files examples/dogfood | wc -l` is 22 (6 + 15 + `PROVENANCE.md`).

## 2. Header exclusion + compile smoke

- [ ] 2.1 Add the `dogfood` entry to `header_config.json` `excluded_directories`. Verify: `python add_headers.py --validate` passes with the vendored tree present.
- [ ] 2.2 Local compile smoke: `result/bin/compactc --target ts --skip-zk examples/dogfood/digital-passport-credential/src/digital-passport-credential.compact /tmp/out-ts/` exits 0. Verify: TS output generated. (The rust target is expected to fail until the fix changes; record the exact diagnostics.)

## 3. TS reference captures

- [ ] 3.1 Author `tests-e2e-rust/fixtures/capture-digital-passport-credential.mjs` (Apache header) per the existing capture pattern: civil-date helpers (`assertCivilDateMatchesEpochDays`, `assertValidDigitalPassportAgePredicate` — every conditional-expression site and assert-fail path) + one issuance/presentation/verification round-trip. Verify: script runs under the TS target and emits committed JSON.
- [ ] 3.2 Commit the JSON reference and document in the change notes which helper circuits and round-trip steps it pins. Verify: the JSON covers every helper conditional site.

## 4. Oracle probes (expected refusals)

- [ ] 4.1 Add `rejection_corpus` entries for the real ternary sites (minimal extracts in each of the three syntactic positions: const RHS, assert argument, interior arithmetic operand) asserting the exact current refusal diagnostics. Verify: `cargo test -p tests-e2e-rust rejection_corpus` green with the pre-fix compiler; each entry names the fix change that flips it.
- [ ] 4.2 Add a `rejection_corpus` entry for a mixed-width comparison operand (the `q * 4`-vs-`Uint<32>` shape) asserting the current refusal. Verify: green pre-fix; names `fix-mixed-width-operand` as the flipper.
- [ ] 4.3 Add a whole-entry expected-failure gate asserting `compactc --target rust` currently exits non-zero on the vendored entry with the known diagnostic, designed to flip in `add-digital-passport-dogfood-fixture`. Verify: gate green pre-fix, documented as expected-fail.
- [ ] 4.4 Run the CI lane added by `compiler-backed-ci-gate` and confirm the probes execute there (not filtered). Verify: the lane runs `rust_backend_` tests.
