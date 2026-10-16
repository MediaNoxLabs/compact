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

"""Native compiler target selection, artifacts, and compatibility acceptance.

The CLI owns scheduling and the temporary directory. Only the two output paths
needed by the optional consumer phase escape this acceptance boundary.
"""

from collections.abc import Callable
from dataclasses import dataclass
import json
from pathlib import Path
from subprocess import CompletedProcess


@dataclass(frozen=True)
class NativeTargetOutputs:
    legacy_rust: Path
    pure: Path


def check(
    compiler: str,
    base: Path,
    *,
    root: Path,
    counter_source: Path,
    pure_source: Path,
    witness_cell_source: Path,
    constructor_map_source: Path,
    alias_source: Path,
    pm_19252_source: Path,
    native_key_value_source: Path,
    run: Callable[..., None],
    capture: Callable[..., CompletedProcess[str]],
    check_manifest: Callable[[Path], None],
) -> NativeTargetOutputs:
    ts, rust, both, pure = (base / name for name in ("ts", "rust", "both", "pure"))
    run(compiler, "--skip-zk", str(counter_source), str(ts))
    assert (ts / "contract/index.js").is_file()
    assert not (ts / "contract/lib.rs").exists()
    run(compiler, "--target", "rust", "--skip-zk", str(counter_source), str(rust))
    assert (rust / "contract/lib.rs").is_file()
    assert (rust / "contract/Cargo.toml").is_file()
    assert not (rust / "contract/index.js").exists()
    capabilities = json.loads((rust / "contract/rust-capabilities.json").read_text())
    assert capabilities["schema_version"] == 3
    assert [(c["name"], c["recorded"], c["observed_call"]) for c in capabilities["circuits"]] == [
        ("increment", True, True), ("read_round", True, True)
    ]
    assert all(c["proof_required"] is True and c["recording_status"] == "available"
               for c in capabilities["circuits"])
    rust_ir = json.loads((rust / "contract/compact-rust-ir.json").read_text())
    assert rust_ir["schema_version"] == 20
    _check_native_key_witnesses(
        compiler, base,
        pm_19252_source=pm_19252_source,
        native_key_value_source=native_key_value_source,
        run=run,
    )
    round_field = next(field for field in rust_ir["ledger_fields"] if field["id"] == "round")
    assert round_field["source"]["file"] == counter_source.name
    assert (round_field["source"]["line"], round_field["source"]["column"]) == (18, 1)
    assert rust_ir["stateful_circuits"]
    assert (rust_ir["stateful_circuits"][0]["source"]["line"], rust_ir["stateful_circuits"][0]["source"]["column"]) == (20, 1)
    for circuit in rust_ir["stateful_circuits"]:
        assert circuit["source"]["file"] == counter_source.name
        assert circuit["source"]["line"] > 0
    for name, source, key, first_line in (
        ("pure", pure_source, "circuits", 18),
        ("witness", witness_cell_source, "witnesses", 20),
        ("constructor", constructor_map_source, "constructor", 21),
        ("alias", alias_source, "type_aliases", 27),
    ):
        output = base / f"source-{name}"
        run(compiler, "--target", "rust", "--skip-zk", str(source), str(output))
        emitted = json.loads((output / "contract/compact-rust-ir.json").read_text())
        assert emitted["schema_version"] == 20
        owners = emitted[key]
        if isinstance(owners, dict):
            owners = [owners]
        assert owners, f"{source.name}: missing {key}"
        assert (owners[0]["source"]["line"], owners[0]["source"]["column"]) == (first_line, 1)
        for owner in owners:
            assert owner["source"]["file"] == source.name
            assert owner["source"]["line"] > 0
            assert owner["source"]["column"] > 0
    check_manifest(rust)
    protected = base / "protected"
    protected.mkdir()
    marker = protected / "keep.txt"
    marker.write_text("compiler output cleanup must not follow symlinks\n")
    nested_link = rust / "contract/linked"
    top_level_link = rust / "keys"
    nested_link.symlink_to(protected, target_is_directory=True)
    top_level_link.symlink_to(protected, target_is_directory=True)
    run(compiler, "--target", "rust", "--skip-zk", str(counter_source), str(rust))
    assert marker.is_file(), "compiler output cleanup followed a directory symlink"
    assert not nested_link.is_symlink()
    assert not top_level_link.is_symlink()
    check_manifest(rust)
    run(
        compiler, "--target=ts", "--target=rust", "--skip-zk",
        str(counter_source), str(both),
    )
    assert (both / "contract/index.js").is_file()
    assert (both / "contract/lib.rs").is_file()
    check_manifest(both)
    legacy_rust = base / "legacy-rust"
    run(compiler, "--rust", "--skip-ts", "--skip-zk", str(counter_source), str(legacy_rust))
    assert (legacy_rust / "contract/lib.rs").read_bytes() == (rust / "contract/lib.rs").read_bytes()
    assert (legacy_rust / "contract/Cargo.toml").read_bytes() == (rust / "contract/Cargo.toml").read_bytes()
    assert (legacy_rust / "contract/rust-capabilities.json").read_bytes() == (
        rust / "contract/rust-capabilities.json"
    ).read_bytes()
    assert not (legacy_rust / "contract/index.js").exists()
    check_manifest(legacy_rust)
    legacy_both = base / "legacy-both"
    run(compiler, "--rust", "--skip-zk", str(counter_source), str(legacy_both))
    assert (legacy_both / "contract/index.js").read_bytes() == (both / "contract/index.js").read_bytes()
    assert (legacy_both / "contract/lib.rs").read_bytes() == (both / "contract/lib.rs").read_bytes()
    assert (legacy_both / "contract/Cargo.toml").read_bytes() == (both / "contract/Cargo.toml").read_bytes()
    check_manifest(legacy_both)
    for label, flags, message in (
        ("mixed-rust", ("--target", "rust", "--rust"), "cannot be combined"),
        ("mixed-skip-ts", ("--skip-ts", "--target=ts"), "cannot be combined"),
        ("skip-ts-alone", ("--skip-ts",), "requires --rust"),
    ):
        rejected_output = base / label
        rejected = capture(
            [compiler, *flags, "--skip-zk", str(counter_source), str(rejected_output)],
            cwd=root, capture_output=True, text=True,
        )
        assert rejected.returncode != 0, label
        assert message in rejected.stderr, (label, rejected.stderr)
        assert not rejected_output.exists(), f"{label}: invalid target selection created output"
    return NativeTargetOutputs(legacy_rust=legacy_rust, pure=pure)


