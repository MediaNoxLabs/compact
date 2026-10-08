#!/usr/bin/env bash
# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
# 	http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.

set -euo pipefail

readonly LEDGER_REV="30fd606e5652d9146e8c7d453c5c01ea03962a89"
readonly REPOSITORY_ROOT="$(git rev-parse --show-toplevel)"
readonly COMPACT_REV="${COMPACT_RUNTIME_GIT_REV:-$(git -C "${REPOSITORY_ROOT}" rev-parse HEAD)}"
readonly COMPACT_URL="${COMPACT_RUNTIME_GIT_URL:-file://${REPOSITORY_ROOT}}"
readonly SMOKE_ROOT="$(mktemp -d "${TMPDIR:-/tmp}/compact-git-consumer.XXXXXX")"

cleanup() {
  rm -rf "${SMOKE_ROOT}"
}
trap cleanup EXIT

mkdir -p "${SMOKE_ROOT}/src"

cat >"${SMOKE_ROOT}/Cargo.toml" <<EOF
[package]
name = "compact-immutable-git-consumer-smoke"
version = "0.0.0"
edition = "2021"
publish = false

[dependencies]
midnight-compact-runtime = { git = "${COMPACT_URL}", rev = "${COMPACT_REV}" }

# Cargo only honours source patches at the consuming workspace root. Ledger8
# transient crypto requires the disk-spill feature from this proofs revision.
[patch.crates-io]
midnight-proofs = { git = "https://github.com/MediaNoxLabs/midnight-zk.git", rev = "532629b044a88473a7175f4a96c2511c91156136" }
EOF

cat >"${SMOKE_ROOT}/src/lib.rs" <<'EOF'
use midnight_compact_runtime as _;
EOF

cargo metadata \
  --format-version 1 \
  --manifest-path "${SMOKE_ROOT}/Cargo.toml" \
  >"${SMOKE_ROOT}/metadata.json"

python3 - "${SMOKE_ROOT}/metadata.json" "${LEDGER_REV}" <<'PY'
import json
import pathlib
import sys

metadata_path = pathlib.Path(sys.argv[1])
ledger_rev = sys.argv[2]
metadata = json.loads(metadata_path.read_text())

ledger_packages = {
    "midnight-base-crypto",
    "midnight-transient-crypto",
    "midnight-serialize",
    "midnight-storage",
    "midnight-coin-structure",
    "midnight-onchain-state",
    "midnight-onchain-vm",
    "midnight-onchain-runtime",
    "midnight-zswap",
}

failures = []
for name in sorted(ledger_packages):
    packages = [package for package in metadata["packages"] if package["name"] == name]
    sources = sorted({package.get("source") or "path" for package in packages})
    if len(packages) != 1:
        failures.append(f"{name}: expected one package, found {len(packages)} ({sources})")
        continue
    if ledger_rev not in sources[0]:
        failures.append(f"{name}: expected Ledger revision {ledger_rev}, found {sources[0]}")

if failures:
    raise SystemExit("Ledger8 source-coherence failure:\n- " + "\n- ".join(failures))

print(f"Ledger8 source coherence verified for {len(ledger_packages)} packages at {ledger_rev}")
PY

cargo check \
  --locked \
  --manifest-path "${SMOKE_ROOT}/Cargo.toml"

echo "immutable Git consumer verified at Compact revision ${COMPACT_REV}"
