---
id: RUST-ADR-0284
alias: ADR-0284
source_sha256: 6f79d1809d6bdc0ec45d344fd9a5266ecd503bb9548c4dfb477b8b7d45c1ea84
publication_status: published
publication_date: 2026-10-09
publication_source_revision: f9a496660a4a69f4dd7c7927111767ad5c0c12b2
---
# RUST-ADR-0284 — Measure lexical binding identity against simpler shared scopes

[ADR register](README.md) · [0.3.0 record register](index-0.3.0.md) · [Reference conventions](references-0.3.0.md)

## Publication disposition

This published historical source record belongs to the milestone 0.3.0 documentation collection. **Original opening status:** accepted for isolated research only, 2026-10-07. Parent R030-01/#345; R030-02/#346. Frozen production baseline `1f9b13944c1ac06d983e952cb1a130750e2a9ee1`. No production promotion is authorized by this research decision. Read dated amendments below; neither this status nor issue closure certifies the whole milestone. No fresh implementation, security, proof or formal acceptance is implied. Original code examples and local evidence locators are historical; machine-local paths are not portable downloads or current setup instructions.



## Original decision and amendments

## ADR0284 — Measure lexical binding identity against simpler shared scopes

Status: accepted for isolated research only, 2026-10-07. Parent R030-01/#345; R030-02/#346. Frozen production baseline `1f9b13944c1ac06d983e952cb1a130750e2a9ee1`. No production promotion is authorized by this research decision.

### Problem

The milestone asks for explicit binding/circuit IDs where a probe demonstrates value. The recorded Plan currently carries typed operands (`TypedValue { ty, value: syn::Expr }`) in cloned lexical name maps. Callee arguments are evaluated in caller order, then bound into an isolated callee scope; local lets evaluate once before extending a cloned scope. Checked composition already retains concrete declaration references for helper kinds. These are real boundaries, not evidence of a proven name-based scope defect. Adding IDs alone must not be presented as fixing an unobserved bug.

### Alternatives and concrete experiment

Compare three frozen actual-emitter variants with identical semantics:

```rust
// Existing: each cloned lexical map clones its typed syntax values.
type Scope = HashMap<String, TypedValue>;
// Ordinary Rust candidate: share immutable typed values across cloned maps.
struct Scope(HashMap<String, Rc<TypedValue>>);
// Identity candidate: each scope owns a name-to-slot index and shared values.
struct BindingId(usize);
struct Scope { names: HashMap<String, BindingId>, values: Vec<Rc<TypedValue>> }
```

Both candidates expose only new/from-iterator/get/insert operations used by the current Plan. IDs remain private and scoped to their owning arena. A slot does not become a globally valid declaration identity; no claim of compile-time cross-scope protection. Retain both candidates, even if measurements favor neither. Do not change source IR, public API, runtime, ABI, schema, expression evaluation or helper call policy. This experiment does not introduce a second evaluator or resolved-IR production stage.

Measure direct ownership/code complexity and behavior on actual checked-in typed IR, including lexical returns, repeated/helper composition, witnessed and funded control cases and original DID/passport. Run selected existing semantic tests in both scratch emitters and compare complete Rust/capability results with an immutable baseline. Record paired alternating render timing with sample ranges; no statistically unsupported speed claim. Isolate default worker and build configuration; exact root dependency identities must be retained. Synthetic shadow-heavy scope measurements, if used, are labeled separately from actual rendering. Negative controls must preserve lexical escape/type/call refusal and side-effect order; output equality establishes unchanged emitted behavior but is not a new proof run.

### Decision criteria

Choose the smallest representation with demonstrated maintenance or resource benefit. If IDs add only bookkeeping and no consumer requires stable resolved references, defer them and document that decision. A future cross-pass resolved representation may justify declaration/binding IDs with explicit provenance and scope ownership, but would require its own ADR and misuse proof. If shared immutable values materially help, propose a separate production decision with measurements and admission regression evidence. Neither experimental shorter code nor a tiny timing change requires adoption.

### Delivery

Keep scratch source/patch, exact hashes, commands/results, failed attempts and real before/after snippets in midnight Obsidian. Research closure does not close R030-01; representative checked profile admission remains a separate requirement, coordinated after ADR0283 source ownership is released. No remote CI/push.

Tracking: https://github.com/MediaNoxLabs/compact/issues/408

### Delivered research decision — 2026-10-07

Retain existing typed lexical maps and checked declaration references. Both actual emitter prototypes preserve43 successful renderings/22 historical errors and pass23 focused tests each; timing/RSS do not establish meaningful improvement. No production promotion. [ADR0284 — Lexical scope and binding identity decision](references-0.3.0.md#note-065) contains before/after forms, exact limitations and hash-verified evidence. R030-01 remains open for representative checked admission.
