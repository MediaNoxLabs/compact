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

"""Acquire pinned genuine compiler ZIPs; never execute or install their contents."""

import argparse
import hashlib
import json
import os
import platform
from pathlib import Path
import re
import shutil
import sys
import tempfile
import urllib.request

FORMAT = "compact-real-compiler-fixtures/v1"
PLATFORMS = {"aarch64-darwin", "x86_64-apple-darwin", "x86_64-unknown-linux-musl", "aarch64-unknown-linux-musl"}


def require(value, message):
    if not value:
        raise ValueError(message)


def load_manifest(path):
    data = json.loads(path.read_text())
    require(data["format"] == FORMAT and data["repository"] == "midnightntwrk/compact", "wrong manifest identity")
    require(set(data["platforms"]) == PLATFORMS and len(data["platforms"]) == len(PLATFORMS), "wrong platform set")
    versions = data["install_versions"]
    require(versions and len(set(versions)) == len(versions), "duplicate or empty install versions")
    catalogue = {}
    ids = set()
    for release in data["catalogue"]:
        version = release["version"]
        require(re.fullmatch(r"\d+\.\d+\.\d+", version) and version not in catalogue, "invalid or duplicate release")
        require(type(release["release_id"]) is int and release["release_id"] > 0, "invalid release id")
        require(set(release["platforms"]) <= PLATFORMS, "unknown release platform")
        for asset in release["platforms"].values():
            require(type(asset["id"]) is int and asset["id"] > 0 and asset["id"] not in ids, "invalid or duplicate asset id")
            ids.add(asset["id"])
            require(Path(asset["name"]).name == asset["name"] and asset["name"].endswith(".zip"), "unsafe archive name")
            require(asset["browser_download_url"] == f'https://github.com/midnightntwrk/compact/releases/download/compactc-v{version}/{asset["name"]}', "noncanonical asset URL")
        catalogue[version] = release
    require(set(versions) <= catalogue.keys(), "missing install release")
    expected = {(v, p) for v in versions for p in catalogue[v]["platforms"]}
    found = set()
    for row in data["archives"]:
        key = row["version"], row["platform"]
        require(key in expected and key not in found, "unknown or duplicate archive row")
        found.add(key)
        asset = catalogue[key[0]]["platforms"][key[1]]
        for field, asset_field in [("asset_id", "id"), ("name", "name"), ("url", "browser_download_url"), ("size", "size")]:
            require(row[field] == asset[asset_field], "archive/catalogue identity mismatch")
        require(row["release_id"] == catalogue[key[0]]["release_id"], "release id mismatch")
        require(type(row["size"]) is int and row["size"] > 0, "invalid archive size")
        require(re.fullmatch(r"[0-9a-f]{64}", row["sha256"]), "invalid archive SHA256")
        reported = row["reported_digest"]
        require(reported is None or reported == "sha256:" + row["sha256"], "publisher digest mismatch")
        provenance = "github-reported-sha256" if reported else "sha256-of-official-download-no-publisher-digest"
        require(row["hash_provenance"] == provenance, "digest provenance mismatch")
    require(found == expected, "missing required archive pin")
    return data


def verify(path, row):
    require(path.is_file() and not path.is_symlink(), f"missing regular fixture archive: {path}")
    require(path.stat().st_size == row["size"], f"fixture size mismatch: {path}")
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    require(digest.hexdigest() == row["sha256"], f"fixture SHA256 mismatch: {path}")


def acquire(row, cache, seed=None, verify_only=False, opener=urllib.request.urlopen):
    dest = cache / (row["sha256"] + ".zip")
    if dest.exists() or dest.is_symlink():
        verify(dest, row)
        return dest, "verified-cache"
    require(not verify_only, f"fixture not acquired: {row['version']} {row['platform']}; run acquire_compilers.py first")
    cache.mkdir(parents=True, exist_ok=True)
    fd, name = tempfile.mkstemp(prefix=".partial-", dir=cache)
    temporary = Path(name)
    try:
        with os.fdopen(fd, "wb") as output:
            source = seed / ("compactc-v" + row["version"]) / row["name"] if seed else None
            if source and source.is_file():
                verify(source, row)
                with source.open("rb") as stream:
                    shutil.copyfileobj(stream, output)
                origin = "verified-seed"
            else:
                request = urllib.request.Request(row["url"], headers={"User-Agent": "Compact-installer-fixture-acquisition"})
                with opener(request, timeout=120) as stream:
                    size = 0
                    for chunk in iter(lambda: stream.read(1024 * 1024), b""):
                        size += len(chunk)
                        require(size <= row["size"], "download exceeds pinned size")
                        output.write(chunk)
                origin = "official-download"
        verify(temporary, row)
        temporary.chmod(0o444)
        os.replace(temporary, dest)
        return dest, origin
    finally:
        temporary.unlink(missing_ok=True)


def host_platform(system=None, machine=None):
    key = (system or platform.system(), machine or platform.machine())
    mapping = {
        ("Darwin", "arm64"): "aarch64-darwin",
        ("Darwin", "x86_64"): "x86_64-apple-darwin",
        ("Linux", "x86_64"): "x86_64-unknown-linux-musl",
        ("Linux", "aarch64"): "aarch64-unknown-linux-musl",
    }
    require(key in mapping, f"unsupported fixture host: {key}")
    return mapping[key]


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--manifest", type=Path, default=Path(__file__).with_name("compiler-archives.json"))
    parser.add_argument("--cache", type=Path, required=True)
    parser.add_argument("--platform", choices=["auto", *sorted(PLATFORMS)], required=True)
    parser.add_argument("--seed-dir", type=Path)
    parser.add_argument("--verify-only", action="store_true")
    parser.add_argument("--receipt", type=Path)
    args = parser.parse_args(argv)
    try:
        data = load_manifest(args.manifest)
        if args.platform == "auto":
            args.platform = host_platform()
        cache = args.cache.resolve()
        rows = []
        for row in data["archives"]:
            if row["platform"] == args.platform:
                path, mode = acquire(row, cache, args.seed_dir, args.verify_only)
                rows.append(dict(row, path=str(path), acquisition=mode))
        require(rows, "no archives selected")
        receipt = {"format": FORMAT, "platform": args.platform, "manifest_sha256": hashlib.sha256(args.manifest.read_bytes()).hexdigest(), "archives": rows, "execution": "none; byte validation only"}
        text = json.dumps(receipt, indent=2) + "\n"
        if args.receipt:
            args.receipt.write_text(text)
        print(text, end="")
        return 0
    except (OSError, ValueError, KeyError, TypeError) as error:
        print(f"compiler fixture acquisition refused: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
