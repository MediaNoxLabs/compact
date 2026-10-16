# Historical path validation: bounded investigation

Decision, 2026-10-16: accept the exact reviewed patch as an opt-in artifact for
the authenticated historical orchestration. The default runner remains unchanged.
Do not patch a running validation or its existing authority in place.

## Measured opportunity

Profiling the current runner found redundant containment work in live path
validation. The measured three-case selection includes a positive historical
check, a live-mutation refusal and a forged-inverse refusal. The change removes
one redundant walk; fresh resolution, reads, hashing and all 198 per-read helper
authentication checks remain. No results, proofs or filesystem state are cached.

Three paired single-process repetitions used identical inputs, commands and
case IDs. Profiled executions were excluded from elapsed-time comparisons.

| Measurement | Before | After |
| --- | ---: | ---: |
| Median complete process | 30.069 s | 26.958 s |
| Median fixture setup | 7.703 s | 6.598 s |
| Median test bodies | 8.822 s | 8.072 s |
| Largest process peak across three repetitions | 97,648,640 B | 97,697,792 B |

Complete-process median decreased **10.34% for these three cases**. Phase values
are separately measured and do not sum to complete-process time. This is not a
speed guarantee for other circuits or the full historical registry.

## Correctness and qualification

Independent review checked the path controls, live mutations and source/authority
authentication. The unchanged full run then passed **974 unique cases**:
292 ordinary, 548 earlier historical, 128 latest historical and six retained
measurement laws. Original case identities, assertions, verifiers, baselines,
manifests and fixture lifecycle were retained. Final source, authority and Git
checks passed. No test was restarted or omitted.

The full run took **27,005.032 s (7 h 30 min 5 s)**. Largest worker-process peak
was 413,483,008 B; sampled concurrent orchestrator/worker RSS reached 748,290,048 B.
The latter is a 0.5 s sampled lower bound on the true peak, not a memory limit.
There is no comparable accepted full-run before/after speedup claim.

A previously disclosed common Git configuration incident was repaired during
the run. Independent review accepted the source/authentication premises and
successful final checks; continuous metadata invariance is not claimed. Two
incidental external bytecode files remain disclosed. These are historical test
results, not upgraded-compiler or current proof-execution qualification.

## Reproduction and evidence

The adjacent README and patch bind the exact predecessor and candidate helper
digests. Use a separate copied bundle and independently supplied candidate
authority; preserve the original runner, source epoch, fixtures and case set.
The normal runner command and its fresh-output requirement remain in the README.

The representative command was run once per paired repetition with Python
3.14.3; `$evidence` denotes the retained local evidence bundle, and `$variant`
is `baseline` or `candidate`:

```sh
python3.14 -B "$evidence/profile_selection.py" \
  --bundle "$evidence/$variant-path-validation" \
  --authority "$authority" \
  --output "$evidence/complete-process/$variant-$repeat.json"
```

The baseline authority was
`1066527b5803cb2f50e5cf9b4b7c70ebd665460bc8e37182b3dc109fbf913549`;
the candidate authority was
`ec34d8fbf42ca48b7c2a62ea57d390f991a96c5250301ef6c2092bc649210f70`.
The parent timer included interpreter startup and exit; instrumented profiles
and raw commands are retained separately. This measurement driver uses the
archived fixtures and is not a new repository test entry point.

Detailed commands, paired measurements, profiles and independent review are in
the midnight vault: **Historical runner optimization — 2026-10-15**, **ADR-0504**
and the **2026-10-16 terminal acceptance** amendment. The bounded investigation is
closed. Use affected tests during delivery and complete historical qualification
when its machinery changes or at final candidate freeze; reuse evidence only
when all relevant premises remain valid.
