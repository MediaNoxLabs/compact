#!/usr/bin/env python3

# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#   http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.

"""Resolve static fixtures from the exact official ledger git cohort.

Cargo.lock, Cargo metadata, checkout HEAD, tracked manifest bytes and pinned
fixture digests must agree. Proving keys still use the official provider and
its pinned identities; this resolver never manufactures or caches proofs.
"""

import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import sys
import tomllib

VERSION = "8.1.3"
REVISION = "b5d3e965a18e4cac17fac4f38446fc55c524439a"
GIT = "https://github.com/midnightntwrk/midnight-ledger"
SOURCE = f"git+{GIT}?rev={REVISION}#{REVISION}"
FIXTURE_HASHES = {
    "spend.bzkir": "904181287e75b0fb596ba5fcc116c882ee5d28e3115304c93ebd913722ce5841",
    "spend.verifier": "3f1569ebcab0655c5c145b28947c74edc4e3f5c6b276e4404b661cf0905b49d3",
    "spend.zkir": "9274ac7708818a9a6d9e2f9caab54f9c9d639ef83166d1d200a378eaa2213115",
}
PROVER_HASH = "996602da7ca386284e656c78ea03e55bffdba29475e6a67965c50de05e13efc2"


def require(condition, message):
    if not condition:
        raise ValueError(message)


def resolve(metadata, lock):
    packages = [p for p in metadata["packages"] if p["name"] == "midnight-ledger"]
    locked = [p for p in lock["package"] if p["name"] == "midnight-ledger"]
    require(len(packages) == len(locked) == 1, "expected one midnight-ledger package")
    package, entry = packages[0], locked[0]
    for item in (package, entry):
        require(item["version"] == VERSION and item["source"] == SOURCE,
                "ledger version/source differs from audited git package")
    manifest = Path(package["manifest_path"])
    require(manifest.is_absolute(), "metadata manifest path must be absolute")
    crate = manifest.parent.resolve(strict=True)
    require(not any(c in str(crate) for c in "\r\n"), "unsafe environment path")
    checkout = crate.parent
    require(crate.name == "ledger" and not manifest.is_symlink(),
            "resolved manifest identity mismatch")
    def git(*args):
        return subprocess.run(["git", "-C", str(checkout), *args],
                              check=True, capture_output=True).stdout
    require(git("rev-parse", "HEAD").decode().strip() == REVISION,
            "ledger checkout VCS identity mismatch")
    require(Path(git("rev-parse", "--show-toplevel").decode().strip()).resolve() == checkout,
            "ledger checkout root mismatch")
    identity = tomllib.loads(manifest.read_text())["package"]
    workspace = checkout / "Cargo.toml"
    require(identity["name"] == "midnight-ledger"
            and identity["version"] == {"workspace": True}
            and tomllib.loads(workspace.read_text())["workspace"]["package"]["version"] == VERSION,
            "resolved manifest identity mismatch")
    for relative in ("Cargo.toml", "ledger/Cargo.toml"):
        path = checkout / relative
        require(not path.is_symlink() and path.read_bytes() == git("show", f"{REVISION}:{relative}"),
                f"ledger tracked manifest differs from pinned source: {relative}")
    static = crate / "static"
    require(not any(c in str(static) for c in "\r\n"), "unsafe environment path")
    for name, digest in FIXTURE_HASHES.items():
        require(hashlib.sha256((static / "dust" / name).read_bytes()).hexdigest() == digest,
                f"static fixture digest mismatch: {name}")
        require((static / "dust" / (name + ".sha256")).read_text().split()[0] == digest,
                f"static fixture digest declaration mismatch: {name}")
    require((static / "dust/spend.prover.sha256").read_text().split()[0] == PROVER_HASH,
            "upstream prover digest declaration mismatch")
    return {
        "format": "compact-ledger-test-static/v1",
        "package": "midnight-ledger", "version": VERSION, "source": SOURCE,
        "vcs_revision": REVISION,
        "static_directory": str(static), "fixture_sha256": FIXTURE_HASHES,
        "upstream_prover_sha256": PROVER_HASH,
        "scope": "External test resolver base; upstream provider supplies and verifies proving material",
    }


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--metadata", type=Path, required=True)
    parser.add_argument("--lock", type=Path, required=True)
    parser.add_argument("--provenance", type=Path, required=True)
    parser.add_argument("--github-env", type=Path)
    args = parser.parse_args(argv)
    try:
        record = resolve(json.loads(args.metadata.read_text()),
                         tomllib.loads(args.lock.read_text()))
        record["cargo_lock_sha256"] = hashlib.sha256(args.lock.read_bytes()).hexdigest()
        # All validation finishes before either output is written.
        args.provenance.write_text(json.dumps(record, indent=2) + "\n")
        if args.github_env:
            with args.github_env.open("a") as output:
                print(f"MIDNIGHT_LEDGER_TEST_STATIC_DIR={record['static_directory']}", file=output)
        print(record["static_directory"])
        return 0
    except (OSError, ValueError, KeyError, TypeError, IndexError, subprocess.CalledProcessError) as error:
        print(f"ledger static fixture resolution failed: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
