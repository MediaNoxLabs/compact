# Source-generated nested Map membership

The Compact source declares `Map<Field, Map<Field, Uint<64>>>` and returns named `Present` from key-only membership. The generated slot's `MapNode` marker has no scalar `CellValue` codec; membership only requires the key codec.

One test covers four exact same-source TS cases: two keys against the generated empty ledger, and two keys against an explicitly supplied official ledger state containing one outer key and an empty inner Map. The populated state uses official `StateValue` and storage Map types. It is not a generated seeded constructor or support for nested `insertDefault`, lookup, or insertion.

Each case checks native/recorded/observed/manual execution, verify-op replay, state/effects/gas, empty private transcript, and prepared Field/Boolean FAB bindings. `oracle/capture.mjs` independently constructs the equivalent state using the official TS `StateMap`/`StateValue` API and records complete source/runtime provenance. No proof generation or application is claimed here.
