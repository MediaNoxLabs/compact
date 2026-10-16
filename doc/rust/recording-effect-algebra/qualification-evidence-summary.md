# Recording qualification evidence inventory

This derivative inventory addresses [#653](https://github.com/betsudo/compact/issues/653). It records inspection of unchanged local originals and identifies publicly inspectable source. It is not an original acceptance authority, a newly executed qualification or a public reproduction of all 974 historical cases. The [inventory JSON](qualification-evidence-inventory.json) preserves original hashes and epochs without private root paths.

| Constituent | Verified availability |
| --- | --- |
| Historical actual source | [ab132650](https://github.com/betsudo/compact/commit/ab1326505ece60a059d6d4ba89b0d204b58e3db5) is anonymously retrievable through GitHub API. All 3,218 present actual-source pins and one required absence match the original transition manifest. |
| Historical archive source | All 150 files match [6e37443b](https://github.com/betsudo/compact/commit/6e37443b4fc2f5363954e0aef88373a48f4d3ae8) and the [published retained fixture](https://github.com/betsudo/compact/tree/785c892c5165cf0eaa12e895d5e54004822678ce/tools/compact-rust-backend/tests/retained-conformance-6e37443b). Preserve its existing Apache-2.0 LICENSE and original notices. Original descriptor identity `f0c7e3eb8dbb4867db80f82dfc64013502026e1e` stays unchanged. |
| Historical authorities and required fixture | The 948-entry fixture needs 907 source copies, 40 inverse payloads and one absent path. Original authority chain, outer wrapper, source freeze and provenance are not a public portable package. |
| Replay archive | SHA-256 `31795590babd55a30ce8cf05175911d51f65511bac0b084e9721154c15e58754`: 272 payloads, 6,741,826 payload bytes, 274 regular members. All hashes/sizes/tar round trips match. 150 files are public source; 91 path-free candidates still need license/provenance/publication review; 31 contain original owner roots and remain maintainer-only. |
| Local implementation and closure | Implementation `22a6f44cce66641c217ffd5caaf8146670b8ab57`, qualified epoch `43d7e2f0f1a14d8396ad8bcf33c6b55df841b1e0`, closure `45b575c85982c8d7a71bbd1714fb197a8e8b877b` remain local identities. All 5,586 manifest blob hashes match the local implementation tree; manifest SHA-256 is `f47437a5f8d9dd380e74ad44abc718319615899fb667358d83033a7b700d9a8f`. Local object verification does not establish public availability. |
| Reviews and receipts | Raw originals remain maintainer-only. The JSON retains all nine published closure-evidence receipt hashes and adds the final acceptance/source reviews. Derivative prose does not replace those originals. |

The original independent historical terminal review (`15104c733d0e5656711e64bb278d6186c4cde0a7554109fbc8ad5f2657f967c0`) accepted 974 cases at `ab132650`, split 245/242/244/243 across four workers, with zero failures/errors/skips/unrun. Its raw report (`7cae10e33764e51486e9300d9c937a567ebb85ca8ec0cb92c3053ebc0ae0907b`) and outer execution receipt (`846792187fc977a8eebef98bac15a134d3489cf938d8836c3b9f8e5d17817290`) were read and rehashed. These are retained local results, not new executions by this inventory.

The acceptance retains its Git metadata incident: fixture initialization inherited global Git variables and temporarily redirected observations through common `core.worktree`. Independent review accepted declared source/authority/case identities after scoped repair and actual endpoint/postchecks; continuously pristine Git configuration was not established. Original cleanup scope and external derived-bytecode disclosure remain. Six historical measurement laws establish no current performance result. The replay package omits complete proof/VM logs referenced by graph receipts.

Final independent coding-agent acceptance (`43673887f9cab2acd6d6f8d44e739e8995e477ef52260e362e3de8dcd65ed3ed`) records the finite local epic closure. Fresh affected checks (1,829 Rust and 289 authenticated Python methods) remain separate from retained compiler/consumer/historical/proof/application constituents and epochs. Counts overlap. This is not a human security audit or a newly executed monolithic release gate; the later public checkpoint has its own source identity.

The [public review at 26886cb4](https://github.com/betsudo/compact/pull/651#issuecomment-6214036479) and [re-review at 785c892c](https://github.com/betsudo/compact/pull/651#issuecomment-6219269611) report local checks with their own public source bindings. The re-review reuses earlier executable checks and repeats Python discovery. Public reports omit raw logs and do not authenticate the private historical archive. [Discussion #644](https://github.com/betsudo/compact/discussions/644) remains a maintainer narrative.

The public source subset has exact public blobs and existing Apache-2.0 licensing. The private replay archive includes no LICENSE; graph-output, wrapper and audit-prose rights were not inferred from source headers. A path-free file is not automatically cleared for redistribution. The bounded path scan is not a comprehensive secrets audit. Any changed authority or sanitized package requires a separately labeled and qualified successor; original authenticated bytes remain unchanged.

This candidate addresses the issue scope's explicit inventory/nonpublic-constituents route and prepares bounded acceptance summaries. Independent coding-agent review accepted the inventory and derivative summaries for local PR inclusion; the [availability page](evidence-availability.md) is updated. Public delivery awaits this candidate PR. The original full historical reproduction criterion is unmet: fetchable authority/fixture/outer controls and a fresh isolated 974-case result are absent. Retain that limitation even if the owner later closes the issue through its documented nonpublic-inventory route. That route does not require a new historical/proof run or private archive/history upload.

## Local public regression command

The current artifact-independent unit qualification is separate from the retained
historical population:

```sh
python3 -B tools/compact-rust-backend/ci_python_checks.py --receipt /tmp/public-python.json
```

It runs current public unit/sidecar controls and records genuine-artifact skips
explicitly. It neither reconstructs nor qualifies the original historical 974
cases. See [required checks](../guides/required-checks.md) for CI scope and branch
rule activation. Local candidate commits and their exact results are recorded in
the PR description before publication; a prepared workflow is not hosted evidence.
