#!/usr/bin/env python3
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

"""Acquire exact historical self-update assets for the current test platform."""

import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import shutil
import tempfile
import urllib.request


MANIFEST = Path(__file__).with_name("manifest.json")
VERSIONS = ("0.5.0", "0.5.1")


def current_target() -> str:
    system, machine = platform.system(), platform.machine()
    if system == "Darwin" and machine in ("arm64", "aarch64"):
        return "aarch64-apple-darwin"
    if system == "Darwin" and machine == "x86_64":
        return "x86_64-apple-darwin"
    if system == "Linux" and machine == "x86_64":
        return "x86_64-unknown-linux-musl"
    raise ValueError(f"no pinned historical self-update assets for {system}/{machine}")


def digest(path: Path) -> str:
    hasher = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            hasher.update(chunk)
    return hasher.hexdigest()


def matches(path: Path, spec: dict) -> bool:
    return (
        path.is_file()
        and not path.is_symlink()
        and path.stat().st_size == spec["bytes"]
        and digest(path) == spec["sha256"]
    )


def acquire(version: str, spec: dict, output: Path, source_dir: Path | None) -> dict:
    release = f"compact-v{version}"
    destination = output / release / spec["name"]
    destination.parent.mkdir(parents=True, exist_ok=True)
    if matches(destination, spec):
        origin = "verified-cache"
    else:
        if destination.exists() or destination.is_symlink():
            raise ValueError(f"existing asset fails pinned byte count or SHA256: {destination}")
        descriptor, temporary_name = tempfile.mkstemp(
            prefix=f".{destination.name}.partial-", dir=destination.parent
        )
        os.close(descriptor)
        temporary = Path(temporary_name)
        try:
            if source_dir is not None:
                source = source_dir / release / spec["name"]
                if not matches(source, spec):
                    raise ValueError(f"source asset fails pinned byte count or SHA256: {source}")
                shutil.copyfile(source, temporary)
                origin = "verified-local-source"
            else:
                url = (
                    "https://github.com/midnightntwrk/compact/releases/download/"
                    f"{release}/{spec['name']}"
                )
                request = urllib.request.Request(url, headers={"User-Agent": "compact-self-update-test/1"})
                with urllib.request.urlopen(request, timeout=60) as response, temporary.open("wb") as sink:
                    transferred = 0
                    while chunk := response.read(min(1024 * 1024, spec["bytes"] - transferred + 1)):
                        transferred += len(chunk)
                        if transferred > spec["bytes"]:
                            raise ValueError(f"release asset exceeds pinned byte count: {release}/{spec['name']}")
                        sink.write(chunk)
                origin = "official-release-download"
            if not matches(temporary, spec):
                raise ValueError(f"downloaded asset fails pinned byte count or SHA256: {release}/{spec['name']}")
            temporary.replace(destination)
        finally:
            if temporary.exists() or temporary.is_symlink():
                temporary.unlink()
    return {
        "release": release,
        "name": spec["name"],
        "bytes": spec["bytes"],
        "sha256": spec["sha256"],
        "path": str(destination.resolve()),
        "origin": origin,
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--source-dir", type=Path, help="copy pre-acquired assets instead of downloading")
    args = parser.parse_args()
    manifest = json.loads(MANIFEST.read_text())
    if manifest["format"] != "compact-self-update-assets/v1" or manifest["repository"] != "midnightntwrk/compact":
        raise ValueError("unsupported historical self-update asset manifest")
    target = current_target()
    output = args.output.resolve()
    source_dir = args.source_dir.resolve() if args.source_dir else None
    assets = []
    for version in VERSIONS:
        release = manifest["releases"][version]
        assets.append(acquire(version, release["installer"], output, source_dir))
        assets.append(acquire(version, release["archives"][target], output, source_dir))
    receipt = {
        "format": "compact-self-update-acquisition/v1",
        "repository": manifest["repository"],
        "target": target,
        "manifest_sha256": digest(MANIFEST),
        "assets": assets,
    }
    output.mkdir(parents=True, exist_ok=True)
    (output / "receipt.json").write_text(json.dumps(receipt, indent=2) + "\n")
    print(json.dumps(receipt, indent=2))


if __name__ == "__main__":
    main()
