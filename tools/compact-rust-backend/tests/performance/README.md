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

Measured locally on the same frozen974-case source: serial171min -> four-process
55min (about68% less wall time), zero failures/errors/skips/unrun. This is a
machine/source-specific measurement, not a universal speed guarantee. Source and
measurement history: ADR-0492 in the midnight vault.

## Sidecar regression controls

```sh
python3 -B -m unittest discover \
  -s tools/compact-rust-backend/tests/performance -p 'test_historical_shards.py' -v
```

These controls are separate from the frozen original974-case registry. They
exercise result corruption, exact accounting and source refusal.

## Reviewed path-validation prototype (goal #508)

`historical-epoch-path-validation.patch` is an **inactive prototype**, not an
update to the default runner. It targets the external H10 orchestration owner
`final_epoch_source.py`, SHA-256
`665d3c279a94793eae0a8ab7c612c6fa42a8143f848b44e5bbb8f3e82a608489`.
Applying it in an isolated copy produces
`ebc721b4ff4e87f4a192b455579125df495980185e6e1b70c5edba89a856b366`.
Do not patch an existing authority or running validation in place.

The prototype removes a redundant containment walk after a relative path has
been joined to its root and compared with its fresh filesystem resolution.
Every live resolution, source read, digest and per-read helper check remains.
It introduces no authentication or outcome cache. The candidate needs its own
explicit authority digest; the transition manifest and historical assertions
stay byte exact.

The bounded investigation, measurements, exact commands and independent review
are recorded in the midnight vault's **Historical runner optimization —
2026-10-15** and **ADR-0504**. Local raw evidence is under
`/Users/ysh/.codex/local-evidence/compact-historical-optimization-20261015/`.
These measurements use the authenticated `ab132650` historical source epoch;
they do not qualify the upgraded compiler's current behavior. The complete
974-case run remains required before activation. Combine that gate with final
candidate freeze rather than treating a representative subset as qualification.
