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

"""Audit the standalone backend lock against the qualified workspace lock.

This checks resolved package identities, checksums, and retained dependency
edges. Cargo may prune feature-inactive edges when selecting only the backend;
those removals are reported, not described as identical feature resolution.
A separate locked standalone build remains required.
"""

import argparse
import json
from pathlib import Path
import tomllib


def packages(path):
    result = {}
    for package in tomllib.loads(Path(path).read_text())["package"]:
        key = (package["name"], package["version"], package.get("source", ""))
        if key in result:
            raise RuntimeError(f"duplicate locked package: {key}")
        result[key] = package
    return result


def resolve(reference, graph):
    parts = reference.split(" ", 2)
    candidates = [key for key in graph if key[0] == parts[0]
                  and (len(parts) < 2 or key[1] == parts[1])
                  and (len(parts) < 3 or key[2] == parts[2].removeprefix("(").removesuffix(")"))]
    if len(candidates) != 1:
        raise RuntimeError(f"missing or ambiguous locked edge: {reference}")
    return candidates[0]


def compare_locks(workspace, standalone):
    approved, selected = packages(workspace), packages(standalone)
    roots = [key for key in selected if key[0] == "compact-rust-backend" and not key[2]]
    if len(roots) != 1:
        raise RuntimeError("standalone lock must contain one local backend package")
    removed = []
    edges = {}
    for key, package in selected.items():
        if key not in approved:
            raise RuntimeError(f"unapproved locked package identity: {key}")
        if package.get("checksum") != approved[key].get("checksum"):
            raise RuntimeError(f"locked package checksum differs: {key}")
    for key, package in selected.items():
        actual = {resolve(dep, selected) for dep in package.get("dependencies", [])}
        expected = {resolve(dep, approved) for dep in approved[key].get("dependencies", [])}
        if actual - expected:
            raise RuntimeError(f"unapproved dependency edge: {key}: {sorted(actual - expected)}")
        if key == roots[0] and actual != expected:
            raise RuntimeError("backend direct dependency edges differ")
        if expected - actual:
            removed.append({"package": key, "removed_edges": sorted(expected - actual)})
        edges[key] = actual
    seen, pending = set(), list(roots)
    while pending:
        key = pending.pop()
        if key not in seen:
            seen.add(key)
            pending.extend(edges[key])
    if seen != selected.keys():
        raise RuntimeError("standalone lock contains packages unreachable from backend")
    return {
        "standalone_packages": len(selected),
        "registry_packages": sum(key[2].startswith("registry+") for key in selected),
        "git_packages": sum(key[2].startswith("git+") for key in selected),
        "local_packages": sum(not key[2] for key in selected),
        "retained_edges": sum(len(value) for value in edges.values()),
        "removed_feature_edges": removed,
        "assurance": "All retained identities/checksums/edges match workspace; inactive edge removals reported. Locked compilation is separate.",
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--workspace", type=Path, default=Path(__file__).resolve().parents[2] / "Cargo.lock")
    parser.add_argument("--standalone", type=Path, default=Path(__file__).with_name("Cargo.lock"))
    args = parser.parse_args()
    print(json.dumps(compare_locks(args.workspace, args.standalone), indent=2))


if __name__ == "__main__":
    main()
