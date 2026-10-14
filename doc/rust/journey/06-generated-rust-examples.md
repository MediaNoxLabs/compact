# Generated Rust: follow one contract end to end

This example uses the checked-in ledger-8 `tiny_oracle` source and generated
Rust at the Rust-AST documentation head
[`46fdf089`](https://github.com/MediaNoxLabs/compact/commit/46fdf0895a012a50250d9b8f3c0b49e49de48b00).
The excerpts are shortened for explanation; links point to the complete source.

## 1. Compact declares meaning

[`examples/rust_backend/tiny_oracle.compact`](../../../examples/rust_backend/tiny_oracle.compact)
declares a small state machine:

```compact
enum STATE { unset, set }

ledger authority: Bytes<32>;
export ledger value: Field;
ledger state: STATE;

witness private$secret_key(): Bytes<32>;

circuit set(v: Field): [] {
  assert(in_state(STATE.unset), "set: attempted to overwrite recorded value");
  const sk = private$secret_key();
  authority = public_key(sk);
  value = disclose(v);
  state = STATE.set;
}

circuit get(): Maybe<Field> {
  return in_state(STATE.set) ? some<Field>(value) : none<Field>();
}
```

Compact owns source typing, disclosure rules, witness declarations, circuit
control flow and ledger declarations. None of those rules should be inferred
from a Rust string template.

## 2. Generated types use mechanical derives

The generated library
[`tests-rust-backend/tiny-oracle/lib.rs`](../../../tests-rust-backend/tiny-oracle/lib.rs)
turns the source enum and product into ordinary Rust types:

```rust
#[derive(Clone, Debug, Default, PartialEq, Eq,
         runtime::CompactCellValue, runtime::CompactStructRepr)]
pub struct Maybe {
    pub is_some: bool,
    pub value: runtime::Field,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq,
         runtime::CompactCellValue, runtime::CompactEnum)]
pub enum STATE { unset, set }
```

The macros implement only mechanical representation rules: FAB alignment,
Cell conversion, field/binary representation and checked enum ordinals. They
do not interpret circuit bodies.

## 3. Ledger declarations become typed slots

```rust
pub const authority: runtime::slots::CellSlot<runtime::FixedBytes<32>> =
    runtime::slots::CellSlot::new(&[0]);
pub const value: runtime::slots::CellSlot<runtime::Field> =
    runtime::slots::CellSlot::new(&[1]);
pub const state: runtime::slots::CellSlot<crate::types::STATE> =
    runtime::slots::CellSlot::new(&[2]);
```

A slot binds the source declaration's value type to its physical ledger path.
Generated code asks the slot to read, write, inspect or record. The runtime
owns the canonical ledger-8 VM program for that operation.

## 4. Native Rust makes order and cost visible

The generated `set` function is ordinary fallible Rust. A simplified excerpt:

```rust
let prior = in_state(context, STATE::unset)?;
context = prior.context;
total_cost += prior.gas_cost;
if !prior.result {
    return Err(runtime::CompactError::AssertionFailed(
        "set: attempted to overwrite recorded value".to_owned(),
    ));
}

let meter = runtime::context::WitnessReadMeter::new(&context);
let (next_private, sk) = witnesses.private_secret_key(
    context.witness_context_with(LedgerView {
        state: context.query.state.get_ref(),
        meter: &meter,
    }),
)?;
total_cost += meter.gas_cost();
context.private_state = next_private;

let step = ledger_slots::authority.write(context, public_key(sk)?)?;
let context = step.context;
total_cost += step.gas_cost;
```

The successor context is consumed and returned. Query costs are accumulated in
source order. Witness ledger reads are metered without making private witness
values part of the public verification program.

## 5. Recording uses the same domain operations

Where the complete circuit is admitted, the generator also emits a recorded
path:

```rust
let frame = runtime::recording::RecordingFrame::new(context);
let (frame, prior) = ledger_slots::state.record_read(frame)?;
if prior != STATE::unset {
    return Err(runtime::CompactError::AssertionFailed(
        "set: attempted to overwrite recorded value".to_owned(),
    ));
}
let (frame, sk) = frame.try_witness_metered(|context, meter| {
    witnesses.private_secret_key(
        context.witness_context_with(LedgerView {
            state: context.query.state.get_ref(), meter,
        }),
    )
})?;
let frame = ledger_slots::authority.record_write(frame, public_key(sk)?)?;
let frame = ledger_slots::value.record_write(frame, v)?;
let frame = ledger_slots::state.record_write(frame, STATE::set)?;
Ok(frame.finish(()))
```

The native execution result and replayable public trace remain distinct inside
`RecordedCircuitResult`. Recording support is emitted only when every required
operation has a checked recorded form.

## 6. The consumer sees a smaller facade

The generated crate exposes native methods, a borrowed recording handle when
witnesses are needed, and—with the transaction feature—typed observed-call
builders. The consumer supplies private state, witnesses, observed public
state and exact verifier artifacts; generated code preserves the Compact entry
point and typed public input.

```rust
let native = contract.set(context, value)?;
let recorded = contract.recording().set(context, value)?;
let call = contract.recording().set_call(&observed, private, value)?;
let prototype = call.prepare(verifier, commitment_randomness)?;
```

These three lines do not make equivalent claims. The first executes natively,
the second also produces a trace, and the third binds that trace to an observed
contract and verifier before ledger preparation.

## What the generated code demonstrates

- Compact meaning is lowered before Rust syntax construction.
- Generated Rust is inspectable and uses ordinary `Result` propagation.
- Runtime slots own ledger mechanics; generated functions own source order.
- Private witness outputs and public verifying operations are separate.
- Gas is observed from executed queries rather than guessed in the emitter.
- A native method can exist without a recorded or proof-capable method.
