# Pinned historical Compact self-update assets

`manifest.json` pins the official `midnightntwrk/compact` `compact-v0.5.0` and
`compact-v0.5.1` installers and archives for the three platforms in
`compact-test.yml`. Asset byte counts and SHA256 digests match GitHub release
metadata; archive digests also match each release's cargo-dist
`dist-manifest.json` and published checksum file. ADR-0230 records the
acquisition and validation evidence.

Acquire only the current platform's four required files before the Rust tests:

```sh
python3 tools/compact/tests/fixtures/self_update/acquire.py \
  --output "$RUNNER_TEMP/compact-self-update-assets"
export COMPACT_SELF_ASSETS_DIR="$RUNNER_TEMP/compact-self-update-assets"
```

The helper validates cached files on every run and refuses changed bytes. It
writes an acquisition receipt to `$COMPACT_SELF_ASSETS_DIR/receipt.json`. The
Rust fixture validates the four files again, serves them from closed localhost
routes, and executes the unchanged historical installer and binary with a
private home, install prefix and receipt. No release asset is checked into the
repository. Only Apple Silicon macOS was dynamically run during development;
the x86 macOS and Linux runners exercise their pinned rows in the configured
test matrix.
