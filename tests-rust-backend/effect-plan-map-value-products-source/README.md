# Guarded Map values with products

The complete genuine `effect_plan_map_value_products.compact` source stores a named value containing `Vector<2, Field>`, `(Field, Boolean)`, `Uint<129>` and a `Uint<64>` timestamp. Its constructor supplies one Map entry; `readFresh` performs membership/assertion, lookup, then a pure timestamp assertion.

Before the prerequisite, authenticated frozen #593 admitted this exact whole source through `guard.guarded-map-read` with recorded/observed APIs, while checked composition refused its Map value type. Default ZKIRv2 Rust+TS frontend generation succeeded. Full source and IR receipts are retained under `compact-map-value-prerequisite/combined`; no generated Rust is rewritten.

## Focused runtime gates

With `ledger-transaction`, two tests cover:

- Four stored wide values (zero, u128 maximum, 2^128+9, and Uint129 maximum), each read at equality and above the timestamp: eight successes.
- Exact literal stored FAB atoms and official CellValue roundtrip preserve the vector, tuple, Boolean and wide field.
- Native/recorded/observed/manual execution, full verifier program, state/effects/execution gas, unchanged private state, empty private FAB, independently replayed gas and exact prepared key/time inputs plus Unit output.
- Missing key and future timestamp: identical typed refusals in all four lanes. Consumed error contexts are not a rollback claim.

This is genuine-source Rust/manual qualification, not a newly captured TS execution oracle. Separate raw generated codec controls cover empty Struct, tuple0/8 and other storage-domain boundaries; these are not attributed to this source.

`--map-value-products` selects the original `readFresh("map-λ",100)` call over a constructor-supplied record with fields `[3,5]`, pair `(7,true)`, and wide `2^128+9`. The proof adapter guards the entire IR before keys, compares manual/native/recorded/observed/replay and exact stored value, verifies the proof and changed-binding refusal, then performs default-strict funded application. Constructor seed is supplied; deployment is not proved. Parent execution receipts establish actual proof results.
