---
id: RUST-ADR-0286
alias: ADR-0286
source_sha256: 22080c63650eab306f15e6ac88994aa43e385a5e5a96c0a16c180319e14b25bc
publication_status: published
publication_date: 2026-10-09
publication_source_revision: f9a496660a4a69f4dd7c7927111767ad5c0c12b2
---
# RUST-ADR-0286 — Classify checked Unit composition attempts

[ADR register](README.md) · [0.3.0 record register](index-0.3.0.md) · [Reference conventions](references-0.3.0.md)

## Publication disposition

This published historical source record belongs to the milestone 0.3.0 documentation collection. **Original opening status:** accepted bounded design and preparation, 2026-10-07. **Production implementation and builds are held until root explicitly releases the ADR0283 compiler/runtime/DID/proof freeze.** Parent R030-01/#345; milestone `rust-backend-v0.3.0` (3). This record precedes implementation. ADR0284's disposition retains current lexical maps and declaration references; this slice does not adopt binding IDs. Read dated amendments below; neither this status nor issue closure certifies the whole milestone. No fresh implementation, security, proof or formal acceptance is implied. Original code examples and local evidence locators are historical; machine-local paths are not portable downloads or current setup instructions.



## Original decision and amendments

## ADR0286 — Classify checked Unit composition attempts

Status: accepted bounded design and preparation, 2026-10-07. **Production implementation and builds are held until root explicitly releases the ADR0283 compiler/runtime/DID/proof freeze.** Parent R030-01/#345; milestone `rust-backend-v0.3.0` (3). This record precedes implementation. ADR0284's disposition retains current lexical maps and declaration references; this slice does not adopt binding IDs.

### Problem and evidence

The shared Unit composition owner currently collapses signature mismatch, whole-graph policy refusal, missing public/helper requirements and typed lowering failure into `None`. This obscures why a bounded domain declined an otherwise valid native circuit. It does not establish a current unsafe fallback or partial-output bug.

Frozen research at `1f9b13944c1ac06d983e952cb1a130750e2a9ee1` inspected `tools/compact-rust-backend/src/recorded/typed_plan/composition.rs:462–569`, the recursive audit at `:129–456`, shared Plan call dispatch in `typed_plan.rs:1138–1197`, and router selection in `recorded.rs:6407–6602`. ADR0266 already provides `ProfileAttempt::{NotApplicable, Rejected, Admitted}` for shielded-send; other profiles deliberately remain independent. Native rendering occurs before recording selection (`lib.rs:2993–3025`) and must retain its error precedence.

Before coding, re-read ADR0283's final committed composition delta and capture its immutable baseline. The older research snapshot must not overwrite its newly delivered collection leaves.

### Decision: one existing owner, three explicit outcomes

Migrate only Unit composition to a checked attempt, reusing its existing recursive audit, declaration classification and shared typed Plan. Preserve profile order, admission policy and completed generated output.

- **NotApplicable:** the current entry signature filter fails: non-Unit result, non-Unit StateReturn, or a parameter outside the existing value domain. This is a deliberately coarse candidacy boundary. Pure exports do not enter this stateful router.
- **Rejected:** that signature matched, but complete graph auditing, required public/helper composition, duplicate binding or typed Plan lowering failed. It rejects this bounded domain, not the IR globally. A candidate with public reads but no required Unit helper must not be inaccurately described as having no recorded effect.
- **Admitted:** the entire audit completed, all final domain obligations held, and every action plus Unit result lowered through the existing Plan. Only a complete plan can escape. Rejected statements, call caches, counters and partial results are discarded.

Existing located native RenderErrors retain precedence and are never rewritten as recording gaps. Remaining Option-based typed Plan failures are attributed to the enclosing action and lowering phase; do not invent a precise nested type cause.

#### Typed private provenance resolution

Use the small existing enum with a default error type:

```rust
// Private compiler API; illustrative until implementation is released.
enum ProfileAttempt<T, E = RecordingGap> {
    NotApplicable,
    Rejected(E),
    Admitted(T),
}
struct CompositionRejection {
    gap: RecordingGap,
    root_action: Option<usize>,
    precision: RejectionPrecision,
    phase: RejectionPhase,
}
enum RejectionPrecision { ConcreteNode, EnclosingAction, Obligation }
enum RejectionPhase { PolicyAudit, TypedLowering, FinalObligation }
```

Shielded-send retains its default `RecordingGap` payload and existing semantics. Unit composition carries the additional private metadata. Do not build a universal strategy registry, change capability schema, or make human diagnostic text control flow. The enum's Option adapter may remain for compatibility, but the checked router must retain refusals explicitly rather than discard them through that adapter.

Thread location through the existing audit traversal: root action ordinal, declaration kind/name, callee chain, and child segments for bindings, branches and arguments. Preserve source names as data with deterministic escaping when rendered. Declaration references already held by `AuditedCall` remain authoritative. No new evaluator or lexical identity system is introduced.

