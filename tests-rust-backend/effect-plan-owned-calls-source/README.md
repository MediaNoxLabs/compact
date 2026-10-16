# Owned internal stateful interfaces

The build script renders the complete retained schema20 IR for
`effect_plan_owned_calls.compact` with the current backend. Its public entry is
`run(Boolean, Boolean) -> Field`; no public or witness-parameter policy is widened.
The only witness results are existing Field and OpaqueBytes representations.

Internal stateful helpers pass and return String, OpaqueBytes, Vector2Field,
(Field, Boolean), Uint160 and a struct containing all five. Each helper writes a
marker and invokes a zero-argument trace witness, so these are real stateful calls.
The source exercises left-to-right call arguments, nested calls, sequential calls,
owned aliases, selected branches and a helper assertion after its callback.

Tests compare native and recorded execution with an independently assembled
RecordingFrame using official runtime slots. Checks include exactly-once callback
order and observations, owned value results, private state/FAB outputs, state,
effects, gas, full public transcript and replay. Each callback position has a
refusal case, as does the selected helper assertion. String values are empty
because no new String witness domain is introduced; opaque byte payloads vary.

This is local runtime qualification, without a proof or network claim.
Run `cargo test -p compact-rust-effect-plan-owned-calls-source-fixture`.

The empty String is projected from `default<Owned>`, an already supported struct
operation. A direct `default<Opaque<"string">>` inside the stateful helper compiled
natively but was refused by the separate recording default-operation policy;
this fixture keeps that unrelated domain unchanged. Initial source/IR evidence
is retained in the local delivery evidence under `default-gap/`.

The root returns Field one/zero from its comparisons to fit the existing terminal
selector rollout policy. The checked algebra accepts the earlier Boolean root,
but that complex shape is not publicly routed yet; that variant is retained in
`bool-root-boundary/` evidence. This fixture does not widen public routing.
