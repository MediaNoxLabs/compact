---
id: RUST-ADR-0350
alias: ADR-0350
source_sha256: 7b458575b5cb5a651bc8b592b506b6bb89e3193d56195a9aeaf36e7fb093665d
publication_status: published
publication_date: 2026-10-09
publication_source_revision: f9a496660a4a69f4dd7c7927111767ad5c0c12b2
---
# RUST-ADR-0350 — Stage a portable developer documentation package for closeout

[ADR register](README.md) · [0.3.0 record register](index-0.3.0.md) · [Reference conventions](references-0.3.0.md)

## Publication disposition

This published historical source record belongs to the milestone 0.3.0 documentation collection. **Original opening status:** staged and independently reviewed; publication remains at milestone closeout. Read dated amendments below; neither this status nor issue closure certifies the whole milestone. No fresh implementation, security, proof or formal acceptance is implied. Original code examples and local evidence locators are historical; machine-local paths are not portable downloads or current setup instructions.



## Original decision and amendments

## ADR0350 — Stage a portable developer documentation package for closeout

Date: 2026-10-08
Status: staged and independently reviewed; publication remains at milestone closeout
Parents: ADR0324/#449, R03008/#352

### Problem

Reviewed guides and runnable examples remain in Obsidian, with vault links, local receipt paths and incomplete repository entry points. Copying them only at the last moment would leave relocation and provenance defects for release qualification. Current backend ABI labels are already corrected at74d04c1a.

### Decision and before/after

Before: Current edition plus companion vault notes and scattered historical examples/evidence.
After: a staged repository-shaped package with an explicit destination manifest, portable links, complete selected example sources, retained evidence identities and entrypoint changes. Keep the package and planning in midnight; do not apply it to the branch before closeout.

Layout: `doc/rust/guides/` for current task guides, `doc/rust/guides/examples/` for complete runnable consumer sources, `doc/rust/evidence/0.3.0/` for bounded text receipts/logs and their portable index, and a short `testkit-rs/README.md`. ADRs retain the independent `doc/rust/adr/` series. Existing backend/runtime/root navigation changes are staged as explicit patches against current bytes.

### Evidence and compatibility

Keep measured historical receipt bytes unchanged. A relocation index maps original locations and hashes to published destinations; do not rewrite a historic receipt to claim new execution. Copy only evidence necessary for guide claims, not targets, SDK build trees, executables or proof material. Preserve complete handwritten example bodies. Manifest/runtime-path adaptations must be recorded; do not claim an adapted consumer ran unless it did. Keep existing qualified SDK preparation layout where possible.

Keep current interface guidance apart from historical measurement/version records. Installation links point to the actual existing build/archive workflow; invent no release download or registry entry. Explanatory proof/signature excerpts remain labeled. No new safety/assurance claim or stopped ADR0285/#409 investigation.

### Acceptance

- All selected Markdown navigation resolves in the staged overlay plus current repository; no unresolved wiki or machine-local navigation remains in current guides.
- Destination/source manifest binds exact bytes and intentional transformations; complete example source references resolve.
- New testkit/root/runtime/backend entry points lead to task guides and retain history.
- Independent review checks relocation, retained evidence attribution and runnable-versus-illustrative labels.
- Save a verified staging archive and readable map in midnight. Repository publication and final candidate qualification remain open; do not close #352 or declare the milestone complete.

No code/runtime/emitter change or tutorial rebuild expected unless staging reveals a material executable adaptation. Conformance time box ended early; this work concerns approved documentation closure.

### ADR0350/#478 documentation staging delivered — 2026-10-08

[ADR0350 — Portable documentation publication staging](references-0.3.0.md#note-127) records a144-file repository overlay, portable guide/example layout and evidence index. All181 current local links pass, including relocated execution of the checker. Handwritten executable sources/manifests/locks are unchanged;72 selected historical artifacts are byte-preserved. Independent review accepted staging only. Existing entrypoint patch applies cleanly; no repository edits or new builds.

Archive [ADR0350 — Portable documentation publication staging.zip](references-0.3.0.md#note-128), SHA256 `b74e0e3e27e6a1705c0e17340fb88b427b4c10f515a3a6f6fee54bccd16dc467`, 169 verified members. #449/#352 still need closeout publication and final-candidate qualification. Parent count12/19; #409 scope question pending; no acceptance inferred.
