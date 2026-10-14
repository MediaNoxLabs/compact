# Historical qualification performance

Use affected compiler/unit/resource tests during delivery. Run the complete
historical suite when its machinery or authenticated input views change and at
final candidate freeze. Reuse accepted evidence only when all relevant source,
tool and authority premises remain valid.

## Complete suite

```sh
python3 tools/compact-rust-backend/historical_law_shards.py \
  --root "$repo" --archive "$archive" --graph "$graph" \
  --workers 4 --output "$fresh_report"
```

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
in the midnight vault. Those earlier measurements are not a before/after baseline
for the 974-case path-validation qualification below.

## Sidecar regression controls

```sh
python3 -B -m unittest discover \
  -s tools/compact-rust-backend/tests/performance -p 'test_historical_shards.py' -v
```

These controls are separate from the frozen original974-case registry. They
exercise result corruption, exact accounting and source refusal.
