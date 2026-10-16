# Historical qualification performance

Use affected compiler/unit/resource tests during delivery. Run the complete
historical suite when its machinery or authenticated input views change and at
final candidate freeze. Reuse accepted evidence only when all relevant source,
tool and authority premises remain valid.

## Complete suite

```sh
python3 tools/compact-rust-backend/historical_law_shards.py \
  --root "$legacy_source" --archive "$archive" --graph "$graph" \
  --workers 4 --output "$fresh_report"
```

`legacy_source` must be an explicit immutable checkout/archive containing the
original driver and runtime descriptor pinned in `historical_law_shards.py`.
Those owners are not present in the current publication checkout. Passing this
checkout as `--root` refuses; it does not reconstruct a compatible predecessor.
The original fixture/evidence bundle is retained separately. This extraction
does not publish that bundle or claim a self-contained current-tree full gate.

The archive and graph are the same explicitly authenticated fixtures required by
`historical_law_dispatcher.py`. Reports must be outside all source/fixture roots
and fresh. One to four processes are supported. Whole modules stay together;
original assertions, verifiers, source views and fixture cleanup stay unchanged.
Each worker stops after a failed group; already-running peers may finish. Failed,
skipped, missing, duplicate or malformed results cannot qualify the complete run.

`--modules module_name ...` runs an explicitly labeled subset. It cannot claim
complete qualification. Reports retain original IDs, outer-case timings, worker
logs, source pins and memory measurements. Timings exclude outer module/class
fixture setup; total elapsed includes it. Concurrent RSS is sampled every0.5s
and is a lower bound on the true peak, with sample errors/count reported.

The sidecar pins the reviewed original driver and runtime descriptor. Changes to
these owners require source review and renewed qualification before updating the
pins. No source/path/authentication/pass-fail cache is used.

Process sharding has its own archived source and measurement history in ADR-0492
in the midnight vault. Those earlier measurements are not a new before/after
benchmark of this publication checkout. The later H10 974-case qualification
also requires its separate authenticated orchestration/authority bundle under
ADR-0508; invoking this generic sidecar alone does not reproduce its outer checks.
There is no new speedup, proof execution or full historical execution claim here.

## Sidecar regression controls

```sh
python3 -B -m unittest discover \
  -s tools/compact-rust-backend/tests/performance -p 'test_historical_shards.py' -v
```

These controls are separate from the frozen original974-case registry. They
exercise result corruption, exact accounting and source refusal.

## Historical path-validation results

The [historical results summary](historical-path-validation-benchmark.md) records
a bounded investigation on a frozen, separately retained historical runner.
Three paired representative processes measured a 10.34% median elapsed-time
reduction. This is not a current-runner or full-suite speedup claim.

The experimental patch, measurement driver, raw results and detailed review are
retained in the midnight vault and local evidence archive. They are not shipped
as runnable tooling in this checkout. The current runner is unchanged; the
complete-suite command above still requires its explicitly selected historical
inputs. Future changes to that machinery require qualification under the policy
above.
