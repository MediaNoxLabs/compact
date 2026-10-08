---
id: RUST-ADR-0341
alias: ADR-0341
source_sha256: da4fed6aa4d4649feff69d48e742180c759cbd71df5121e4fc71d62bc84792f7
publication_status: published
publication_date: 2026-10-09
publication_source_revision: f9a496660a4a69f4dd7c7927111767ad5c0c12b2
---
# RUST-ADR-0341 — Verify bounded call-graph resource accounting with seeded properties

[ADR register](README.md) · [0.3.0 record register](index-0.3.0.md) · [Reference conventions](references-0.3.0.md)

## Publication disposition

This published historical source record belongs to the milestone 0.3.0 documentation collection. **Original opening status:** accepted for bounded test-only implementation and isolated mutation experiments. Read dated amendments below; neither this status nor issue closure certifies the whole milestone. No fresh implementation, security, proof or formal acceptance is implied. Original code examples and local evidence locators are historical; machine-local paths are not portable downloads or current setup instructions.



## Original decision and amendments

## ADR0341 — Verify bounded call-graph resource accounting with seeded properties

Date:2026-10-07
Status: accepted for bounded test-only implementation and isolated mutation experiments
Parent:R03017/#361

Root reviewed the existing tests and retained evidence. This decision authorizes the specific finite model below; it does not authorize runtime trust work or a general unbounded fuzz campaign.

### Problem

The existing36 deterministic resource cases generate a single chain topology with repeated adjacent edges. They verify depth/edge count and a lower bound on work. Hand-written pure-cycle/DAG and malformed-IR tests add useful controls, but there is no retained seed/budget result for a varied acyclic graph family, nor a verified resource-specific code-mutant pair in the reviewed ADR0291 evidence.

### Ownership

Add a cohesive test-only `src/resource_limits/seeded_graph_tests.rs` and a cfg(test) module link in `resource_limits.rs`. Reuse existing private types/policy. If a smaller nested module under tests.rs is clearer, that is equivalent. Do not add production helpers or change rendering/metrics just to simplify testing. Scratch-only mutants and receipts live in an isolated `/tmp` copy after a frozen source/lock handoff.

### Finite generator and independent expectation

- Four fixed u64 seeds, e.g. `0x03017001,0x03017002,0x03017003,0x03017004`,16cases each: **64 case identities** (count distinct serialized inputs separately; do not assume seed variation guarantees uniqueness). Use a tiny explicitly wrapping deterministic sequence in test code; no new rand dependency. Record the exact generator revision. Seed alone is not enough if generator code changes.
- Each graph has1–8 stateful Unit declarations, at most3 outgoing call **occurrences** per declaration, targets only later declaration indices, zero parameters, Unit result/return, and one final `Assert(true,"ok")` per declaration. Names are fixed-width ASCII so renaming/permutation does not alter string budgets accidentally. Keep serialized case ≤32KiB and call-occurrence/path enumeration bounded by construction.
- Include explicit mandatory diamond, repeated-target and disconnected fixtures in the finite set. Assert that the final set actually contains these motifs; do not rely on chance. Keep at least one direct edge and depth>1 control for the limit test.
- An independent bounded path/occurrence enumerator starts at **every declaration**, including disconnected/internal declarations. For this intentionally simple model, local owned syntax work is hand-accounted as `7 + 2*outdegree`: declaration/name/result/return(4), Assert/Boolean/message(3), and call/name(2) per occurrence. Verify this definition with a tiny literal base example, rather than taking expectations from `measure`.
- From explicit unfolding, derive expected total expanded work and longest path in declarations; raw nodes are `1 + sum(local_owned_work)` and call edges are the total input occurrence count. The implementation's color/postorder DP is not reused in this oracle. Failures print seed,case,edges,expected/actual and compact serialized input.

### Required checks

1. For each case, `measure(...,Limits::CENSUS)` returns Acyclic and exact nodes/edge count/call depth/expanded work against the independent model.
2. Deterministically permute declaration order and remap names by source identity, preserving call targets. Node/work/call depth/expanded depth/string/literal metrics must stay equal. Do **not** require pending-peak equality: worklist scheduling can legitimately change it. No source byte equality claim for declaration-order changes.
3. For each nonzero exact metric, independently lower only expanded-work or call-depth budget: exact limit accepts, one less refuses with exact Kind/limit/observed fields. Keep other limits census-sized, so an earlier unrelated budget cannot satisfy the test accidentally.
4. Select one small graph per seed for ordinary public render + syn syntax acceptance under the actual DEFAULT policy. Do not infer compiled/executed Rust semantics from `syn::parse_file`. Existing generated consumer tests own that dimension.
5. No arbitrary recursion, intentionally huge input, `catch_unwind`, stack overrides, or relaxed policy. A failing generated case becomes a minimized fixed regression only after diagnosing it.

### Scratch implementation mutants

After the positive focused suite passes, build one isolated scratch mutant at a time with identical toolchain/lock/features:

- **Occurrence accounting mutant:** deduplicate repeated targets when constructing the graph, leaving the original IR/input untouched. A repeated-edge expanded-work property must fail at an explicit expected/actual work assertion. Existing repeated-edge guard test is a useful independent control. Compile failure does not count as a killed mutant.
- **Boundary inclusivity mutant:** change central resource `value > limit` refusal to `value >= limit`. Exact-limit acceptance must fail at the named property assertion. Use bounded in-process guard tests only; no oversize render attempts are needed.

Retain patch/source/lock hashes, command, exact failing test/assertion and exit status. Restore nothing in the main checkout because production is never mutated. Report two targeted mutants and their outcomes; no general mutation score. Changing input bytes/graph edges is an input perturbation, not an implementation mutant.

### Gate and budget

After source freeze, Rust1.99 locked/offline -j4/incremental0 focused resource suite, all-target strict backend Clippy and scoped formatting. Actual Rust1.88 focused new tests if needed to establish their maintained MSRV. Use current leased warm target; copy/standalone mutant output must not share mutable compiler binaries. External process budget e.g.60s per prebuilt filtered suite, separate measured build timing; retain timeout as failure, not accepted refusal. No full fixture render, proofs or TS recapture for test-only changes.

Receipt records64cases, four literal seeds, generator/model hash, motif counts, maximum input/nodes/edges/unfolded occurrences, exact command/features/toolchains and successful/failing case counts. Named test counts and generated-case counts stay separate. A small replay selector may be supplied only if useful; default test run remains fixed and reproducible.

### Alternatives and limits

A coverage-guided libFuzzer harness would add deployment/infrastructure work without first fixing the missing graph-spec oracle. Hand-written additional chains would not cover varied topology. The proposed suite is intentionally narrow and cannot certify every IR variant, arbitrary stack safety, frontend resources or runtime security. The full R03017 requirement crosswalk and final candidate qualification remain open.


### Delivery — 2026-10-07

Signed commit `22e2689c`, 37 resource tests on Rust 1.99 and five new tests on Rust 1.88; both isolated mutants fail intended assertions.

[Seeded compiler properties and generated state assertions — 2026-10-07](references-0.3.0.md#note-168) contains archive, scope and limits. Parent acceptance remains separate.
