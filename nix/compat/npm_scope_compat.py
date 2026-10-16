#!/usr/bin/env python3
# This file is part of Compact.
# Copyright (C) 2026 contributors to Minokawa Compact
# SPDX-License-Identifier: Apache-2.0
# Licensed under the Apache License, Version 2.0 (the "License");
# You may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#	http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.

"""Adapt npm names for the two fixed ledger 8.1.3 WASM package profiles.

This is a Nix packaging boundary, not a dependency resolver or lockfile repair.
The real upstream package and lock must already describe the expected profile.
"""

import argparse
import copy
import gzip
import hashlib
import io
import json
from pathlib import Path
import shutil
import stat
import tarfile
from typing import NamedTuple
from types import MappingProxyType


class PackageProfile(NamedTuple):
    package: str
    version: str

    @property
    def upstream_name(self):
        return f"@midnightntwrk/{self.package}"

    @property
    def compatibility_name(self):
        return f"@midnight-ntwrk/{self.package}"

    @property
    def lib_path(self):
        return Path("lib/node_modules") / self.compatibility_name

    @property
    def tar_path(self):
        return Path("lib") / f"midnight-{self.package}-{self.version}.tgz"


# This is an explicit compatibility cohort, not arbitrary name/version rewriting.
PROFILES = MappingProxyType({
    "onchain-runtime-v3": PackageProfile("onchain-runtime-v3", "3.1.2"),
    "zkir-v2": PackageProfile("zkir-v2", "2.1.1"),
})
METADATA = {"package.json", "package-lock.json"}


class ProfileMismatch(ValueError):
    """The source is not the specifically supported upstream package."""


def require(condition, message):
    if not condition:
        raise ProfileMismatch(message)


def select_profile(name):
    require(name in PROFILES, f"unsupported package profile: {name}")
    return PROFILES[name]


def no_duplicate_keys(pairs):
    result = {}
    for key, value in pairs:
        require(key not in result, f"duplicate JSON key: {key}")
        result[key] = value
    return result


def parse_metadata(data):
    try:
        result = json.loads(data, object_pairs_hook=no_duplicate_keys)
    except (UnicodeDecodeError, json.JSONDecodeError) as error:
        raise ProfileMismatch(f"invalid package metadata: {error}") from error
    require(isinstance(result, dict), "package metadata must be an object")
    return result


def inventory(root):
    """Read all regular payloads; refuse links and special filesystem objects."""
    require(root.is_dir() and not root.is_symlink(), "source must be a real directory")
    files = {}
    for path in sorted(root.rglob("*")):
        mode = path.lstat().st_mode
        require(not stat.S_ISLNK(mode), f"symlink in package: {path.relative_to(root)}")
        if stat.S_ISDIR(mode):
            continue
        require(stat.S_ISREG(mode), f"special file in package: {path.relative_to(root)}")
        files[path.relative_to(root).as_posix()] = path.read_bytes()
    return files


def rename_metadata(files, profile):
    require(METADATA <= files.keys(), "upstream package.json and package-lock.json are required")
    package = parse_metadata(files["package.json"])
    lock = parse_metadata(files["package-lock.json"])
    require(package.get("name") == profile.upstream_name, "unexpected upstream package name")
    require(package.get("version") == profile.version, "unexpected upstream package version")
    for field in ("dependencies", "optionalDependencies", "peerDependencies", "devDependencies"):
        require(not package.get(field), f"unexpected upstream {field}")
    require(lock.get("name") == profile.upstream_name and lock.get("version") == profile.version,
            "lock root name/version does not match the expected package")
    require(lock.get("lockfileVersion") == 3, "expected lockfileVersion 3")
    require(lock.get("packages") == {"": {"name": profile.upstream_name, "version": profile.version}},
            "expected the upstream dependency-free lock package")
    require(set(lock) == {"name", "version", "lockfileVersion", "requires", "packages"}
            and lock["requires"] is True, "unexpected upstream lock profile")

    expected_package, expected_lock = copy.deepcopy(package), copy.deepcopy(lock)
    expected_package["name"] = profile.compatibility_name
    expected_lock["name"] = profile.compatibility_name
    expected_lock["packages"][""]["name"] = profile.compatibility_name
    replacement = {}
    for name, count, expected in (("package.json", 1, expected_package),
                                  ("package-lock.json", 2, expected_lock)):
        old = json.dumps(profile.upstream_name).encode()
        new = json.dumps(profile.compatibility_name).encode()
        require(files[name].count(old) == count, f"unexpected name occurrences in {name}")
        replacement[name] = files[name].replace(old, new)
        require(parse_metadata(replacement[name]) == expected,
                f"unauthorized metadata change in {name}")
    return replacement


