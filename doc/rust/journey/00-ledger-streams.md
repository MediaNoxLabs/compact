# Ledger streams: the boundary of this story

The Compact Rust journey documented here belongs to the **ledger-8 stream**.
Ledger-9 is not ready, and this documentation deliberately avoids converting
future application requirements into present compatibility claims.

## Delivered reference: ledger-8

The common starting point is
[`LFDT-Minokawa/compact` at `eb72a5ab`](https://github.com/LFDT-Minokawa/compact/commit/eb72a5ab6085bf9b8564cc80c98c973e04825c2d),
dated 2026-06-25. It pins a coherent compiler, ledger VM, runtime and ZK
dependency family. Each Rust generation in this story preserves that
authority:

1. the original Scheme-to-Rust text backend demonstrates the native target;
2. Rust-AST moves the target boundary to a closed typed IR;
3. the effect prototype changes recording composition without inventing a new
   ledger or cryptographic authority.

This is why “ledger-8” is more than a branch label. It constrains which
runtime types, proof artifacts, dependency identities and application receipts
can substantiate a claim.

## Comparison input: ledger-9

Ledger-9 application and protocol work may reveal useful future requirements.
Those observations can motivate an ADR, refusal case or compatibility test.
They cannot establish any of the following for the ledger-8 backend:

- ledger-9 runtime compatibility;
- ledger-9 proof or application support;
- readiness of foreign curves, WebAuthn or broader Passport ACC flows;
- upstream or product adoption.

The 0.3 passport cohort therefore retained a bounded set of ledger-8-suitable
pure programs and cross-language codec vectors. Full Passport ACC adoption was
removed instead of being reported under a broad “passport supported” label.

## Stream comparison

| Question | ledger-8 stream | ledger-9 stream |
| --- | --- | --- |
| Status in this story | Delivered reference and qualified scoped milestones | Not ready; future comparison only |
| Compiler/runtime authority | Pinned and evidenced at exact revisions | No delivery baseline claimed |
| Rust backend evidence | Original oracle, Rust-AST 0.1–0.3, active effect prototype | None claimed |
| Passport evidence | 17 files, 75 pure exports, 202 behavior cases, 4 codec vectors | Full ACC adoption deferred |
| Valid use here | Close a named ledger-8 criterion | Motivate future design or tests |

The governing rule is simple: **a ledger-9 observation may shape future work,
but only ledger-8 evidence can close the ledger-8 milestones on this page.**
