# Frontend ordered-return regression sources

`frontend_return_plan_gate.py` compiles these sources with two explicit Scheme
frontends and one identical Rust backend. It checks fresh IR, native compilation
and reported recording capabilities. Runtime and proof receipts are separate.

- `storage_mutations`: nested branches and lexical bindings; storage update,
  remove and coin-check classes. These use existing StateAction emitters. Native
  compilation does not imply algebra support for Set or coin operations.
- `value_effects`: assertions, calls, witness and storage reads keep their existing
  expression representation and recording availability.
- `checked_subtraction`: a compiler-inserted subtraction guard remains part of
  one checked arithmetic expression after promotion to an ordered return plan.
- `examples/rust_backend/effect_plan_counter_only.compact`: the source-bound
  Counter runtime/proof fixture has exactly two Counter fields and no extra Cell.

Example (supply actual local tool paths):

```sh
python3 -B tools/compact-rust-backend/frontend_return_plan_gate.py \
  --compiler /path/to/compactc \
  --scheme /path/to/candidate-compactc-scheme \
  --baseline-scheme /path/to/baseline-compactc-scheme \
  --output /tmp/frontend-return-gate \
  --source-manifest /path/to/retained-source-census.json
```

The optional census reads `rows[].source`; it recompiles each source and never
substitutes previously retained IR. Each baseline must succeed. Accepted native
compilation and recorded/observed-call availability must not decrease.

## Explicit remaining boundary

Tracked in [#516](https://github.com/MediaNoxLabs/compact/issues/516) and CoPS-003
in the midnight knowledge base.

`cast_wrapped_mutation.compact` is a retained **unsupported** source, outside the
positive gate. Inferred integer branch results make the frontend place a
`safe-cast Field Uint<1>` around the whole conditional; current terminal-return
traversal does not cross that value wrapper. Explicit branch-local Field casts
(as in `storage_mutations`) avoid it, but the inferred form needs a separate
semantic normalization change. Do not count the annotated case as coverage of
both forms.

The existing Cell-specific `let-scope-needs-action?` and mutation-bearing let
initializers are also outside this slice. This change removes ADT identity from
the terminal storage-mutation classifier, not every frontend shape dependency.
