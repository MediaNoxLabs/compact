# Retained Rust emission snapshots

These 86 rustfmt-normalized, unhosted `lib.rs` snapshots preserve an emission
regression oracle for the registry's build-time generated consumers. The existing
169 flat libraries remain their own snapshots, including the one flat library
with a build binding. Accepted compiler probes have no retained output here.

Build scripts still generate code using the current emitter. Cargo dependency
bindings, retained IR consistency, execution assertions, proofs and negative
controls remain separate gates. Comparing two current-emitter outputs does not
replace comparison to this reviewed retained output.

The pre-migration committed bodies for the 29 converted flat libraries remain in
the original AST branch history (base `46fdf`) and the original migration branch.
These snapshots are a reviewed successor for the current ledger8.1.3 emitter;
they are not an assertion that current output is identical to that predecessor.

## Check and deliberately update

Use the existing fixture checker with a current Compact compiler selected by
`COMPACTC`. It checks all 255 retained outputs by default; `--only` selects a
source basename for focused checks. The full local parity gate uses the same
mapping and compares snapshots before joining fresh outputs to compiled hosts.

```sh
COMPACTC="$PWD/result/bin/compactc" python3 -B tools/compact-rust-backend/check_fixture_outputs.py --only field_to_bytes32_oracle.compact
```

For an intentional source or emitter change, first run that check and review the
reported drift. Then regenerate only the affected snapshots explicitly:

```sh
COMPACTC="$PWD/result/bin/compactc" python3 -B tools/compact-rust-backend/check_fixture_outputs.py --update --only field_to_bytes32_oracle.compact
```

Review and commit the actual Rust diff together with the source/emitter change.
A normal check never updates snapshots. Do not run an automatic update as part
of validation or treat newly regenerated output as evidence that behavior is
correct; retained execution and proof assertions must still pass.
