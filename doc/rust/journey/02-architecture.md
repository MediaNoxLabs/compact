# Architecture: one frontend, explicit boundaries

## The current conceptual pipeline

```text
Compact source
    │
    ▼
Chez Scheme parser, type checker and shared analysis
    │  Lnodisclose + source-owner metadata
    ▼
private versioned Rust IR (JSON)
    │  strict serde model + resource preflight
    ├──────────────────────────────┐
    ▼                              ▼
native semantic fold         recording effect lowering
    │                              │
    │                        checked RecordedPlan
    └──────────────┬───────────────┘
                   ▼
          syn / quote Rust AST
                   │
             prettyplease
                   ▼
      generated Rust crate + manifest
                   │
      midnight-compact-runtime
                   │
 official ledger-8 VM / state / FAB / crypto / ZK
```

“IR schema 20” in the closed Rust-AST milestones means the twentieth revision
of this private Chez-to-Rust interchange. It is not Compact language version
20 and not a universally recognized specification level.

## Why syntax trees mattered

`syn` does not prove that generated code is semantically correct. What it does
is move an important class of failure earlier: output is constructed as Rust
syntax rather than assembled as strings. `quote` and `proc-macro2` make larger
fragments composable; `prettyplease` makes the resulting syntax readable and
deterministic.

The stronger guarantee comes from the surrounding discipline:

- the IR enums are closed and reject unknown fields;
- types, actions and expressions have explicit variants;
- resource preflight bounds depth, nodes and call expansion;
- malformed IR is an error, while unsupported recording is a typed capability
  outcome;
- generated fixtures compile and execute; snapshots alone are not acceptance.

## Four execution layers that must not be conflated

| Layer | Question | Owner |
| --- | --- | --- |
| Native execution | Can the typed Compact operation run and return the right state/value? | Generated code + native runtime |
| Recording | Can execution produce a complete ordered public VM/FAB trace? | Checked recording plan + runtime frame |
| Preparation and replay | Does that trace replay against observed state and the selected verifier? | Runtime transaction adapter |
| Proof/application | Can the exact artifacts prove, verify and apply under the pinned ledger policy? | Prover, verifier and ledger |

A native method can exist without recording. A recorded method can exist when
proof material is absent. A successful proof preparation is not a submitted or
finalized network transaction. The documentation and capability reports keep
these distinctions because collapsing them created misleading coverage claims
early in the project.

## Before: shape recognition

```text
Stateful circuit
  ├─ is this a voting shape?
  ├─ is this a guarded optional write?
  ├─ is this a shielded payout?
  ├─ is this funded mint?
  └─ otherwise: legacy fallback or structured refusal
```

This model is safe when every admitted shape is tightly tested, but its reuse
unit is too large. Supporting `MapLookup`, a witness and an assertion in three
separate profiles does not prove that a fourth arrangement composes correctly.

## After: checked effects

```text
Stateful circuit
  │
  ├─ resolve declarations and lexical identities
  ├─ type each value and operation
  ├─ compose ordered effects and returns
  ├─ validate resource, scope and replay invariants
  └─ render only the complete checked plan
```

The effect vocabulary is intentionally not a new ledger. It references runtime
operations whose semantics remain owned by the pinned Midnight primitives.
The plan owns compiler facts: types, source order, bindings, effects, result
roles, admissibility and the precise reason a supported native operation is not
recordable.

## Source provenance: current truth

Published Rust-AST work carries source-owner file, line and column information
through the private IR and uses structural paths in typed recording gaps. At
the prototype cutoff, the accepted closure plan still describes these as
declaration locations—not arbitrary nested expression ranges.

Original occurrence tracking and authenticated source views prevent tests from
quietly replacing genuine frontend artifacts with convenient hand-built JSON.
The separate transient-value origin proposal remains a proposal unless a later
qualified candidate demonstrates it. Publication must never expose absolute
checkout paths, secrets, witness values or private proof material.

## Generated API and runtime ownership

Generated crates expose typed Compact values, witnesses, native entry points,
recorded calls where available, and transaction preparation helpers under the
matching runtime ABI. The runtime owns reusable representation and integration
mechanics: bounded integers, fixed bytes/vectors, ledger ADT access, FAB
alignment, VM programs, recording frames, verifier decoding and optional
transaction construction.

That ownership is a trade-off. Generated code becomes smaller and more regular;
the runtime becomes a safety-critical compatibility layer. This is why runtime
ABI, dependency graph and exact ledger/proof versions are tested independently
from the compiler IR schema.
