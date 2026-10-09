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

"""Separate-consumer acceptance for generated contracts sharing one runtime.

The caller selects the compiler, source order, repository and process boundaries.
The Rust assets are copied unchanged; each positive or negative Cargo check keeps
its existing diagnostic contract and runs in the same temporary consumer crate.
"""

from collections.abc import Callable, Mapping, Sequence
import json
from pathlib import Path
import subprocess
import tomllib


def check(
    compiler: str,
    base: Path,
    root: Path,
    sources: Sequence[tuple[str, Path]],
    *,
    assets: Path,
    run: Callable[..., None],
    execute: Callable[..., subprocess.CompletedProcess[str]],
    check_manifest: Callable[..., None],
    environ: Mapping[str, str],
) -> None:
    contracts = []
    for name, source in sources:
        output = base / f"shared-{name}"
        run(
            compiler, "--target", "rust", "--rust-runtime-root", str(root),
            "--skip-zk", str(source), str(output),
        )
        check_manifest(output, require_zkir=name != "field_add")
        contract = output / "contract"
        manifest = tomllib.loads((contract / "Cargo.toml").read_text())
        assert Path(manifest["dependencies"]["midnight-compact-runtime"]["path"]) == root / "runtime-rs"
        assert not (contract / "runtime-rs").exists()
        contracts.append((contract, manifest["package"]["name"]))

    consumer = base / "shared-consumer"
    consumer.mkdir()
    (consumer / "tests").mkdir()
    (consumer / "Cargo.toml").write_text(
        "[package]\nname = \"compactc-shared-runtime-smoke\"\n"
        "version = \"0.1.0\"\nedition = \"2024\"\n\n[dependencies]\n"
        + "".join(
            f"{name} = {{ path = {json.dumps(str(contract))} }}\n"
            for contract, name in contracts
        )
        + f'midnight-compact-runtime = {{ path = {json.dumps(str(root / "runtime-rs"))} }}\n'
    )
    (consumer / "tests/both.rs").write_text(
        (assets / 'tests/both.rs').read_text()
    )
    environment = dict(environ)
    environment.setdefault("CARGO_TARGET_DIR", str(root / "target/compactc-consumer"))
    execute(["cargo", "test", "--quiet"], cwd=consumer, env=environment, check=True)

    # Check the generated slot type from a separate consumer crate.
    (consumer / "examples").mkdir()
    (consumer / "examples/wrong_cell_value.rs").write_text(
        (assets / 'examples/wrong_cell_value.rs').read_text()
    )
    rejected = execute(
        ["cargo", "check", "--quiet", "--example", "wrong_cell_value"],
        cwd=consumer, env=environment, capture_output=True, text=True,
    )
    assert rejected.returncode != 0, "wrong Cell value type unexpectedly compiled"
    assert "expected `bool`, found `u64`" in rejected.stderr, rejected.stderr

    (consumer / "examples/wrong_set_element.rs").write_text(
        (assets / 'examples/wrong_set_element.rs').read_text()
    )
    rejected = execute(
        ["cargo", "check", "--quiet", "--example", "wrong_set_element"],
        cwd=consumer, env=environment, capture_output=True, text=True,
    )
    assert rejected.returncode != 0, "wrong Set element type unexpectedly compiled"
    assert "error[E0308]: mismatched types" in rejected.stderr, rejected.stderr
    assert "found `bool`" in rejected.stderr, rejected.stderr

    (consumer / "examples/wrong_vector_key.rs").write_text(
        (assets / 'examples/wrong_vector_key.rs').read_text()
    )
    rejected = execute(
        ["cargo", "check", "--quiet", "--example", "wrong_vector_key"],
        cwd=consumer, env=environment, capture_output=True, text=True,
    )
    assert rejected.returncode != 0, "raw-byte vector key unexpectedly compiled"
    assert "error[E0308]: mismatched types" in rejected.stderr, rejected.stderr
    assert "FixedVector" in rejected.stderr, rejected.stderr

    (consumer / "examples/nested_map_scalar_lookup.rs").write_text(
        (assets / 'examples/nested_map_scalar_lookup.rs').read_text()
    )
    rejected = execute(
        ["cargo", "check", "--quiet", "--example", "nested_map_scalar_lookup"],
        cwd=consumer, env=environment, capture_output=True, text=True,
    )
    assert rejected.returncode != 0, "nested Map scalar lookup unexpectedly compiled"
    assert "lookup" in rejected.stderr and "MapNode" in rejected.stderr, rejected.stderr


    (consumer / "examples/wrong_list_element.rs").write_text(
        (assets / 'examples/wrong_list_element.rs').read_text()
    )
    rejected = execute(
        ["cargo", "check", "--quiet", "--example", "wrong_list_element"],
        cwd=consumer, env=environment, capture_output=True, text=True,
    )
    assert rejected.returncode != 0, "wrong List element type unexpectedly compiled"
    assert "error[E0308]: mismatched types" in rejected.stderr, rejected.stderr
    assert "found `bool`" in rejected.stderr, rejected.stderr

    (consumer / "examples/wrong_map_key.rs").write_text(
        (assets / 'examples/wrong_map_key.rs').read_text()
    )
    rejected = execute(
        ["cargo", "check", "--quiet", "--example", "wrong_map_key"],
        cwd=consumer, env=environment, capture_output=True, text=True,
    )
    assert rejected.returncode != 0, "wrong Map key type unexpectedly compiled"
    assert "error[E0308]: mismatched types" in rejected.stderr, rejected.stderr
    assert "expected `bool`" in rejected.stderr, rejected.stderr

    (consumer / "examples/wrong_map_value.rs").write_text(
        (assets / 'examples/wrong_map_value.rs').read_text()
    )
    rejected = execute(
        ["cargo", "check", "--quiet", "--example", "wrong_map_value"],
        cwd=consumer, env=environment, capture_output=True, text=True,
    )
    assert rejected.returncode != 0, "wrong Map value type unexpectedly compiled"
    assert "error[E0308]: mismatched types" in rejected.stderr, rejected.stderr
    assert "found `bool`" in rejected.stderr, rejected.stderr

    (consumer / "examples/wrong_merkle_leaf.rs").write_text(
        (assets / 'examples/wrong_merkle_leaf.rs').read_text()
    )
    rejected = execute(
        ["cargo", "check", "--quiet", "--example", "wrong_merkle_leaf"],
        cwd=consumer, env=environment, capture_output=True, text=True,
    )
    assert rejected.returncode != 0, "wrong Merkle leaf type unexpectedly compiled"
    assert "error[E0308]: mismatched types" in rejected.stderr, rejected.stderr
    assert "expected `BoundedUint<255>`" in rejected.stderr, rejected.stderr

    (consumer / "examples/recorded_merkle_index.rs").write_text(
        (assets / 'examples/recorded_merkle_index.rs').read_text()
    )
    supported = execute(
        ["cargo", "check", "--quiet", "--example", "recorded_merkle_index"],
        cwd=consumer, env=environment, capture_output=True, text=True,
    )
    assert supported.returncode == 0, supported.stderr

    (consumer / "examples/recorded_historic_index.rs").write_text(
        (assets / 'examples/recorded_historic_index.rs').read_text()
    )
    supported = execute(
        ["cargo", "check", "--quiet", "--example", "recorded_historic_index"],
        cwd=consumer, env=environment, capture_output=True, text=True,
    )
    assert supported.returncode == 0, supported.stderr

    (consumer / "examples/wrong_recorded_historic_leaf.rs").write_text(
        (assets / 'examples/wrong_recorded_historic_leaf.rs').read_text()
    )
    rejected = execute(
        ["cargo", "check", "--quiet", "--example", "wrong_recorded_historic_leaf"],
        cwd=consumer, env=environment, capture_output=True, text=True,
    )
    assert rejected.returncode != 0, "wrong recorded historic leaf unexpectedly compiled"
    assert "error[E0308]: mismatched types" in rejected.stderr, rejected.stderr
    assert "expected `BoundedUint<255>`" in rejected.stderr, rejected.stderr

    (consumer / "examples/recorded_history_reset.rs").write_text(
        (assets / 'examples/recorded_history_reset.rs').read_text()
    )
    run("cargo", "check", "--quiet", "--example", "recorded_history_reset", cwd=consumer,
        env=environment)

    (consumer / "examples/recorded_historic_tree_reset.rs").write_text(
        (assets / 'examples/recorded_historic_tree_reset.rs').read_text()
    )
    run("cargo", "check", "--quiet", "--example", "recorded_historic_tree_reset", cwd=consumer,
        env=environment)

    (consumer / "examples/recorded_historic_known.rs").write_text(
        (assets / 'examples/recorded_historic_known.rs').read_text()
    )
    run("cargo", "check", "--quiet", "--example", "recorded_historic_known", cwd=consumer,
        env=environment)

    (consumer / "examples/wrong_recorded_merkle_leaf.rs").write_text(
        (assets / 'examples/wrong_recorded_merkle_leaf.rs').read_text()
    )
    rejected = execute(
        ["cargo", "check", "--quiet", "--example", "wrong_recorded_merkle_leaf"],
        cwd=consumer, env=environment, capture_output=True, text=True,
    )
    assert rejected.returncode != 0, "wrong recorded Merkle leaf unexpectedly compiled"
    assert "error[E0308]: mismatched types" in rejected.stderr, rejected.stderr
    assert "expected `BoundedUint<255>`" in rejected.stderr, rejected.stderr

    (consumer / "examples/plain_merkle_reset_history.rs").write_text(
        (assets / 'examples/plain_merkle_reset_history.rs').read_text()
    )
    rejected = execute(
        ["cargo", "check", "--quiet", "--example", "plain_merkle_reset_history"],
        cwd=consumer, env=environment, capture_output=True, text=True,
    )
    assert rejected.returncode != 0, "plain Merkle history reset unexpectedly compiled"
    assert "error[E0599]" in rejected.stderr, rejected.stderr
    assert "MerkleSlot<T, DEPTH, true>" in rejected.stderr, rejected.stderr
