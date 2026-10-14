# Compact Rust-AST engineering storybook

A dependency-free, single-page visualization of the Compact Rust backend
journey: the ledger-8 foundation, TypeScript/JavaScript reference path,
original Scheme-to-Rust text emitter, typed Rust-AST backend and current
recording-effect-algebra prototype.

The narrative is intentionally bounded. Ledger-9 is shown as not ready;
prototype commit `f3cef799` is a local evidence cutoff rather than a published
release; and inventory, native execution, recording, replay, proof and ledger
application remain separate claims.

## Preview locally

From this directory:

```sh
python3 -m http.server 4173
```

Open <http://localhost:4173/>. No package manager, build tool, fonts, analytics
or network requests are required.

To preview the packaged directory instead:

```sh
python3 package.py
python3 -m http.server 4173 --directory dist
```

## Validate and package

```sh
python3 check.py
python3 package.py
```

`check.py` validates local assets, fragment links, milestone commit links and
the absence of machine-local references. `package.py` creates:

- `dist/`, ready for any static host;
- `docs/` at the repository root, ready for GitHub Pages branch publishing;
- `artifacts/compact-rust-ast-storybook.zip` at the repository root;
- a SHA-256 checksum beside the archive.

`dist/` and `artifacts/` are ignored generated outputs. The branch intentionally
tracks `docs/` so GitHub Pages can serve it without a build workflow. Packaging
is local and does not push, deploy or mutate a remote repository.

## Publish later

For GitHub Pages, push the dedicated `rust-ast-docs` branch when it is ready,
then select **Deploy from a branch → `rust-ast-docs` → `/docs`** in repository
Pages settings. No GitHub Actions workflow is needed. The site uses only
relative paths, so it works at the repository subpath. The published directory
contains:

- `index.html`
- `styles.css`
- `data.js`
- `app.js`
- `.nojekyll`

No workflow is added here; the branch remains isolated from
`codex/rust-backend-ast` until maintainers decide to merge it.

## Evidence

The complete written narrative and source register live at
[`doc/rust/journey`](../../doc/rust/journey/README.md). Important public
milestones link directly to their commits in the page. The current effect
prototype cutoff is local and therefore deliberately has no fabricated remote
commit URL.
