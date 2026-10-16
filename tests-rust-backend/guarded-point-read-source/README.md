# Original guarded point read regression

This fixture generates the exact original source retained at
`tools/compact-rust-backend/tests/proof-regressions/guarded-point-read/effect_plan_jubjub.compact`.
It intentionally keeps the point read inside each branch. The #517 hoisted
source and fixture remain separate and unchanged.

The runtime test compares both branches over identity/nonidentity points and
seven scalar boundaries against native execution and the hoisted fixture's
recorded program: result, state, effects, execution gas, private state, public
transcript and independent replay gas. These 28 cases establish finite runtime
regression coverage; shared compiler proof behavior has a separate gate.

`compact-rust-proof-smoke --guarded-point-read <fresh matching compiler output>`
requires the exact original retained IR before reading keys. It uses the same
source proof harness as #517, with this fixture's independently generated crate.
The compiler output and proof keys must be regenerated after the shared compiler
fix; old keys are not evidence for the changed ZKIR.

## Official VM boundary controls

The proof smoke binary also has `--guarded-point-vm <artifact-root>`. The root
contains freshly compiled `witness/`, `aggregate/` and `invalid/` source outputs,
plus `carrier.verifier` (a verifier used solely by the typed call-preparation
API; this route performs no proof/key validation). Independent typed ledger
programs construct the public/private transcripts for the official checker.

The route checks both witness and nested aggregate branches, both valid-point
branches, and inactive malformed coordinates. Active malformed `(0,0)` still
causes the pinned official curve constructor to panic. This is explicitly a
host-boundary diagnosis, not proof rejection or a security guarantee. Its
negative preimage has matching active inputs and a fresh communication
commitment, so an unrelated stale-commitment error cannot satisfy the check.

Separate synthetic `IrSource` unit controls cover all inactive nested-guard
combinations, active valid points, changed valid defaults, flipped guards and
non-Boolean guards. They supplement source-produced artifacts and real proofs;
they do not establish generated Rust recording support for point witnesses or
aggregate-return profiles.
