# Current-profile wide-add producer proposal

`capture.mjs` is a portable successor of original producer
`cb21e82cfbaf9b0ffa441f41cc024fe5c0e8756c7e4b846604c18e530f80737a`.
It preserves the complete original arithmetic, witness, narrowing, transcript,
serialization and query-cost assertion body. The finite current-profile execution
below has passed locally and received independent terminal acceptance.
Publication and final composed-source acceptance remain separate.

The historical raw capture `9bbef47b8f3ae54325e26908a6b5b7a6a85cbc9adf8e0797a8cb55232bd03701`
and its files-label-only public successor
`052cd0a03bfc1e116fa2d4f011129e7973ea86fef8ccf45806ee3dde155069bd`
remain independent artifacts. A new capture uses compact-runtime 0.16.101 with
onchain-runtime 3.1.2 and TypeScript 6.0.3. It cannot recreate the original
onchain-runtime 3.0.0/tool epoch.

## Inputs and command

Use canonical absolute paths for all arguments. Existing directories and input
files must already exist. Both output paths must be fresh. The producer creates
no dependencies, aliases or generated code; it never installs packages or uses
the network. The caller owns a private generated-contract dependency layout.
That layout must resolve `@midnight-ntwrk/compact-runtime` to the exact selected
runtime entry; QueryContext identity is checked before instrumentation. Shared
`node_modules` must not be modified. Custom Node loaders/search paths are refused.

First compile the same Compact source into a fresh output with the source-bound
current frontend and CLI. Both targets can be requested in one invocation:

```sh
COMPACTC_SCHEME="$FRONTEND" "$COMPACTC" --target ts --target rust --skip-zk \
  --rust-runtime-root "$REPO" "$SOURCE" "$GENERATED"
```

`SOURCE` is `examples/rust_backend/effect_plan_wide_add.compact`. `GENERATED` is
absent before compilation. The generated files include `contract/index.js`,
`contract/index.d.ts`, `contract/compact-rust-ir.json` and `contract/lib.rs`.
No keys or proofs are generated. Record the actual compiler command and compiler,
frontend, source/import, runtime and tool identities separately. Compare the
complete fresh IR with the retained schema20 IR before using the existing Rust
fixture. Report every observed difference individually; no arbitrary fields,
comments, timestamps or source paths may be ignored by a blanket normalization.
If the IR does not join, stop for review.

The caller creates a private byte-pin JSON (`INPUTS`) independently before
launching the producer:

```json
{"schema":1,"files":[{"path":"/absolute/input/file","sha256":"64 lowercase hex characters"}]}
```

Hash bytes, not timestamps. Include the producer, current source, generated JS
and declarations/IR/Rust, CLI, frontend launcher, Node executable and the entire
selected compact-runtime, resolved onchain-runtime, TypeScript, `@noble/hashes`
and `object-inspect` package payloads. Walk package roots recursively excluding
`node_modules`; supply the resolved package payloads separately. Also include
the nearest generated-contract package.json when present and all external
frontend/import inputs from the authenticated frontend profile. Each path must
be canonical and unique; symlink/special-file payloads are refused. Additional
explicit provenance inputs are allowed and are rehashed. Missing pins or changed
bytes fail before capture. Inputs and resolution are rechecked before publication.
This inventory pins inputs; it does not cache results or prove that the caller
selected a trusted compiler.

```sh
"$NODE" "$REPO/tests-rust-backend/effect-plan-wide-add-source/oracle/capture.mjs" \
  --source "$SOURCE" --contract "$GENERATED/contract/index.js" \
  --runtime "$RUNTIME_PACKAGE" --compiler "$COMPACTC" --frontend "$FRONTEND" \
  --typescript "$TYPESCRIPT_PACKAGE" --ir "$GENERATED/contract/compact-rust-ir.json" \
  --rust "$GENERATED/contract/lib.rs" --inputs "$INPUTS" \
  --output "$NEW_CAPTURE" --provenance "$PRIVATE_RECEIPT"
```

Run with `NODE_OPTIONS` and `NODE_PATH` absent and no Node execution flags.
The public capture retains the original row format and publishes content labels
and hashes. The separate private receipt records actual paths and invocation.
It must not be committed. Existing outputs are never replaced. Instrumentation
is restored even if a circuit assertion or publication fails.

## Repeating the finite qualification

Execute all 21 cases (15 successes, six refusals) and compare ordered, scalar-type-
exact `rows` with the retained JSON. Report new versions and input identities
separately. Gas, errors, transcript or private-state differences are findings;
do not normalize them away or rewrite the old oracle. Run private refusal
controls for a mismatched runtime module, a changed pinned source/generated
input, and preexisting output protection. Keep old artifacts unchanged.

Only after the exact fresh-IR join, run the four current Rust fixture methods:

```sh
cargo +1.99.0 test --offline --locked -j2 \
  -p compact-rust-effect-plan-wide-add-source-fixture \
  --features ledger-transaction --tests
```

Use an own fresh Cargo target and capture its actual generated `OUT_DIR` source,
retained IR, runtime features and dependency origins. These four methods check
finite behavior and replay; they do not generate or verify proofs. Independent
review of source and actual results is required for any successor qualification.

## Executed current-profile results (terminal review accepted)

The 2026-10-16 run executed the recipe above with current compiler0.31.134 and
ledger8.1.3. Compilation took 6.535 seconds; the genuine 21-case Node capture took
0.376 seconds. Three private safety controls took 0.323 seconds in total. The
four Rust fixture methods passed in 115.625 seconds including their fresh own
Cargo build. These are measured command times, not complete workflow wall time
or general performance claims.

The complete fresh typed IR matched retained `e11ffee3…` byte for byte. All 21
ordered rows matched historical `oracle/typescript.json` with exact scalar types;
no gas/error/private/transcript fields were removed or normalized. The three
private refusal controls exercised the runtime-module mismatch, changed expected
generated-input hash and preexisting-output protection. Actual Rust `OUT_DIR`
code matched fresh CLI output SHA256
`8121b27bfda88e0f652ee497b185498268cc9542be5a363ab014849d43117cea`.
Actual test dependency fingerprints and depfiles joined the selected runtime,
host backend and observed source inputs to the frozen worktree. The empty
library harness was not counted as an extra test.

[`current-ledger813-typescript.json`](current-ledger813-typescript.json), SHA256
`b6d2de03ff405ed923cfbb7a252ad48b46c960f1e130dc69fc51bb0c113fff99`,
is published separately from historical `typescript.json`; existing Rust tests
continue using that historical oracle unchanged. The capture includes logical
content labels for 493 pinned inputs. Labels are not download locations. Exact
machine paths, the caller input manifest and invocation receipt remain private;
opaque hashes identify those receipts without distributing or reconstructing them.
The old raw capture `9bbef47b…`, public metadata successor `052cd0a0…` and producer
`cb21e82c…` remain historical identities. This new run uses onchain3.1.2 and does
not recreate the old onchain3.0.0/tool epoch.

Independent coding-agent terminal review accepted the current finite results:
`wide-add-current-profile-terminal-independent-review.json`, SHA256
`f05978c9d7c3b1353d39c0cb35f544d20ab19ca2a3769dd224a46a4df34e305c`.
The receipt is retained external evidence, not distributed here. This publication
source change performs no additional execution; publication and final composed-
source acceptance remain separate.
No prover/verifier keys, proofs, funded transactions, Forge integration or full
workspace/language coverage are claimed.
