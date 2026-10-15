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

"""Prepare and validate the default suite's six adjacent collection artifacts.

The CLI chooses sources and schedules this phase. Proof execution and the later
constructor-List artifact remain at their original positions in that recipe.
"""

from collections.abc import Callable
from dataclasses import dataclass
from pathlib import Path

from . import proof_artifacts


@dataclass(frozen=True)
class CollectionProofs:
    set: Path
    set_oracle: Path
    map: Path
    constructor_map: Path
    list: Path
    list_shapes: Path


def prepare(
    compiler: str,
    base: Path,
    *,
    set_source: Path,
    set_oracle_source: Path,
    map_source: Path,
    constructor_map_source: Path,
    list_source: Path,
    list_shapes_source: Path,
    run: Callable[..., None],
    check_manifest: Callable[[Path], None],
    check_observed_map_call_consumer: Callable[[Path, Path], None],
) -> CollectionProofs:
    set_proof = base / "set-proof"
    run(compiler, "--target", "rust", str(set_source), str(set_proof))
    check_manifest(set_proof)
    for circuit in (
        "add", "contains", "remove", "seen_size", "seen_is_empty",
        "add_field", "contains_field", "reset_fields", "choose",
    ):
        proof_artifacts.require_circuit_files(set_proof, circuit)
    set_oracle_proof = base / "set-oracle-proof"
    run(compiler, "--target", "rust", str(set_oracle_source), str(set_oracle_proof))
    check_manifest(set_oracle_proof)
    proof_artifacts.require_circuit_files(set_oracle_proof, 'check')
    map_proof = base / "map-proof"
    run(compiler, "--target", "rust", str(map_source), str(map_proof))
    check_manifest(map_proof)
    check_observed_map_call_consumer(map_proof, base)
    for circuit in (
        "put", "put_pair", "put_default", "has", "get", "remove_key",
        "table_size", "table_is_empty", "reset_table",
    ):
        proof_artifacts.require_circuit_files(map_proof, circuit)
    constructor_map_proof = base / "constructor-map-proof"
    run(compiler, "--target", "rust", str(constructor_map_source), str(constructor_map_proof))
    check_manifest(constructor_map_proof)
    for circuit in ("table_size", "history_size", "get_true", "get_false_history"):
        proof_artifacts.require_circuit_files(constructor_map_proof, circuit)
    list_proof = base / "list-proof"
    run(compiler, "--target", "rust", str(list_source), str(list_proof))
    check_manifest(list_proof)
    for circuit in ("item_count", "items_empty", "first_item", "prepend", "drop_first", "clear_items"):
        proof_artifacts.require_circuit_files(list_proof, circuit)
    list_shapes_proof = base / "list-shapes-proof"
    run(compiler, "--target", "rust", str(list_shapes_source), str(list_shapes_proof))
    check_manifest(list_shapes_proof)
    for circuit in ("push_flag", "push_count", "push_tag", "push_choice", "push_packet", "first_packet", "first_choice"):
        proof_artifacts.require_circuit_files(list_shapes_proof, circuit)
    return CollectionProofs(
        set=set_proof,
        set_oracle=set_oracle_proof,
        map=map_proof,
        constructor_map=constructor_map_proof,
        list=list_proof,
        list_shapes=list_shapes_proof,
    )
