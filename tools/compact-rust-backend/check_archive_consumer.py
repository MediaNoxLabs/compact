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

"""Build two generated contracts together using only the runtime archives."""

import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import tarfile
import tempfile
import tomllib


ROOT = Path(__file__).resolve().parents[2]
CONTRACTS = (
    ("counter", ROOT / "examples/rust_backend/counter.compact"),
    ("cell-boolean", ROOT / "examples/rust_backend/cell_boolean.compact"),
)
PACKAGE_NAMES = ("midnight-compact-runtime-macros", "midnight-compact-runtime")


def run(arguments: list[str], *, cwd: Path = ROOT, env: dict[str, str] | None = None) -> str:
    result = subprocess.run(arguments, cwd=cwd, env=env, capture_output=True, text=True)
    if result.returncode:
        raise RuntimeError(
            f"{' '.join(arguments)} failed with {result.returncode}:\n"
            f"{result.stdout[-4000:]}\n{result.stderr[-4000:]}"
        )
    return result.stdout


def sha256(path: Path) -> str:
    with path.open("rb") as content:
        return hashlib.file_digest(content, "sha256").hexdigest()


def install_archive(vendor: Path, entry: dict, macro_version: str) -> Path:
    name = entry["name"]
    version = entry["version"]
    if name not in PACKAGE_NAMES or not isinstance(version, str):
        raise RuntimeError("release manifest contains an unexpected package")
    archive = ROOT / entry["archive"]
    if not archive.is_file() or sha256(archive) != entry["sha256"]:
        raise RuntimeError(f"{archive}: missing or different from release manifest")
    directory = vendor / f"{name}-{version}"
    if directory.exists():
        raise RuntimeError(f"{directory}: already present in upstream vendor source")
    with tarfile.open(archive, "r:gz") as package:
        members = package.getmembers()
        if not members:
            raise RuntimeError(f"{archive}: empty package")
        for member in members:
            destination = (vendor / member.name).resolve()
            if not destination.is_relative_to(directory.resolve()) or not member.isfile():
                raise RuntimeError(f"{archive}: unsafe member {member.name}")
        package.extractall(vendor, filter="data")
    manifest = tomllib.loads((directory / "Cargo.toml").read_text())
    if manifest["package"]["name"] != name or manifest["package"]["version"] != version:
        raise RuntimeError(f"{archive}: package identity mismatch")
    if name == "midnight-compact-runtime":
        dependency = manifest["dependencies"]["midnight-compact-runtime-macros"]
        if dependency.get("version") != f"={macro_version}" or "path" in dependency:
            raise RuntimeError(f"{archive}: macro dependency is not version-only")
    files = {
        path.relative_to(directory).as_posix(): sha256(path)
        for path in directory.rglob("*") if path.is_file()
    }
    (directory / ".cargo-checksum.json").write_text(
        json.dumps({"files": files, "package": entry["sha256"]}, sort_keys=True)
    )
    return directory


