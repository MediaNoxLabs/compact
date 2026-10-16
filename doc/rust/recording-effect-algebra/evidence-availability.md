# Published source and evidence availability

This packet publishes the recording backend's source, retirement accounting and
a **maintainer-reported local closure summary**. It does not distribute all
historical acceptance material or establish a publicly reproducible full release
gate. A hash identifies evidence; it does not make unavailable evidence
independently inspectable.

## Public anchors

| Material | Public anchor | What a reader can inspect |
| --- | --- | --- |
| Published source checkpoint | [785c892c5165cf0eaa12e895d5e54004822678ce](https://github.com/betsudo/compact/commit/785c892c5165cf0eaa12e895d5e54004822678ce), tree `2f8d8916fda80b006d9ea8420df760980ddac07b` | Compiler/runtime/test sources and the documentation before this availability correction |
| Earlier publication test checkpoint | [26886cb4b35ae5ed1e62ab033e7449a41a96bd0b](https://github.com/betsudo/compact/commit/26886cb4b35ae5ed1e62ab033e7449a41a96bd0b), tree `b5befc40ddeefaa1804748e9b4d85870062acbad` | Exact public source for the reported publication checks; later Markdown corrections and patch removal are not new executions |
| Public local-check review | [Original AI-assisted review](https://github.com/betsudo/compact/pull/651#issuecomment-6214036479), [re-review at 785c892c5](https://github.com/betsudo/compact/pull/651#issuecomment-6219269611) | Reviewer-reported commands, outcomes and limitations; these comments do not embed raw logs or certify the private historical archive |
| Closure summary | [Discussion #644](https://github.com/betsudo/compact/discussions/644) | Published narrative of the local acceptance and delivery history |
| Original boundary accounting | [Retirement ledger](retirement-ledger.json) | All 84 retained boundary rows and their original identities |

The checkpoint links refer to existing public commits, not self-referential hashes
of this page. Later availability/link edits change documentation only. Local
commit IDs in historical `Source-epoch:` trailers describe provenance; they are
not promises that the corresponding local history is published.

## Maintainer-only historical material

The original local implementation `22a6f44c`, qualified source `43d7e2f0`,
closure `45b575c8`, 5,586-file source manifest, raw acceptance/review receipts,
source-continuity joins, historical runner bundles and path-bearing replay
archive are **maintainer-only**, retained in the midnight Obsidian vault and
local evidence archive. They are not public source anchors or a downloadable
reproduction package. The identifiers and receipt hashes in
[closure-evidence.json](closure-evidence.json) retain that historical scope;
its `implementation` and `new_execution` fields refer to the original local
epoch, not the later public checkpoint above.

The detailed path-validation patch, commands, measurements and incident record
have the same availability policy. The [historical results summary](../../../tools/compact-rust-backend/tests/performance/historical-path-validation-benchmark.md)
publishes only the bounded result and its limits. No full-history archive tags
or private fixture upload are implied by this documentation.

The original local acceptance is reported, not independently reproducible from
this repository. Published source and reviewer-reported local checks remain
separate from unavailable historical execution evidence. An independent
coding-agent review is not a human security audit.

## Public reproduction follow-up

[#653](https://github.com/betsudo/compact/issues/653) tracks a reviewed,
redistributable qualification/evidence package, including source/authority and
fixture availability, provenance review and an isolated reproduction where
possible. It is required before claiming public reproducibility of the full
historical acceptance. Sanitized material must retain its own qualified successor
identity rather than silently replacing original authenticated bytes.

[#652](https://github.com/betsudo/compact/issues/652) separately tracks small
fixture-independent authentication controls for ordinary CI. Synthetic controls
do not substitute for the original historical graph/proof evidence.
