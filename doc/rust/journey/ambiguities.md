# Unresolved ambiguities and missing evidence

This list is intentionally part of the publication packet.

1. **Moving prototype state.** The main prototype checkout had uncommitted work
   after `f3cef799` while this story was authored. It is not described as
   delivered. Refresh only from a new signed checkpoint and its EA13 receipts.
2. **Nested source ranges.** Current accepted planning still describes
   declaration `file/line/column` plus structural paths. Later source-map work
   may exist in the moving checkout, but no complete qualified nested-range
   claim is made here.
3. **Transient provenance.** ADR/CoPS work proposes tracking transient-derived
   persistence origins. Treat it as proposal/follow-up until a qualified
   candidate demonstrates the full path.
4. **Historical oracle heads.** The initial architecture comparison used
   `codegen-rust` at `589c92ef`; the retained branch later advanced to
   `b9fdb80b`. Measurements tied to the first head are not silently repinned.
5. **Proof reuse.** Many 0.4 family proofs are reusable only while source, IR,
   generated Rust, ZKIR, keys, runtime, dependency graph and policy remain
   identical. EA13 must publish the final applicability/reuse matrix.
6. **Source inventory versus conformance.** The 198/232/1,055 census and the
   private IR production counts are inventories, not a complete language
   semantics proof. Historical Agda coverage remains separate.
7. **Performance portability.** The 171→55 minute test-gate result and the
   selected compiler resource timings are local measurements. Other hosts and
   CI schedulers need their own baselines.
8. **Full Passport ACC.** It was removed from the 0.3 milestone. The bounded
   passport and retained Jubjub evidence must not be described as full ACC,
   foreign-curve, WebAuthn or ledger9/10 adoption.
9. **Publication state.** This documentation branch is local and intentionally
   not pushed, merged, deployed or connected to remote CI.