Concrete audit failures retain the inspected expression/action and callee path. Cycles retain the call site and active chain. Shared Plan failures initially retain only the enclosing action and `TypedLowering` phase. A successful callee cache must not cache invocation-specific failure paths. Audit traversal order is deterministic validation order; it is not a claim that an unselected branch executes at runtime.

### Domain rejection and deliberate fallback

Every attempt receives original IR/declarations and owns fresh scratch state. A later distinct profile may accept only by independently producing a complete plan from that original input. It cannot reuse rejected steps/counters/caches or retry the same policy with guards removed.

Preserve `audited_local::unit_helper` → RecordedUnit classification: refusal of a public read at the narrower native-local boundary is expected when the same helper can be fully audited and recorded on the shared frame. Refusing that first alternative is not itself a composition rejection. Pure-only and read-only Boolean restrictions remain intact; hidden crypto/public mixtures that fit neither route remain rejected.

Diagnostic precedence:

1. Existing native/located errors win.
2. A complete accepted profile or independently complete legacy lowering wins over deferred domain refusals.
3. Earlier concrete action/type diagnostics remain primary when no full plan succeeds.
4. A concrete nested composition audit failure may refine a coarse legacy UnsupportedAction only at the **same typed root action ordinal**. Earlier failures, concrete type/expression diagnoses and nonconcrete obligation/lowering refusals retain their previous public gap.
5. No path-prefix sniffing, parsing detail strings, or replacing a real native error with a more attractive profile message. Keep the internal outcome tests separate from public diagnostic tests: an internal refusal need not always change the capability report.

Retain a composition refusal locally beside the existing send refusal. Avoid a broad router refactor.

### Before and after

Before: `composition::lower(...) -> Option<TypedPlan>` discards audit reasons; the outer router sees only success/absence.

After: `lower_unit_composition_checked(...) -> ProfileAttempt<TypedPlan, CompositionRejection>` returns only a complete plan, an unrelated signature, or a typed domain refusal. The outer router still tries complete distinct profiles in the same order and can retain precise eligible diagnostic evidence. Runtime/emitter evaluation primitives, source semantics, public generated APIs, ABI and IR/capability schemas are unchanged.

### Required tests and acceptance

- Direct NotApplicable controls for signature misses; matched rejection controls for graph/domain/final obligations; Admitted flat/chunked controls.
- Existing pure Unit guards: local/transitive bindings, wrong scope/type, unused and unselected hidden effects, arity and cycles. Assert native RenderError versus genuine recording refusal explicitly; existing bool-only tests conflate these outcomes.
- Existing read-only/helper controls: local-audit refusal followed by valid RecordedUnit admission, bounded Boolean Map helper restrictions and pure-only cache restrictions.
- Existing witnessed crypto and repeated-helper controls: exact call/argument/witness order, distinct bindings, hidden effects and caller/callee provenance. Preserve current native/recorded/replay assertions.
- Existing funded shielded-send checked tests keep their exact rejection/precedence behavior. Test a Unit composition refusal followed by independently complete zswap admission using a suitable existing valid Unit intent helper/control. Prove the actual profile pair directly; public rendering may select an earlier valid profile, which is not evidence that the later edge was reached.
- Successful prefix followed by a rejected suffix must emit no admitted partial plan. A scratch-only mutant returning the prefix must fail this control; never modify production with the mutant.
- Exact source path tests for unused binding, unselected branch, nested argument and transitive callee; first earlier-action and native-error precedence tests. Enclosing typed-lowering locations must be described honestly.
- Focused backend composition/send suites and strict backend Clippy; relevant generated Unit/repeated-helper/DID behavioral tests and existing positive Unit source manifest after ADR0283 is frozen.
- One immutable full corpus before/after comparison, coordinated with root: all previously admitted generated source and capability results remain identical; intentional unsupported diagnostic refinements are individually listed with old/new code/path/detail. No unexplained gain/loss or regenerated body accepted silently.
- No new proof requirement for a diagnostics-only migration when admitted bytes remain identical. Root may select a current DID strict smoke in the next integrated gate. Changed admitted bodies are a regression to investigate, not permission to refresh keys.

### Ownership and exclusions

Prepared ownership after explicit freeze release: `recorded/typed_plan/composition.rs`; surgical `typed_plan.rs` checked wrapper; `recorded/profile_attempt.rs` generic private payload/documentation; surgical `recorded.rs` router/typed provenance use; focused checked-composition tests. Runtime, CI, public switches, funding policy, source constructors, lexical representation and unrelated profiles are out of scope. No builds, implementation, commits or push are authorized during the present freeze.

### Retained research

Research proposal `/tmp/rust030-checked-unit-plan/PROPOSAL.md`, SHA256 `3005384cd1c491bec6cb254719609d50f00ac7a2fa85d64f7d815540a38a6f67`; receipt SHA256 `14639c378bd93c7c0261a15f9d937173ca00e403f25766aee7c75257cad531f4`. The design above resolves private typed provenance without requiring IDs or another evaluator. Parent milestone acceptance remains separate from this bounded delivery.



Tracking: https://github.com/MediaNoxLabs/compact/issues/410 (milestone 3). Preparation only; waiting for root’s explicit ADR0283 freeze release before implementation or builds.
