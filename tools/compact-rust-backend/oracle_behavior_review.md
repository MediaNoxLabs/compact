# Pinned 37-oracle manual behavior review

The manual source inventory pass covers all 37 pinned sources and classifies
203 candidate rows. Two rows are constructor-only sources with no exported
circuit. The current reviewed source/test/capture contents are pinned in
`oracle_behavior_review.json`; ADR216, ADR218 and ADR220 also have separate
case-level matrices. Source/test changes require review before updating hashes.

This is not complete behavioral coverage. Cases are sampled, many dimensions
are partial, and cryptographic proof/ledger suites were not audited here.
The JSON records these limits independently from the completed inventory pass.
All evidence paths in the JSON are repository-relative.

## Direct-boundary gaps resolved by this delivery series

- ADR216 adds actual `assert_parity.ping` and `walkerVectorElement` execution,
  plus direct independent cases for all 43 literal pure exports (91 cases).
- ADR218 directly captures all 25 ternary pure exports (83 cases, including
  eight exact expected failures).
- ADR220 adds direct independent cases for six call-argument pure exports and
  three AssetRegistry guards previously covered only transitively. It also adds
  a direct TS result for `sumVec`, whose existing direct Rust call was used as
  a reference for a different function. These are 30 pure cases across ten APIs.
- ADR220 adds independent `AssetRegistry.close` success and repeated-close
  refusal. Existing native/recorded/replay tests already executed this API;
  the missing boundary was its independent TS capture.

## Remaining sampled-path and trace limits

- Set/Map emptiness exports currently see only empty collections.
- `call_arg.impureInIfArm` takes only true in reviewed captures.
- `struct_collision.runWrapBeta` and `chunked_ledger.ping` see only true.
- Ternary `streamCompareEq` and `streamStructMember` specialized captures see
  only false. These are sampled-path limits, not missing exported invocation.
- Reviewed fixture tests for `map.put`, `nested_map.ping`, `witnesses.pull` and
  Set `check(8)` lack full independent recorded transcript/gas comparisons.
  Recorded `check(7)` exists; separate nested MapSlot tests are not exported
  `ping` recording evidence. Additional proof harnesses may contain evidence,
  but none was inferred without review.

## Review rules and current classifications

Six mixed-width comparison functions genuinely execute through typed function
pointers, with two independent captured outcomes each. All ten AssetRegistry
stateful exports execute; writable insert/update/refusal cases have substantial
state/private/gas/replay evidence. Constructor expressions are not treated as
calls to arbitrary exported APIs, even when their expressions look alike.

The current matrix has 114 direct native outcome rows against TS, 45 ADR216
case-matrix rows, 25 ADR218 case-matrix rows, 11 ADR220 case-matrix rows, six direct
function-pointer rows and two constructor-only rows. These categories describe
the assertion boundary, not all-input, all-branch, proof or ledger acceptance.
The prior direct/transitive distinctions remain recorded on ADR220 rows.
