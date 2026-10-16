# Historical path-validation results

This is a summary of a bounded local investigation completed on 2026-10-16 for
goal #508. It concerns the frozen `ab132650` historical source epoch and its
separately retained orchestration and fixtures. The experimental patch and
measurement driver are not part of this checkout; no optimization described here
is enabled in the current runner.

## Measured result

Profiling identified a redundant containment walk in live path validation.
Removing it preserved fresh filesystem resolution, source reads, hashing and all
198 per-read helper authentication checks. No proofs, test outcomes or
filesystem state were cached.

Three paired single-process repetitions used the same inputs, commands and
three cases: a positive historical check, a live-mutation refusal and a
forged-inverse refusal. Profiled executions were excluded from elapsed-time
comparisons.

| Measurement | Before | After |
| --- | ---: | ---: |
| Median complete process | 30.069 s | 26.958 s |
| Median fixture setup | 7.703 s | 6.598 s |
| Median test bodies | 8.822 s | 8.072 s |
| Largest process peak across three repetitions | 97,648,640 B | 97,697,792 B |

Complete-process median decreased **10.34% for these three cases**. Phase values
are separately measured and do not sum to complete-process time. The result does
not establish a full-suite speedup, current-compiler performance or a general
improvement for other circuits.

## Qualification and availability

Independent coding-agent review accepted the scoped controls and the subsequent
historical qualification: **974 unique cases**, with original assertions,
verifiers, source/authority checks and fixture lifecycle. That run took
27,005.032 s (7 h 30 min 5 s). It has no comparable accepted full-run baseline
from which to calculate a speedup, and is not current compiler proof execution.
Detailed execution qualifications and incident disclosures remain in the
original acceptance record.

The exact patch, helper/authority digests, reproduction commands, paired raw
measurements, profiles and detailed reviews are retained in the midnight vault
under **Historical runner optimization — 2026-10-15**, **ADR-0504**, its
**2026-10-16 terminal acceptance** amendment and the **PR649 detailed historical
benchmark evidence archive**. The local evidence bundle is named
`compact-historical-optimization-20261015`. This is retained evidence, not a
publicly reproducible benchmark distribution or a repository test entry point.

Use the [performance testing policy](README.md) for affected tests and historical
qualification. Publishing this summary does not change the runner, historical
authentication requirements or the qualification policy.
