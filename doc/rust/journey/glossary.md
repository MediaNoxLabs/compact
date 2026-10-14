# Glossary

**Compact**
Midnight's smart-contract language and compiler pipeline.

**Chez Scheme**
The implementation language of the Compact frontend and compiler passes used
here. It remains responsible for parsing, typing and shared analysis.

**Rust-AST backend**
The target that consumes a private typed IR and constructs Rust syntax through
`syn`, `quote`, `proc-macro2` and `prettyplease`.

**Private IR**
The versioned JSON interchange between Chez and the Rust renderer. Its schema
version is internal; it is not a Compact language specification version.

**FAB / aligned value**
The field-aligned binary/value representation used across Compact values,
witness transcripts and ledger/proof boundaries.

**Native execution**
Running a generated circuit against the Rust runtime to obtain a result and
updated context. It does not imply recording or proof support.

**Recording**
Producing the ordered public VM/effect program and private/public values needed
for replay and later proof-oriented integration.

**Replay**
Checking a recorded program against observed state and runtime invariants.

**Preparation**
Constructing a ledger call/prototype from recorded execution, observed state,
verifier material and caller-supplied policy.

**Proof applicability**
Whether a circuit and its exact artifacts have the required public/private
inputs and policy to be proved. A generated recording API is not enough.

**Shape recognizer / recording profile**
A specialized admission path that recognizes a known whole-circuit pattern.
It was useful for early coverage but did not compose naturally.

**Recording effect algebra**
The prototype's typed vocabulary and composition rules for building a complete
`RecordedPlan` from checked operations.

**Typed refusal / `RecordingGap`**
A structured explanation that native code may exist while a complete recorded
path does not. It carries a stable code, node, structural path and detail.

**Oracle**
An independent implementation or captured output used for comparison. Oracles
are evidence and can themselves contain bugs.

**Old-first law**
A migration check that asks the legacy implementation first, then ensures the
new path preserves accepted/refused behavior and takes ownership deliberately.

**CoPS**
Compact problem statements that record cross-component issues without
pretending a local workaround is an upstream specification decision.

**ADR**
Architecture Decision Record: problem, decision, alternatives, ownership,
evidence and limitations at a named point in history.