def _check_native_key_witnesses(
    compiler: str,
    base: Path,
    *,
    pm_19252_source: Path,
    native_key_value_source: Path,
    run: Callable[..., None],
) -> None:
    native_output = base / "native-own-public-key"
    run(compiler, "--target", "rust", "--skip-zk", str(pm_19252_source), str(native_output))
    native_contract = native_output / "contract"
    native_ir = json.loads((native_contract / "compact-rust-ir.json").read_text())
    native_circuit = next(c for c in native_ir["stateful_circuits"] if c["name"] == "test1")
    assert native_ir["witnesses"] == []
    assert native_circuit["actions"] == [
        {"kind": "native_witness_call", "builtin": "own_public_key"}
    ]
    native_capability = json.loads((native_contract / "rust-capabilities.json").read_text())
    assert len(native_capability["circuits"]) == 1
    native_export = native_capability["circuits"][0]
    assert native_export["name"] == "test1"
    assert native_export["proof_required"] is False
    assert native_export["recorded"] is False
    assert native_export["observed_call"] is False
    assert native_export["recording_status"] == "not_applicable"
    assert native_export["recording_unavailable"]["ir_node"] == "StateAction::NativeWitnessCall"
    native_source = (native_contract / "lib.rs").read_text()
    assert "context.own_coin_public_key()?" in native_source
    assert "pub fn test1<Private>(" in native_source
    assert "pub fn test1<Private, W:" not in native_source
    native_value_output = base / "native-own-public-key-value"
    run(compiler, "--target", "rust", "--skip-zk", str(native_key_value_source),
        str(native_value_output))
    native_value_contract = native_value_output / "contract"
    native_value_ir = json.loads((native_value_contract / "compact-rust-ir.json").read_text())
    assert native_value_ir["schema_version"] == 20
    assert native_value_ir["witnesses"] == []
    assert [(c["name"], c["return_value"]["value"]["kind"])
            for c in native_value_ir["stateful_circuits"]] == [
        ("key", "native_witness_call"), ("key_bytes", "struct_field")
    ]
    assert native_value_ir["stateful_circuits"][1]["return_value"]["value"]["value"] == {
        "kind": "native_witness_call", "builtin": "own_public_key"
    }
    native_value_capability = json.loads((native_value_contract / "rust-capabilities.json").read_text())
    assert [(c["name"], c["proof_required"], c["recording_status"])
            for c in native_value_capability["circuits"]] == [
        ("key", False, "not_applicable"), ("key_bytes", False, "not_applicable")
    ]
    native_value_source = (native_value_contract / "lib.rs").read_text()
    assert native_value_source.count("context.own_coin_public_key()?") == 2
    assert "pub fn key<Private, W:" not in native_value_source
    assert "pub fn key_bytes<Private, W:" not in native_value_source
