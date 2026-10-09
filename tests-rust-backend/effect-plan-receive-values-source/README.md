# Point equality and empty local structs

Build-time generation uses exact source IR for
`examples/rust_backend/effect_plan_receive_values.compact`. Both circuits return
a supported Boolean-field Struct; storage remains a Boolean Cell. The empty
named Struct is local only.

```sh
cargo test -p compact-rust-effect-plan-receive-values-source-fixture --all-features
```

Point cases use official runtime identity and generator multiplication. Explicit
expected Booleans cover identity/identity, separately constructed equal generator
points, identity/generator, and distinct generator multiples. Both marker values
and branch selections check aggregate equality, repeated point reuse and the
order of one/two writes followed by a read. Empty defaults and zero-field
construction compare equal and are reused after either prior Cell state.

An independent reference composes official slot/recording operations from the
known expected Booleans. Tests compare native and recorded result/state/effects,
execution gas, private state/output absence, exact public operations, and replay
against independently recorded replay gas.

These finite cases qualify generated runtime behavior; they do not validate
arbitrary point decoding, change subgroup validation, introduce empty storage or
root interfaces, prove full grammar inclusion, retire the receive selector, or
provide a new cryptographic proof qualification.