def check_graph(metadata: dict, consumer: Path, vendor: Path, entries: list[dict]) -> None:
    packages = metadata["packages"]
    runtime_id = None
    for name in PACKAGE_NAMES:
        found = [package for package in packages if package["name"] == name]
        if len(found) != 1:
            raise RuntimeError(f"expected one {name} in consumer graph, got {len(found)}")
        package = found[0]
        if not package["source"].startswith("registry+"):
            raise RuntimeError(f"{name} was not resolved as a registry package")
        path = Path(package["manifest_path"]).resolve()
        if not path.is_relative_to(vendor.resolve()):
            raise RuntimeError(f"{name} did not come from the archive vendor source")
        if name == "midnight-compact-runtime":
            runtime_id = package["id"]
    if runtime_id is None:
        raise RuntimeError("runtime package missing from resolved graph")
    nodes = {node["id"]: node for node in metadata["resolve"]["nodes"]}
    for name, _ in CONTRACTS:
        package_name = f"compact-contract-{name}"
        found = [package for package in packages if package["name"] == package_name]
        if len(found) != 1:
            raise RuntimeError(f"expected one generated {package_name}")
        direct = {dependency["pkg"] for dependency in nodes[found[0]["id"]]["deps"]}
        if runtime_id not in direct:
            raise RuntimeError(f"{package_name} did not resolve the shared archived runtime")
    lock = (consumer / "Cargo.lock").read_text()
    for entry in entries:
        name = entry["name"]
        version = entry["version"]
        if f'name = "{name}"\nversion = "{version}"\nsource = "registry+' not in lock:
            raise RuntimeError(f"{name} is not a versioned registry entry in Cargo.lock")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--manifest", required=True, type=Path,
                        help="release manifest produced by check_release_packages.py")
    parser.add_argument("--compiler", default="compactc", help="packaged compactc executable")
    args = parser.parse_args()
    release = json.loads(args.manifest.read_text())
    if release.get("schema") != 1 or release.get("source", {}).get("commit") != run(
        ["git", "rev-parse", "HEAD"]
    ).strip():
        raise RuntimeError("release manifest does not describe this source commit")
    entries = release.get("packages")
    if not isinstance(entries, list) or [entry.get("name") for entry in entries] != list(PACKAGE_NAMES):
        raise RuntimeError("release manifest must contain macro then runtime archives")
    if release["source"].get("tree") != run(["git", "rev-parse", "HEAD^{tree}"]).strip():
        raise RuntimeError("release manifest does not describe this source tree")
    macro_version = entries[0]["version"]
    runtime_version = entries[1]["version"]
    for entry in entries:
        archive = ROOT / entry["archive"]
        if not archive.is_file() or sha256(archive) != entry["sha256"]:
            raise RuntimeError(f"{archive}: missing or different from release manifest")

    (ROOT / "target").mkdir(exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="rust-archive-consumer-", dir=ROOT / "target") as scratch:
        temporary = Path(scratch)
        vendor = temporary / "vendor"
        run(["cargo", "vendor", "--offline", "--locked", "--versioned-dirs",
             "--manifest-path", str(ROOT / "runtime-rs/Cargo.toml"), str(vendor)])
        for entry in entries:
            install_archive(vendor, entry, macro_version)

        contracts = {}
        before = 'midnight-compact-runtime = { path = "runtime-rs", package = "midnight-compact-runtime" }'
        after = (
            'midnight-compact-runtime = '
            f'{{ version = "={runtime_version}", package = "midnight-compact-runtime" }}'
        )
        for name, source in CONTRACTS:
            artifact = temporary / name
            run([args.compiler, "--target", "rust", "--skip-zk", str(source), str(artifact)])
            contract = artifact / "contract"
            manifest_path = contract / "Cargo.toml"
            generated = manifest_path.read_text()
            if generated.count(before) != 1:
                raise RuntimeError(f"generated {name} manifest has an unexpected runtime dependency")
            manifest_path.write_text(generated.replace(before, after))
            shutil.rmtree(contract / "runtime-rs")
            shutil.rmtree(contract / "runtime-rs-macros")
            contracts[name] = contract

        consumer = temporary / "consumer"
        (consumer / "tests").mkdir(parents=True)
        (consumer / ".cargo").mkdir()
        (consumer / "Cargo.toml").write_text(
            '[package]\nname = "compact-rust-archive-consumer"\nversion = "0.1.0"\n'
            'edition = "2024"\n\n[workspace]\n\n[dependencies]\n'
            + "".join(
                f'compact-contract-{name} = {{ path = "{contract}", features = ["ledger-transaction"] }}\n'
                for name, contract in contracts.items()
            )
        )
        (consumer / ".cargo/config.toml").write_text(
            '[source.crates-io]\nreplace-with = "archive-vendor"\n'
            f'[source.archive-vendor]\ndirectory = "{vendor}"\n'
        )
        (consumer / "tests/counter.rs").write_text(
            'use compact_contract_counter as counter;\n'
            'use compact_contract_cell_boolean as cell;\n'
            'use compact_contract_counter::runtime::context::ConstructorContext;\n'
            'use compact_contract_counter::runtime::ledger::{ContractAddress, StateValue, read_cell};\n'
            '#[test]\nfn archived_runtime_executes_generated_counter() {\n'
            '    let state = counter::ledger_contract::initial_state(ConstructorContext::new(())).unwrap();\n'
            '    let context = state.into_circuit_context(ContractAddress::default());\n'
            '    let call = counter::ledger_contract::Contract::default().increment(context).unwrap();\n'
            '    let read = counter::ledger_contract::Contract::default().read_round(call.context).unwrap();\n'
            '    assert_eq!(read.result.value(), 1);\n}\n'
            '#[test]\nfn archived_runtime_executes_generated_cell() {\n'
            '    let address: counter::runtime::ledger::ContractAddress =\n'
            '        cell::runtime::ledger::ContractAddress::default();\n'
            '    let state = cell::ledger_contract::initial_state(ConstructorContext::new(())).unwrap();\n'
            '    let context = state.into_circuit_context(address);\n'
            '    let call = cell::ledger_contract::Contract::default().set_flag(context).unwrap();\n'
            '    let StateValue::Array(fields) = call.context.query.state.get_ref() else {\n'
            '        panic!("expected generated Cell field array");\n'
            '    };\n'
            '    assert!(read_cell::<bool, _>(&fields.get(0).unwrap()).unwrap());\n}\n'
        )
        environment = os.environ.copy()
        environment.setdefault("CARGO_TARGET_DIR", str(ROOT / "target/compactc-consumer"))
        environment.setdefault("CARGO_INCREMENTAL", "0")
        environment.setdefault("CARGO_BUILD_JOBS", "2")
        run(["cargo", "test", "--offline", "--quiet"], cwd=consumer, env=environment)
        metadata = json.loads(run(["cargo", "metadata", "--offline", "--format-version", "1"],
                                  cwd=consumer, env=environment))
        check_graph(metadata, consumer, vendor, entries)
    print("archive-only Counter + Cell consumer passed with one shared runtime; public registry publication remains unverified")


if __name__ == "__main__":
    main()
