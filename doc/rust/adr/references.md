# Evidence and publication conventions

## Publication boundary

The ADR body records historical engineering decisions and evidence. The original UTF-8 source hashes are in [the manifest](publication-manifest.json); source notes remain in the project knowledge base. This repository publication includes the ADR text, not private wallet/proof bytes, operational journals or every historical test artifact.

The public [Milestone 2 closeout](https://github.com/MediaNoxLabs/compact/issues/103#issuecomment-6017383781), individual issue closure links and pinned commits provide reviewable summaries and implementation provenance. They do not make a local receipt publicly downloadable or establish that each old command is reproducible against today's toolchain.

## Symbolic historical locators

The following substitutions remove machine-specific paths while retaining artifact names, historical hashes and command structure:

| Symbolic root | Meaning |
|---|---|
| `${COMPACT_SOURCE}` | The historical Compact checkout or worktree used by the recorded command; its exact revision/profile remains stated in the record |
| `${MIDNIGHT_LEDGER_SOURCE}` | Historical ledger-8 checkout, including upstream static proof/test material |
| `${LOCAL_EVIDENCE}` | An unpublished local scratch, proof, capture or test-receipt directory |
| `${HISTORICAL_NIX_STORE}` | Historical Nix store; derivation identities remain provenance, not portable installation instructions |

These are documentation placeholders, not variables configured by the repository. Relative `target/` paths are historical build outputs unless a record says otherwise. A command using a placeholder requires selecting the matching source revision, toolchain, inputs and evidence location; copying it verbatim does not reproduce an archived run. No local evidence file was imported merely because an ADR referenced it.

## Unpublished supporting notes

The following Obsidian references were not ADRs and are outside this publication. Their titles and available source hashes preserve traceability. “Unavailable” means no uniquely matching source note was located during this import. Related public context links are navigation aids, not claims that the private note's full contents or artifact bytes are public.

### private-note-01

**Original note:** ABI-28 live wallet admission — 2026-10-04.

Source SHA256: `cdb675cbe0e17ea0ec8f8eb2344842edebe46d818f72ae9a61bf090951bb3fe3`. This note remains unpublished.

Related public context: [Milestone 2 closeout](https://github.com/MediaNoxLabs/compact/issues/103#issuecomment-6017383781).

### private-note-02

**Original note:** ADR213 original Coracle start source admission matrix — 2026-10-06.

Source SHA256: `dde2ede7f3ce32a11e586a9a909f87b64f92695333b3ab22e7337e4ead15308a`. This note remains unpublished.

Related public context: [RUST-ADR-0213](0213-record-original-coracle-start-with-two-player-funding.md).

### private-note-03

**Original note:** ADR213 original Coracle start strict offer proof plan — 2026-10-06.

Source SHA256: `a30a8c542f3a1300a2e22e1922cde3981a7b034b1468cdb80df3dc36c7790e33`. This note remains unpublished.

Related public context: [RUST-ADR-0213](0213-record-original-coracle-start-with-two-player-funding.md).

### private-note-04

**Original note:** ADR217 — Confirmed observation binding design.

Source SHA256: `fd37def780514e0596c3b5c9ee3e6e1935e0b22f459b8499eac8f703028251d0`. This note remains unpublished.

Related public context: [RUST-ADR-0217](0217-current-revision-live-wallet-shielded-acceptance.md).

### private-note-05

**Original note:** Generated Rust — Transient offer reconciliation research — 2026-10-06.

Source SHA256: `75d36b1ffebb963eb4c1d78f4668e5bc511457a66e6577ae7928f2e3327d41f8`. This note remains unpublished.

Related public context: [Milestone 2 closeout](https://github.com/MediaNoxLabs/compact/issues/103#issuecomment-6017383781).

### private-note-06

**Original note:** Artifacts/rust-backend-v2-remote-03c03a39/README.

Source SHA256: `804ab8af5cb99139da8390b4a12548009da3a76b1bc6a538caa10e29a3d0103b`. This note remains unpublished.

Related public context: [Milestone 2 closeout](https://github.com/MediaNoxLabs/compact/issues/103#issuecomment-6017383781).

### private-note-07

**Original note:** Rust backend architecture overview — 2026-10-06 — 03c03a39.

Source SHA256: `6c386eb4f6814e19ea5d22b0a53122012406204c3f47705134bee5f704e124b8`. This note remains unpublished.

Related public context: [Milestone 2 closeout](https://github.com/MediaNoxLabs/compact/issues/103#issuecomment-6017383781).

### private-note-08

**Original note:** Milestone 2 — ADR delivery map.

Source SHA256: `f9494601bcf645c66179c8a1c07305858afed277abac57c9547e3e014c03d518`. This note remains unpublished.

Related public context: [Milestone 2 closeout](https://github.com/MediaNoxLabs/compact/issues/103#issuecomment-6017383781).

### private-note-09

**Original note:** Milestone 2 — Current local checkpoint.

Source SHA256: `72d5ee6220f3d797b3447b1d94d27c0a803eb2129ba438c49ae15f4883938b10`. This note remains unpublished.

Related public context: [Milestone 2 closeout](https://github.com/MediaNoxLabs/compact/issues/103#issuecomment-6017383781).

### private-note-10

**Original note:** Milestone 2 — Full TS parity delivery accelerator.

Source SHA256: `6e885e501bee2b5d52349f12943d8cfad71aad7f8d4a009341f2f80aa4de2d39`. This note remains unpublished.

Related public context: [Milestone 2 closeout](https://github.com/MediaNoxLabs/compact/issues/103#issuecomment-6017383781).

### private-note-11

**Original note:** Milestone 2 — Installer stabilization plan and artifact provenance — 2026-10-06.

Source SHA256: `6db80c210e49e11b6a26da3cc7c01bbee5a67366dd378e84ca89fcbdf03e4bc9`. This note remains unpublished.

Related public context: [Milestone 2 closeout](https://github.com/MediaNoxLabs/compact/issues/103#issuecomment-6017383781).

### private-note-12

**Original note:** Oracle acceptance matrix — 37 codegen-rust fixtures — 2026-10-02.

Source SHA256: `cc7dd2107e6066074d02ad7c341c9d4069c3cb0730d11fbf9ff3869e90465156`. This note remains unpublished.

Related public context: [Milestone 2 closeout](https://github.com/MediaNoxLabs/compact/issues/103#issuecomment-6017383781).

### private-note-13

**Original note:** Original Coracle concede — guaranteed actionful payout research — 2026-10-06.

Source SHA256: `e7f8c06a647b9d57f79a49b19d06d539efb0b9a2edb6f13113ef73011f2cd18d`. This note remains unpublished.

Related public context: [Milestone 2 closeout](https://github.com/MediaNoxLabs/compact/issues/103#issuecomment-6017383781).

### private-note-14

**Original note:** Original Coracle start — two-player funding research — 2026-10-06.

Source SHA256: `4f821a25fcf3b3b0fb3148f29c9d9a3f8bac31a0371b089011ef2aea950413eb`. This note remains unpublished.

Related public context: [Milestone 2 closeout](https://github.com/MediaNoxLabs/compact/issues/103#issuecomment-6017383781).

### private-note-15

**Original note:** Original microDAO buy_in — guaranteed mint funding research — 2026-10-06.

Source SHA256: `4b097735b835aed19bf06fdff44586f9605e21d8f9604726afb4141c00c16328`. This note remains unpublished.

Related public context: [Milestone 2 closeout](https://github.com/MediaNoxLabs/compact/issues/103#issuecomment-6017383781).
