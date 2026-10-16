# Vector-map composition controls

This is a synthetic complete-IR consumer, not a newly compiled Compact contract or a TypeScript oracle. `original-ir.json` preserves the exact full source IR for retained corpus entry 096 (`map_fn_oracle.compact`). `build/synthetic.rs` appends five roots and two pure helpers; it does not replace the original `ping`, constructor, or ledger layout. The build writes both the derived full IR and generated Rust to OUT_DIR.

- `products`: nonidentity Field arithmetic, a captured offset, tuple members and named-product/vector results; the map formal shadows the outer parameter only within the body.
- `nested`: nested maps distinguish inner formals, captured outer elements and outer parameters.
- `empty`: an explicitly typed zero-length map returns no elements and never executes its fallible pure body. Compiler tests separately enforce validation of an empty map's body.
- `fallible`: a genuine fallible pure source helper precedes mapping; distinct body failures establish left-to-right first-error behavior. Since a pure helper has no legal side effects, runtime behavior alone does not prove invocation count; a separate generated-AST check must bind the single source call.
- `witnessed`: one outer witness supplies a captured value to three iterations; exact private output, callback prefix and provider refusal are checked. The witness is outside the map's pure body.

These roots produce no public ledger program. Native/recorded/observed/manual results, state, effects, private outputs and gas agree, while preparation retains the typed `EmptyTranscript` refusal. The serialized empty verifier carrier only reaches that policy boundary; it is not proof material. The original source `ping` has a separate public-write preparation test and retains the existing immutable TypeScript oracle tests.