def write_archive(path, files, source):
    """Write a deterministic npm tarball without executing npm or package code."""
    with path.open("xb") as output:
        with gzip.GzipFile(fileobj=output, filename="", mode="wb", mtime=0) as compressed:
            with tarfile.open(fileobj=compressed, mode="w", format=tarfile.PAX_FORMAT) as archive:
                for name, data in sorted(files.items()):
                    info = tarfile.TarInfo(f"package/{name}")
                    info.size = len(data)
                    info.mode = 0o755 if (source / name).stat().st_mode & 0o111 else 0o644
                    info.mtime = info.uid = info.gid = 0
                    info.uname = info.gname = ""
                    archive.addfile(info, io.BytesIO(data))
    with tarfile.open(path, "r:gz") as archive:
        members = archive.getmembers()
        require([member.name for member in members] == [f"package/{name}" for name in sorted(files)],
                "archive file inventory changed")
        for member in members:
            require(member.isfile() and archive.extractfile(member).read() == files[member.name[8:]],
                    f"archive payload changed: {member.name}")


def sha256(data):
    return hashlib.sha256(data).hexdigest()


def adapt(source, output, *, profile_name):
    profile = select_profile(profile_name)
    require(source.is_absolute() and output.is_absolute(), "source/output must be absolute paths")
    require(not source.is_symlink(), "source must not be a symlink")
    source = source.resolve(strict=True)
    require(not output.exists() and not output.is_symlink(), "output already exists")
    resolved_output = output.parent.resolve(strict=True) / output.name
    require(not resolved_output.is_relative_to(source), "output must be outside the source")
    files = inventory(source)
    replacement = rename_metadata(files, profile)
    expected = files | replacement
    output.mkdir()
    try:
        package_root = output / profile.lib_path
        package_root.mkdir(parents=True)
        for name, data in expected.items():
            target = package_root / name
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_bytes(data)
            target.chmod(0o755 if (source / name).stat().st_mode & 0o111 else 0o644)
        require(inventory(package_root) == expected, "copied package inventory/payload changed")
        require(inventory(source) == files, "upstream source changed during adaptation")
        write_archive(output / profile.tar_path, expected, source)
        require(inventory(source) == files, "upstream source changed during archive construction")
        receipt = {
            "upstream_name": profile.upstream_name, "compatibility_name": profile.compatibility_name, "version": profile.version,
            "metadata_fields_changed": ["package.json:name", "package-lock.json:name",
                                        "package-lock.json:packages[\"\"].name"],
            "upstream_files_sha256": {name: sha256(data) for name, data in sorted(files.items())},
            "compatibility_files_sha256": {name: sha256(data) for name, data in sorted(expected.items())},
            "tar_sha256": sha256((output / profile.tar_path).read_bytes()),
        }
        support = output / "nix-support"
        support.mkdir()
        (support / "npm-scope-compatibility.json").write_text(json.dumps(receipt, indent=2) + "\n")
        return receipt
    except BaseException:
        shutil.rmtree(output)
        raise


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--profile", choices=tuple(PROFILES), required=True)
    parser.add_argument("--source", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    try:
        adapt(args.source, args.output, profile_name=args.profile)
    except (ProfileMismatch, OSError) as error:
        parser.exit(1, f"ledger npm scope compatibility: {error}\n")


if __name__ == "__main__":
    main()
