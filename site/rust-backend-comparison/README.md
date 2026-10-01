# Compact Rust backend atlas

A self-contained static site comparing the local typed Rust backend at
`2f93774e` with the unmodified `codegen-rust` oracle at `589c92ef`. Both
descend from `ledger-8` at `eb72a5ab`.

## Preview

From this directory:

```sh
python3 -m http.server 4173
```

Open `http://localhost:4173/`. The site uses relative assets and no runtime
dependencies. Upload this directory unchanged to a static host such as GitHub
Pages, Netlify, or any ordinary web server. Set the host's publish directory to
`site/rust-backend-comparison` when deploying from this repository.

## Content and evidence

- The architecture, domain model, type mapping, runtime, macro, packaging, and
  test comparisons come from source inspection at the pinned commits above.
- The source line counts exclude generated fixture libraries. They include
  comments and are presented as codebase shape, never as a quality score.
- The local fixture and test results refer to the last completed local gates:
  128 generated outputs current, 410 workspace test/doc-test groups passed.
- The oracle test suite was inspected but not re-run for this site. Fixture
  counts do not imply whole-language parity.
- The local `compact-rustc` forces `--skip-zk`; deployable proving artifacts
  are outside the verified comparison scope.

Primary local sources: `compiler/rust-ir-passes.ss`,
`tools/compact-rust-backend/src`, `runtime-rs/src`, `runtime-rs-macros/src`,
and `tests-rust-backend`. Primary oracle sources: `compiler/rust-passes*.ss`,
`runtime-rs/src`, `runtime-rs-macros/src`, and `tests-e2e-rust` at
[`589c92ef`](https://github.com/MediaNoxLabs/compact/tree/589c92ef961ce89ac730140e81b76d4c08222a70).

The fuller written comparison is saved in the `midnight` Obsidian vault at
`Initiatives/02 Compact Rust emission/Rust backend architecture comparison — 2026-10-01.md`.
