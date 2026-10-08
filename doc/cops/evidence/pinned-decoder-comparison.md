# CoPS-001 preparation: ledger-8 state/verifier decoding resource contract

Read-only comparison, 2026-10-08. No malformed-input experiment, stress test, build, source/vault edit or issue creation. This is an API/source comparison, not a vulnerability finding.

## Pinned identities

- Live `git ls-remote upstream refs/heads/ledger-8` for LFDT-Minokawa/compact returned **eb72a5ab6085bf9b8564cc80c98c973e04825c2d**. Its `runtime/package.json` declares compact-runtime **0.16.101**, onchain-runtime-v3 **^3.0.0**; its lock resolves **3.0.0**. Current installed package and node_modules lock also report **3.0.0**, npm integrity `sha512-HbFbUOsvgpEFy1OT0Ni2LMFk4jnMQS1em+H+Mof/B3DpnNKl0Lkx5QiulKOc6pyU4KPEagOqx4fYNyVXFwgZ7g==`.
- Live upstream midnight-ledger tag `onchain-runtime-3.0.0` resolves **3e72c13160328368ade84fb4b73fb944caeeb781**. Live `ledger-8.0.3` resolves **615be91b079ed8df4026c1fd75352ea6d49de1a4**. Existing local Git objects supplied exact source bytes; no fetch or checkout mutation was needed.
- Installed wallet-live and wallet-handoff **@midnight-ntwrk/ledger-v8 8.0.3** expose the same state/operation wrappers plus versioned verifier-key construction.
- Current Rust workspace Cargo.lock: **midnight-ledger 8.0.3**, **midnight-onchain-runtime 3.0.0**, **midnight-serialize 1.0.0**, **midnight-transient-crypto 2.0.1**. Checksums retained in `source-identities.json`. Do not mistake the also-cached ledger8.1 crate or the newer lg8p working checkout for this project's pinned dependency.
- Cached serialize1.0.0 `.cargo_vcs_info.json` points to **ddb4568f0fab09987a399a89d7395785910e124b**. Its deserializable.rs SHA256 is `ec25eec9629d0cda2f81fc3f6a37d04b3b1318c180ee0c681fc389a197816b2b`, byte-identical to that file at the onchain3.0.0 source tag.

Exact text snapshots and hashes are in `sources/` and `source-identities.json`. Installed WASM binary hashes are retained without copying binaries. Release source/API alignment is checked; reproducible correspondence between npm WASM machine code and tagged source was **not rebuilt or proved**.

## Concrete entry-point comparison

