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

## Reviewed ADR221 and ADR222 additions

ADR221 resolves the seven named sampled-path gaps: Set/Map nonempty checks,
`call_arg.impureInIfArm(false)`, `struct_collision.runWrapBeta(false)`,
`chunked_ledger.ping(false)` and the true paths of ternary `streamCompareEq`
and `streamStructMember`. Two pure cases compare exact TS results. Five
stateful cases compare TS seeded state/effects, complete ordered public VM,
query cost sums, empty private outputs and replay. Signed correction `38702dd9`
adds exact full-program equality without normalization; earlier shape checks
remain as additional assertions.

ADR222 resolves the four named recorded-trace gaps with nine direct cases:
Map insert/replace/distinct keys, nested-map `ping` initial/repeat, witness
`pull` returning 42/0, and Set `check` at 7/8. These tests compare complete public
VM programs, private output atoms/alignment, TS state, query cost sums and
upstream replay. The witness ledger-view read is deliberately counted as a
query cost; it is not fabricated into a public VM observation. Constructor and
serialization queries are excluded from the retained per-call cost arrays.

The matrix records exact signed source IDs, capture/test paths, per-case
identities, dimensions and hashes for these additions. ADR216's ternary test
file hash is refreshed after verifying its assertions are unchanged by the
separate ADR221 addition. No other old case claims are upgraded.

## Remaining limits

The named gaps above are resolved only for their listed cases. This review does
not establish every input, branch, exception, transcript or proof path. Some
other tests still compare VM shape or Rust parity rather than a full independent
program. Cryptographic proof and ledger application remain separate evidence
not audited here; neither ADR221 nor ADR222 adds such a claim.

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
