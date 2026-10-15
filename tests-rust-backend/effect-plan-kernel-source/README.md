# Kernel effects composed with storage and witnesses

Actual source `examples/rust_backend/effect_plan_kernel.compact` writes a Cell,
claims a nullifier, then either obtains a mint domain and amount from ordered
witnesses or claims a receive. This is an explicitly new composition beyond the
historical kernel family. The original eight-export source remains unchanged.

```sh
cargo test -p compact-rust-effect-plan-kernel-source-fixture --all-features
```

Tests compare native, recorded and manually composed official adapters for both
branches with mint amounts 0, 13 and u64::MAX. They check state, effects, execution
gas, private state/transcript, exact public program and independent replay gas.
Failures at each witness establish once-only domain-before-amount calls and
matching errors. An error returns no partial frame, so no returned public-effect
prefix is claimed. There is no proof claim for this expansion fixture; the
original source retains the separate cryptographic/transaction qualification.