| Surface | Exact source route | Admission / guarantee |
|---|---|---|
| Upstream Compact TS ledger-8 | [runtime/src/index.ts:101–113](https://github.com/LFDT-Minokawa/compact/blob/eb72a5ab6085bf9b8564cc80c98c973e04825c2d/runtime/src/index.ts#L101) reexports `ContractOperation` and `ContractState` from onchain-runtime-v3 | Compact adds no binary state/key admission wrapper here. The APIs come from its ledger runtime dependency. |
| Installed onchain JS3.0.0 | `midnight_onchain_runtime_wasm_bg.js:1327–1332`, `ContractState.deserialize(raw)`; `:1245–1248`, `ContractOperation.verifierKey` setter | Direct WASM call and JS error propagation. No length comparison, caller budget option or default encoded limit in these wrappers. Input is already a JS Uint8Array. |
| Onchain WASM3.0.0 source | [state.rs:445–446](https://github.com/midnightntwrk/midnight-ledger/blob/3e72c13160328368ade84fb4b73fb944caeeb781/onchain-runtime-wasm/src/state.rs#L445); verifier setter [state.rs:490–493](https://github.com/midnightntwrk/midnight-ledger/blob/3e72c13160328368ade84fb4b73fb944caeeb781/onchain-runtime-wasm/src/state.rs#L490); operation deserialize `:500–501` | Both use `from_value_ser`; the verifier setter is the analogous standalone verifier input, not a fictitious exported `VerifierKey.deserialize` API. |
| Shared WASM adapter | [lib.rs:59–71](https://github.com/midnightntwrk/midnight-ledger/blob/3e72c13160328368ade84fb4b73fb944caeeb781/onchain-runtime-wasm/src/lib.rs#L59) | `data.to_vec()` copies JS input into Rust/WASM memory, then `tagged_deserialize`; maps errors to `JsError` through an InvalidData context. No pre-copy encoded-budget parameter or owned total-work budget in this adapter. |
| Mainstream ledger-v8 8.0.3 | [ledger-wasm/src/lib.rs:56–62](https://github.com/midnightntwrk/midnight-ledger/blob/615be91b079ed8df4026c1fd75352ea6d49de1a4/ledger-wasm/src/lib.rs#L56) reexports onchain wrappers; [contract.rs:339–355](https://github.com/midnightntwrk/midnight-ledger/blob/615be91b079ed8df4026c1fd75352ea6d49de1a4/ledger-wasm/src/contract.rs#L339) constructs versioned verifier key | Copies `raw_vk.to_vec()`, accepts version v3 through `tagged_deserialize`, refuses unsupported version labels. No encoded-size policy at this boundary. |
| Rust AST compatibility APIs | `runtime-rs/src/transaction/decoding.rs`, `ObservedContractState::decode`, `decode_verifier_key` | Borrowed byte slice/Cursor into the same tagged serializer; exact decode errors and trailing check. No extra encoded maximum on the legacy APIs. Decode is neither observation authentication nor key/proof verification. |
| Rust AST limited APIs | Same file, `EncodedSizeLimit::admit`, `decode_with_limit`, `decode_verifier_key_with_limit` | Refuses `len > selected_limit` **before** calling the upstream decoder, no fallback. Caller already owns the input allocation; transport acquisition needs its own limit. This is a concrete extra boundary compared with the inspected TS/WASM wrappers. |

The captured Rust working source includes the concurrently developed ADR0359 **opt-in 64 MiB `Default` preset** (`decoding.rs:31–44,72–75`). That preset is not yet treated here as a tested/committed delivery. It does not change legacy APIs or establish a protocol maximum. The committed pre-preset ADR0353 limited APIs already supplied explicit caller-selected admission.

## Shared upstream protections and precise gaps

At pinned serialize1.0.0:

- [deserializable.rs:29–68](https://github.com/midnightntwrk/midnight-ledger/blob/3e72c13160328368ade84fb4b73fb944caeeb781/serialize/src/deserializable.rs#L29): exact expected tag, typed decode, rejects trailing bytes. Invalid tag, incomplete input and trailing input are not silently accepted. This is format admission, not origin authentication.
- `:23–26,75–89`: recursion threshold **50 in debug / 250 otherwise**, applied through participating types' `check_rec`. This is not a per-call caller-selected resource context.
- `:93–101` and [util.rs:36–40](https://github.com/midnightntwrk/midnight-ledger/blob/3e72c13160328368ade84fb4b73fb944caeeb781/serialize/src/util.rs#L36): declared Vec length is decoded, initial allocation uses at most a **32 MiB capacity estimate**, then element decoding/push continues. The initial reservation cap is **not a total decoded-allocation cap**.
- `util.rs:44–55`: byte-vector reads grow in 4096-byte chunks until requested bytes/read error; this is progressive reading, not an aggregate heap budget.
- Inspected tagged API is `tagged_deserialize(reader)` with no per-call total heap/object/CPU budget parameter. Runtime-specific VM query gas does not meter this binary deserialization route.

**Present:** shared exact-format decoding, inherited recursion/allocation precautions, error propagation; Rust AST additionally has caller-selected encoded-byte admission. **Absent from inspected APIs:** a total decoded heap/object/work budget and a TS/WASM wrapper encoded-limit option. **Unverified:** arbitrary-input termination/no-panic/resource safety, host/application transport limits, exact release WASM optimization/profile details, and a protocol-defined maximum legal state/verifier size. WASM address-space/host limits are not a documented successful per-request decoding budget. No exploitability, payload amplification ratio or production DoS outcome was established.

## Suggested CoPS-001 problem statement

> Ledger-8 binary ContractState and verifier-key ingestion share tagged deserialization across the TypeScript/WASM and native Rust stacks. The pinned upstream serializer validates tags, structure and trailing input and has recursion and initial-allocation precautions, but its API does not expose a caller-controlled aggregate decoded heap/object/work budget. The mainstream TS wrappers also copy Uint8Array input into WASM before decoding without an encoded-size policy at that boundary. Rust AST now provides explicit preparse encoded-byte admission; its proposed 64 MiB opt-in preset is an application policy, not a ledger protocol maximum or proof of total resource containment. Design a shared, explicit resource contract for these ingress APIs, including allocation/work accounting or an independently reviewed containment alternative, with transport/input acquisition ownership and cross-language error semantics documented. Preserve existing valid-format behavior and compatibility entrypoints. This records an engineering assurance gap and deferred design requirement, not a demonstrated decoder vulnerability.

Suggested acceptance questions for that future problem: which limits cover acquisition, encoded bytes, recursion, decoded objects/allocation and work; how upstream and JS/WASM propagate them before materializing large objects; how valid state/key sizes remain configurable; and which finite regression measurements substantiate the documented promise. No stress campaign or broader parser audit is proposed by this comparison.

## Subsequent delivery notification

After this source snapshot, root reported the preset signed as `9ffd7880`, with seven focused tests and strict Clippy passing. That notification supersedes the in-progress delivery status above; this comparison did not independently rerun those gates. The captured bytes and original source identities remain unchanged.


## Delivery status after this read-only capture

The opt-in64MiB preset was subsequently delivered as signed9ffd7880405ed5cfd62909ed4aac64a638c7fc02; seven decoder tests and scopedClippy pass. The report above retains its capture-time qualification. Verified archive [CoPS-001 — Pinned TS and ledger8 decoder comparison.zip](pinned-ts-ledger8-comparison.zip), SHA256 `c12dd62daacdae82bc0954c689caa89b66f95755f091ba1abc37136c46c27c53`, 26members.
