#!/usr/bin/env python3

# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#  	http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.

"""Rehearse the two runtime crate packages before publishing either one."""

from pathlib import Path
import json
import subprocess
import tarfile
import tomllib


ROOT = Path(__file__).resolve().parents[2]
MACROS = ROOT / "runtime-rs-macros"
PACKAGES = ("midnight-compact-runtime-macros", "midnight-compact-runtime")


def run(*arguments: str) -> None:
    subprocess.run(["cargo", *arguments], cwd=ROOT, check=True)


def inspect_package(name: str) -> None:
    source_dir = ROOT / ("runtime-rs-macros" if name.endswith("-macros") else "runtime-rs")
    manifest = tomllib.loads((source_dir / "Cargo.toml").read_text())
    version = manifest["package"]["version"]
    archive = ROOT / "target" / "package" / f"{name}-{version}.crate"
    prefix = f"{name}-{version}/"
    with tarfile.open(archive, "r:gz") as package:
        members = package.getnames()
        if not members or any(not member.startswith(prefix) for member in members):
            raise RuntimeError(f"{archive}: unexpected archive paths")
        for required in ("Cargo.toml", "LICENSE", "README.md", "src/lib.rs"):
            if prefix + required not in members:
                raise RuntimeError(f"{archive}: missing {required}")
        if name == "midnight-compact-runtime":
            packaged_manifest = package.extractfile(prefix + "Cargo.toml")
            if packaged_manifest is None:
                raise RuntimeError(f"{archive}: missing packaged manifest")
            dependency = tomllib.loads(packaged_manifest.read().decode())["dependencies"][
                "midnight-compact-runtime-macros"
            ]
            if dependency.get("version") != "=0.1.0" or "path" in dependency:
                raise RuntimeError(f"{archive}: macro dependency is not publishable")
    print(f"checked {archive.relative_to(ROOT)} ({len(members)} entries)", flush=True)


def main() -> None:
    run("package", "-p", PACKAGES[0], "--allow-dirty")
    inspect_package(PACKAGES[0])

    # The runtime's exact macro version must be published before an ordinary
    # cargo package can resolve it. The patch verifies the unpacked package
    # against the local source without changing the packaged manifest.
    patch = "patch.crates-io.midnight-compact-runtime-macros.path=" + json.dumps(str(MACROS))
    run("package", "-p", PACKAGES[1], "--allow-dirty", "--config", patch)
    inspect_package(PACKAGES[1])
    print("release rehearsal passed; registry publication and remote consumer remain unverified")


if __name__ == "__main__":
    main()
