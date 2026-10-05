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

"""Prepare only the public parameters needed by existing counter installer tests.

Network access is explicit setup; --verify-only never downloads. No compiler,
installer, or proof tool is executed here. Historical and current families remain
separate, with upstream source hashes also found in the pinned archive tools.
"""

import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import sys
import tempfile
import urllib.request
import zipfile

from acquire_compilers import host_platform, load_manifest, require, verify

ORIGIN = "https://midnight-s3-fileshare-dev-eu-west-1.s3.eu-west-1.amazonaws.com/"
SOURCE_PATH = "base-crypto/src/data_provider.rs"
FILECOIN_SOURCE = "https://github.com/midnightntwrk/midnight-ledger/blob/a81e393e81e2fa6e7e6ff89d0ed470c7be5e74c4/" + SOURCE_PATH
MIDNIGHT_SOURCE = "https://github.com/midnightntwrk/midnight-ledger/blob/0b1e7be93a0709d433212f28f16f9a96e0285e2b/" + SOURCE_PATH
PARAMETERS = [
    {"name": "bls_filecoin_2p10", "size": 196996,
     "sha256": "d1a3403c1f8669e82ed28d9391e13011aea76801b28fe14b42bf76d141b4efa2",
     "compiler_versions": ["0.24.0"], "source": FILECOIN_SOURCE},
    {"name": "bls_midnight_2p5", "size": 6532,
     "sha256": "0a1c9229f315fc1868ff25f668fb83aec4d09f4f23a706b5197c692c619d72c6",
     "compiler_versions": ["0.30.0", "0.31.0"], "source": MIDNIGHT_SOURCE},
]


def acquire_parameter(row, cache, seed=None, verify_only=False, opener=urllib.request.urlopen):
    dest = cache / row["name"]
    if dest.exists() or dest.is_symlink():
        verify(dest, row)
        return dest, "verified-cache"
    require(not verify_only, f"missing public parameter {dest}; run acquire_parameters.py first")
    cache.mkdir(parents=True, exist_ok=True)
    fd, name = tempfile.mkstemp(prefix=".partial-", dir=cache)
    temporary = Path(name)
    try:
        with os.fdopen(fd, "wb") as output:
            source = seed / row["name"] if seed else None
            if source and source.exists():
                # Explicit read-only seed may itself be a Nix-store symlink.
                source = source.resolve()
                verify(source, row)
                with source.open("rb") as stream:
                    shutil.copyfileobj(stream, output)
                origin = "verified-seed"
            else:
                request = urllib.request.Request(ORIGIN + row["name"], headers={"User-Agent": "Compact-counter-parameter-setup"})
                with opener(request, timeout=120) as stream:
                    data = stream.read(row["size"] + 1)
                    require(len(data) == row["size"], "public parameter size mismatch")
                    output.write(data)
                origin = "official-download"
        verify(temporary, row)
        temporary.chmod(0o444)
        os.replace(temporary, dest)
        return dest, origin
    finally:
        temporary.unlink(missing_ok=True)


def archive_tools(manifest, archive_cache, platform):
    tools = []
    for row in manifest["archives"]:
        parameter = next((p for p in PARAMETERS if row["version"] in p["compiler_versions"]), None)
        if row["platform"] != platform or (parameter is None and row["version"] != "0.22.0"):
            continue
        archive = archive_cache / (row["sha256"] + ".zip")
        verify(archive, row)
        with zipfile.ZipFile(archive) as bundle:
            tool = bundle.read("zkir")
            if row["version"] == "0.22.0":
                # This older wrapper binds its own bundled SRS via ZKIR_PP;
                # it predates the managed-data provider and MIDNIGHT_PP.
                require(b'export ZKIR_PP="$thisdir"' in bundle.read("compactc"),
                        "old compiler wrapper no longer selects bundled parameters")
                bundled = []
                for name in ["kzg", "kzg.vp"]:
                    data = bundle.read(name)
                    bundled.append({"name": name, "size": len(data),
                                    "sha256": hashlib.sha256(data).hexdigest()})
                tools.append({"compiler_version": row["version"], "platform": platform,
                              "archive_sha256": row["sha256"], "zkir_sha256": hashlib.sha256(tool).hexdigest(),
                              "parameter_source": "archive-bundled-ZKIR_PP", "bundled_parameters": bundled})
                continue
        # Compare the authoritative expected-hash bytes embedded in the genuine
        # historical/current verifier's managed-data table, without executing it.
        require(bytes.fromhex(parameter["sha256"]) in tool,
                f"pinned zkir lacks expected parameter digest: {row['version']} {platform}")
        tools.append({"compiler_version": row["version"], "platform": platform,
                      "archive_sha256": row["sha256"], "zkir_sha256": hashlib.sha256(tool).hexdigest(),
                      "parameter": parameter["name"], "expected_digest_embedded": True})
    require(tools, "no selected counter tools")
    return tools


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--cache", type=Path, required=True)
    parser.add_argument("--archive-cache", type=Path, required=True)
    parser.add_argument("--platform", default="auto")
    parser.add_argument("--manifest", type=Path, default=Path(__file__).with_name("compiler-archives.json"))
    parser.add_argument("--seed-dir", type=Path)
    parser.add_argument("--verify-only", action="store_true")
    parser.add_argument("--receipt", type=Path)
    args = parser.parse_args(argv)
    try:
        manifest = load_manifest(args.manifest)
        platform = host_platform() if args.platform == "auto" else args.platform
        require(platform in manifest["platforms"], "unknown platform")
        tools = archive_tools(manifest, args.archive_cache.resolve(), platform)
        selected = {tool["parameter"] for tool in tools if "parameter" in tool}
        rows = []
        for row in PARAMETERS:
            if row["name"] not in selected:
                continue
            path, origin = acquire_parameter(row, args.cache.resolve(), args.seed_dir, args.verify_only)
            rows.append(dict(row, path=str(path), acquisition=origin, url=ORIGIN + row["name"], hash_provenance="upstream-source-and-embedded-tool-digest"))
        receipt = {"format": "compact-counter-public-parameters/v1", "platform": platform,
                   "manifest_sha256": hashlib.sha256(args.manifest.read_bytes()).hexdigest(),
                   "tools": tools, "parameters": rows, "execution": "none; archive and parameter byte validation only"}
        text = json.dumps(receipt, indent=2) + "\n"
        if args.receipt:
            args.receipt.write_text(text)
        print(text, end="")
        return 0
    except (OSError, ValueError, KeyError, TypeError, zipfile.BadZipFile) as error:
        print(f"counter parameter setup refused: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
