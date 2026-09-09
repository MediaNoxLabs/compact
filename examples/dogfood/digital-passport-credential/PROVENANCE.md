# Provenance: `examples/dogfood/digital-passport-credential/`

This directory is a **vendored third-party enclave** (see the dogfood ADR and
AGENT.md §1). Its contents are not authored in this repo and must never be
locally modified: any divergence from upstream is picked up only by an explicit,
recorded re-sync (procedure below). It exists as bounded dogfooding — proof that
this toolchain compiles the real, named, production contract it exists to serve —
and deliberately supersedes, **for this enclave only**, the corpus de-branding
decision of commit `08decb1` / ADR-0001's follow-up note. Everything outside
`examples/dogfood/` remains third-party-free.

## What is vendored

| Path | Source | Files |
| --- | --- | --- |
| `src/` | Upstream package `midnight-verifiable-credential-digital-passport`, package tree `packages/midnight-verifiable-credential-digital-passport/src/`, copied **verbatim** (upstream Apache-2.0 headers intact) | 22 |
| `core-compact-staging/` | npm package `@midnight-ntwrk/credential-compact@0.1.0-rc3`, `dist/credentials.compact` + `dist/credentials/` (the 15 files the upstream staging script `scripts/stage-core-compact.mjs` resolves through the package's exports map) | 15 |

Upstream `src/digital-passport-credential.compact` contains
`include "../core-compact-staging/credentials";` — the staging directory must
therefore sit as this directory's sibling of `src/` for the compiler's relative
include to resolve. Upstream regenerates the staging dir as an untracked build
artifact on every `compact` run; this repo commits it instead so the fixture is
hermetic and CI is network-free. The staged bytes are still exactly the
published npm `dist/` bytes (byte-verified; see "Verification").

## Upstream identity

- Repository: <https://github.com/midnightntwrk/midnight-verifiable-credential-digital-passport>
  (pnpm monorepo; the vendored package lives at
  `packages/midnight-verifiable-credential-digital-passport/` inside it)
- Pinned revision: `cdeb860bc426590adeca3f39dc8d8efc8a62638f`
  (`cdeb860b`, 2026-09-07, "chore(deps): update pnpm to v10.34.5 [security] (#31)")
- License: Apache-2.0 (Midnight Foundation copyright; upstream `LICENSE` /
  per-file SPDX headers)
- Core (staged) package: `@midnight-ntwrk/credential-compact@0.1.0-rc3`
  — tarball: <https://registry.npmjs.org/@midnight-ntwrk/credential-compact/-/credential-compact-0.1.0-rc3.tgz>

## Refresh procedure (manual, explicit)

Nothing in this repo tracks upstream: when upstream moves, this tree stays
pinned until a human re-syncs. A refresh updates the vendored bytes **and** the
revision recorded above **in the same commit**.

### 1. Refresh `src/` from the git pin

```sh
git clone https://github.com/midnightntwrk/midnight-verifiable-credential-digital-passport.git /tmp/upstream-dpp
git -C /tmp/upstream-dpp checkout <NEW_REV>
rsync -a --delete /tmp/upstream-dpp/packages/midnight-verifiable-credential-digital-passport/src/ src/
# then update the pinned revision in this file, same commit
```

### 2. Refresh `core-compact-staging/` from the npm core package

Upstream's `scripts/stage-core-compact.mjs` resolves the package's exports-map
subpath `@midnight-ntwrk/credential-compact/credentials.compact` (→
`dist/credentials.compact`) and stages it plus its sibling `dist/credentials/`
tree. The same bytes are obtainable without a pnpm install:

```sh
# no-pnpm variant: fetch the published tarball straight from the registry
rm -rf /tmp/core-tarball-staging && mkdir -p /tmp/core-tarball-staging
curl -sL "$(npm view @midnight-ntwrk/credential-compact@<VERSION> dist.tarball)" | tar xz -C /tmp/core-tarball-staging
cp  /tmp/core-tarball-staging/package/dist/credentials.compact        core-compact-staging/credentials.compact
rm -rf core-compact-staging/credentials
cp -R /tmp/core-tarball-staging/package/dist/credentials              core-compact-staging/credentials
```

(If the core package version changes, also update the version recorded above in
the same commit.)

### 3. Verification

```sh
# src/ is byte-identical to the pinned upstream rev (expect: no output, exit 0)
git diff --no-index /tmp/upstream-dpp/packages/midnight-verifiable-credential-digital-passport/src src

# staging is byte-identical to the npm dist/ (expect 15 "cmp" successes)
find core-compact-staging -type f | wc -l   # → 15
cmp /tmp/core-tarball-staging/package/dist/credentials.compact core-compact-staging/credentials.compact
for f in /tmp/core-tarball-staging/package/dist/credentials/*; do
  cmp "$f" "core-compact-staging/credentials/$(basename "$f")"
done

# nothing silently dropped by .gitignore's bare dist/gen/out/artifacts patterns
git status --ignored --porcelain .          # → must be empty (run from the repo root)
git ls-files examples/dogfood | wc -l       # → 38 (22 src + 15 staged + this PROVENANCE.md)
```

The `.gitignore` truncation check matters: the repo's `.gitignore` has bare
`dist`, `gen`, `out`, `artifacts`, `node_modules`, and `result` patterns that
match directories of those names at **any** depth. At the current pin no path
segment in either tree collides, so no renames were needed; if a future refresh
introduces one, rename that segment and record the rename here.

## Neutralization note

Vendoring is deliberately **verbatim**: upstream headers, names, and branding
stay intact so the fixture remains recognizable and byte-comparable against
upstream. A future neutralization pass (renaming/rebadging) remains possible if
the branding cost ever outweighs the dogfood value; if undertaken it must be
recorded here and in the dogfood ADR as a deliberate divergence from verbatim
(which this file otherwise forbids).
