<!--
This file is part of Compact.
Copyright (C) 2026 Midnight Foundation
SPDX-License-Identifier: Apache-2.0
-->

# Delta Spec: dogfooding/digital-passport-fixture

## MODIFIED Requirements

### Requirement: Vendored upstream source at a pinned revision

The repo MUST contain the upstream contract's Compact sources byte-identical to upstream rev `cdeb860b` under `examples/dogfood/digital-passport-credential/`: the `*.compact` subset of the upstream package `src/` tree (6 files: the entry `digital-passport-credential.compact` and the five modules under `digital-passport-credential/`) plus `core-compact-staging/` (15 files from `@midnight-ntwrk/credential-compact@0.1.0-rc3` `dist/`). A `PROVENANCE.md` MUST record the upstream URL, revision, npm core package/version, the vendored-file manifest, and the refresh procedure. The vendored sources MUST NOT be locally modified — any upstream divergence is picked up only by an explicit, recorded re-sync.

TypeScript material from the upstream package MUST NOT be vendored: no gate compiles, executes, or reads it, and its exclusion is a recorded, deliberate divergence from whole-tree verbatim (recorded in `PROVENANCE.md` and ADR 0003's follow-up note).

Because the vendored set is a subset rather than the whole `src/` tree, byte-verification MUST be manifest-based: each vendored `.compact` file byte-identical to its upstream counterpart at the pinned revision, **and** the vendored `src/**/*.compact` path set equal to the upstream path set at that revision, so a newly added upstream `.compact` file cannot be silently missed on refresh.

#### Scenario: provenance is answerable
- **WHEN** a reader asks "what code is this and where did it come from"
- **THEN** `PROVENANCE.md` states the upstream repo, exact rev, npm core package+version, the vendored-file manifest, and how to refresh both

#### Scenario: refresh is an explicit act
- **WHEN** upstream moves after the pin
- **THEN** nothing in this repo changes until a human re-syncs and the PROVENANCE revision is updated in the same commit

#### Scenario: the subset is complete and per-file verbatim
- **WHEN** the refresh verification procedure in `PROVENANCE.md` runs
- **THEN** every vendored `.compact` file is byte-identical (`cmp`) to its counterpart in the upstream tree at the pinned rev, and the vendored `src/**/*.compact` file list matches the upstream list exactly (added/renamed upstream modules surface as a list diff, not silent absence)

#### Scenario: the enclave carries no TypeScript
- **WHEN** the enclave tree `examples/dogfood/digital-passport-credential/` is inspected
- **THEN** it contains only `.compact` files and `PROVENANCE.md` (22 files: 6 `src/` + 15 staged + `PROVENANCE.md`); no `.ts` file exists under the enclave
