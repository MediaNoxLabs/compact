# Checked unsigned subtraction and multiplication

The current backend emits the full retained IR of
`examples/rust_backend/effect_plan_unsigned_arithmetic.compact`. Both public
Struct-returning exports, `run` and `narrow`, must be recorded and observed.

`run` subtracts two witnessed Uint8 operands, writes the difference, then
conditionally calls a stateful Uint16-returning multiplication helper. Its two
witnessed operands observe the preceding write. `narrow` conditionally multiplies
and applies the existing checked Uint8 cast. No pure-call bridge hides the tested
multiplication: the helper is stateful because it calls witnesses.

Four tests cover 12 successful subtraction/product cases; six accepted narrowing
cases; two out-of-range results and their skipped branches; four underflows;
six witness failures and two skipped refusals. Tagged witnesses record private
state, operand value, and metered difference/narrowed Cells, proving left-to-right
once-only callbacks and visible write prefixes. Results are also checked against
independent integer arithmetic. A manual official-runtime/slot program checks
state, effects, execution gas, ordered public operations and independent replay
gas. Errors do not return partial frames; only witness-observed prefixes are
claimed.

Uint8 multiplication widens to Uint16 and is representable over its input domain.
The overflow controls are **checked result narrowing** (`UnsignedOutOfRange`),
not machine-u128 multiplication overflow. Wide-native arithmetic and wider proof
ABI are outside this source fixture. Proof execution is separately reported.

## Source limitation retained in evidence

The initial inline conditional expression `operand(...) * operand(...)` under an
annotated Uint16 local emits IR but native rendering rejects inferred Uint65025
against operation result Uint65535. The exact original source emits TS with the
same pinned frontend (`--skip-zk` only, not TS execution). The accepted source
uses an explicitly Uint16-returning stateful helper. This fixture does not repair
or claim acceptance of the inline inferred-bound shape.

For proof/integration callers, `support::Witness`, `support::U8`,
`support::context()` and `RETAINED_IR` are public. Witness private state is u64;
the standard constructor context starts at 7. `Witness::refuse_tag` optionally
refuses one tagged callback. No new runtime representation is introduced.

```sh
cargo test --offline --locked -p compact-rust-effect-plan-unsigned-arithmetic-source-fixture --all-features
```
