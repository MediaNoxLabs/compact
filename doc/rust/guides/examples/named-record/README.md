# Application-owned named inputs (historical 25-case recipe)

The [wrapper](src/lib.rs), [complete table test](tests/age_predicate.rs), manifests, lock and three captured JSON inputs are unchanged copies of the accepted external recipe. See the [walkthrough](../../passport.md#give-a-long-call-named-inputs-in-your-application) and [historical source/lock/test evidence](../../../evidence/0.3.0/index.md#named-record). This is one integration test covering 25 stored cases (6 successes and 19 exact errors).

## Required generated dependencies

This is a complete handwritten example, not a self-contained generated SDK archive. Before Cargo can run, the unchanged manifest requires:

- `generated/contract/`: the maintained `compact-rust-vc-passport-adoption-fixture` package from the historical source snapshot `fba6845f`; its `lib.rs` must match the hash in the retained source receipt. Its original consumer manifest points runtime to `../../sdk/runtime-rs`.
- `sdk/runtime-rs` and `sdk/runtime-rs-macros`: the exact qualified source pair identified by the historical receipt. The existing [SDK preparation script](../../prepare-sdk.py) can stage source components, but using a new source checkpoint requires a new qualification. Its extra testkit files are not required by this pure wrapper.

The original generated manifest/runtime-path adaptation is recorded in the source receipt. A freshly generated default package has a different package name from this maintained fixture: do not imply that the unchanged handwritten Cargo manifest accepts it without a deliberate application-manifest adaptation. This example includes neither generated output nor the SDK. No executable source or manifest was changed, and no new build or execution is claimed.

Once those exact dependencies are restored and qualified, the retained command was `cargo +1.99.0 test --offline --locked -j4 --test age_predicate` with `CARGO_INCREMENTAL=0`. The lock requires the recorded registry cache; do not resolve a fresh unconstrained graph and call it the original run.
