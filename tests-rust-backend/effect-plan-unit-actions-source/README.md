# Unit action admission expansion

Exact source: `examples/rust_backend/effect_plan_unit_actions.compact`.
`build.rs` uses its retained schema-20 IR and the current default backend.
This is additional admission coverage, separate from old-first family inclusion
and from the original Coracle source proofs.

Sixteen declared Field Cells produce the compiler's real nested layout:
`stored` is `[0,0]`; the final padding Cell is `[1,14]`. `verify` calls a read-only
Field-to-Unit helper without a write. `no_helper` directly calls a zero-argument,
Unit-result witness with no stateful helper and no public operation. These cross
the removed flat-path, write-count and helper-count gates without synthetic IR.

Runtime controls compare native and recorded effects/state/gas against an
independent official-slot query program, preserve assertion and witness failures,
and check private-state/output ordering. The empty recording refuses preparation
with `EmptyTranscript`; its test key carrier is only for that early refusal.
No dummy VM action is added. This fixture has no proof claim.
