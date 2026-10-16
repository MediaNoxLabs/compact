# Original Kernel shielded effects source

The current backend generates this fixture at build time from the complete,
unchanged eight-export `kernel_shielded_effects_oracle.compact` source and its
exact retained IR. Seven exports exercise Kernel effects; `read_state` is the
ordinary Cell read control. Existing TS observations cover operation order,
duplicate claims, mint zero/max/overflow, branch selection, witnesses and replay.
Execution gas and combined replay gas have different accounting boundaries.

```sh
cargo test -p compact-rust-kernel-shielded-effects-oracle-fixture --all-features
```

`compact-rust-proof-smoke --kernel-shielded-effects OUTPUT` first binds the full
retained IR before accessing keys. Its source guard rejects mutated declarations
and the separate composition fixture. Generate fresh keys from the exact source.
The proof route keeps three distinct claims:

- Seven call proofs verify and reject a changed public binding. This alone does
  not establish transaction acceptance.
- Five unmatched-offer transactions reject with exact mandatory claim errors;
  `selected(false)` records zero effects and refuses `EmptyTranscript` preparation.
- Offer-backed `funded_mint` mints 42 with a matching output and Night-backed Dust,
  passes default strict validation, and applies to the ledger.

Full strict validation is required only in the funded positive case. No fabricated
coin, synthetic offer, or balancing-disabled negative is counted as that success.
