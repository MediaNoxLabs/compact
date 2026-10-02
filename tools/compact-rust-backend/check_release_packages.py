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

"""Rehearse and identify the two runtime crates before publishing either one."""

import argparse
import hashlib
from pathlib import Path
import json
import subprocess
import tarfile
import tomllib


ROOT = Path(__file__).resolve().parents[2]
MACROS = ROOT / "runtime-rs-macros"
PACKAGES = ("midnight-compact-runtime-macros", "midnight-compact-runtime")
LOCKS = ("Cargo.lock", "tools/compact-rust-backend/Cargo.lock", "flake.lock")


def run(*arguments: str) -> None:
    subprocess.run(["cargo", *arguments], cwd=ROOT, check=True)


def output(*arguments: str) -> str:
    return subprocess.check_output(arguments, cwd=ROOT, text=True).strip()


def digest(path: Path) -> str:
    with path.open("rb") as content:
        return hashlib.file_digest(content, "sha256").hexdigest()


def candidate_tag(tag: str) -> None:
    if output("git", "status", "--porcelain", "--untracked-files=normal"):
        raise RuntimeError("release candidate requires a clean worktree")
    subprocess.run(["git", "check-ref-format", f"refs/tags/{tag}"], cwd=ROOT, check=True)
    ref = f"refs/tags/{tag}"
    if output("git", "cat-file", "-t", ref) != "tag":
        raise RuntimeError(f"{tag}: release candidate requires an annotated tag")
    subprocess.run(["git", "verify-tag", ref], cwd=ROOT, check=True)
    if output("git", "rev-list", "-n1", ref) != output("git", "rev-parse", "HEAD"):
        raise RuntimeError(f"{tag}: tag does not point at HEAD")


def inspect_package(name: str) -> dict[str, object]:
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
    return {
        "name": name,
        "version": version,
        "archive": archive.relative_to(ROOT).as_posix(),
        "sha256": digest(archive),
        "bytes": archive.stat().st_size,
        "entries": len(members),
    }


def manifest(packages: list[dict[str, object]], tag: str | None) -> dict[str, object]:
    return {
        "schema": 1,
        "source": {
            "commit": output("git", "rev-parse", "HEAD"),
            "tree": output("git", "rev-parse", "HEAD^{tree}"),
            "dirty": bool(output("git", "status", "--porcelain", "--untracked-files=normal")),
            "candidate_tag": tag,
        },
        "toolchain": {
            "cargo": output("cargo", "--version"),
            "rustc": output("rustc", "--version"),
        },
        "locks": {path: digest(ROOT / path) for path in LOCKS},
        "packages": packages,
        "verification": {
            "runtime_macro_patch": "local source patch for unpacked Cargo verification",
            "registry_publication": False,
        },
    }


def first_difference(actual: object, expected: object, location: str = "manifest") -> str | None:
    if type(actual) is not type(expected):
        return location
    if isinstance(actual, dict):
        if actual.keys() != expected.keys():
            return location
        for key in actual:
            difference = first_difference(actual[key], expected[key], f"{location}.{key}")
            if difference:
                return difference
    elif isinstance(actual, list):
        if len(actual) != len(expected):
            return location
        for index, (current, saved) in enumerate(zip(actual, expected)):
            difference = first_difference(current, saved, f"{location}[{index}]")
            if difference:
                return difference
    elif actual != expected:
        return location
    return None


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    mode = parser.add_mutually_exclusive_group()
    mode.add_argument("--manifest", type=Path, help="write a release rehearsal manifest")
    mode.add_argument("--verify-manifest", type=Path, help="repackage and check a saved manifest")
    parser.add_argument("--candidate-tag", help="require a clean, signed annotated tag at HEAD")
    args = parser.parse_args()
    if args.candidate_tag and not (args.manifest or args.verify_manifest):
        parser.error("--candidate-tag requires --manifest or --verify-manifest")
    expected = None
    if args.verify_manifest:
        expected = json.loads(args.verify_manifest.read_text())
        if not isinstance(expected, dict) or expected.get("schema") != 1:
            raise RuntimeError("unsupported release manifest schema")
        source = expected.get("source")
        if not isinstance(source, dict):
            raise RuntimeError("release manifest has no source object")
        recorded_tag = source.get("candidate_tag")
        if recorded_tag is not None and not isinstance(recorded_tag, str):
            raise RuntimeError("release manifest has an invalid candidate tag")
        if args.candidate_tag and args.candidate_tag != recorded_tag:
            raise RuntimeError("candidate tag differs from saved manifest")
        args.candidate_tag = recorded_tag
    if args.candidate_tag:
        candidate_tag(args.candidate_tag)

    run("package", "-p", PACKAGES[0], "--allow-dirty")
    packages = [inspect_package(PACKAGES[0])]

    # The runtime's exact macro version must be published before an ordinary
    # cargo package can resolve it. The patch verifies the unpacked package
    # against the local source without changing the packaged manifest.
    patch = "patch.crates-io.midnight-compact-runtime-macros.path=" + json.dumps(str(MACROS))
    run("package", "-p", PACKAGES[1], "--allow-dirty", "--config", patch)
    packages.append(inspect_package(PACKAGES[1]))
    actual = manifest(packages, args.candidate_tag)
    if expected is not None:
        difference = first_difference(actual, expected)
        if difference:
            raise RuntimeError(f"release manifest mismatch at {difference}")
        print(f"verified {args.verify_manifest}")
    elif args.manifest:
        args.manifest.parent.mkdir(parents=True, exist_ok=True)
        args.manifest.write_text(json.dumps(actual, indent=2, sort_keys=True) + "\n")
        print(f"wrote {args.manifest}")
    print("release rehearsal passed; registry publication and remote consumer remain unverified")


if __name__ == "__main__":
    main()
