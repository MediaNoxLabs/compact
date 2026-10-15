# Composed Zswap intents

Current backend generation from exact `effect_plan_composite.compact` IR. The
source composes input registration, a selected stateful output helper, another
input registration, witnessed coin/recipient arguments, and a returned original
coin. It exercises repeated owned values without moves or duplicate evaluation.

```sh
cargo test -p compact-rust-effect-plan-composite-source-fixture --all-features
```

An independent reference composes the official recording-frame intent adapters.
Tests cover both branches, zero/42 values, user/contract recipients, exact private
Unit/witness output order, private state, intent-event order, state and execution
gas. No public query is fabricated: preparation returns `EmptyTranscript`.
Witness failure, output cursor overflow and ordinary-observation allocation lock
preserve exact errors and the observable witness-call prefix. Errors return no
partial public frame, so tests make no returned partial-frame claim.

Repeated equal inputs/outputs are deliberately used to test compiler ownership
and event preservation. This fixture is not a balanced transfer, a valid offer,
or a proof-qualified transaction. The original composite transfer fixture and
proof-smoke route provide separate strict offer/transaction evidence.
