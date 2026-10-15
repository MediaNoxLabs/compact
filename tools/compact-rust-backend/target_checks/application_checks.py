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

"""Retained application contracts and root-Let regression checks.

The CLI owns flag precedence, environment setup, and temporary-directory lifetime.
Sources and command runners are explicit inputs; each check retains its own
acceptance assertions, failure order, and optional proof commands.
"""

from collections.abc import Callable
import json
from pathlib import Path


def check_coracle_start(
    compiler: str,
    base: Path,
    *,
    proof_requested: bool,
    run: Callable[..., None],
    check_coracle_source: Callable[[str, Path], None],
) -> None:
    check_coracle_source(compiler, base)
    output = base / "coracle"
    if proof_requested:
        (output / "keys").mkdir(exist_ok=True)
        run("zkir", "compile", str(output / "zkir/start.zkir"), str(output / "keys/start.prover"), str(output / "keys/start.verifier"))
        run("cargo", "run", "--quiet", "-p", "compact-rust-proof-smoke", "--", "--coracle-start", str(output))
    print("original Coracle start recorded with exact two-wallet paths; all four proof-required exports have recording APIs")


def check_coracle_concede(
    compiler: str,
    base: Path,
    *,
    proof_requested: bool,
    run: Callable[..., None],
    check_coracle_source: Callable[[str, Path], None],
) -> None:
    check_coracle_source(compiler, base)
    output = base / "coracle"
    if proof_requested:
        (output / "keys").mkdir(exist_ok=True)
        run("zkir", "compile", str(output / "zkir/concede.zkir"), str(output / "keys/concede.prover"), str(output / "keys/concede.verifier"))
        run("cargo", "run", "--quiet", "-p", "compact-rust-proof-smoke", "--", "--coracle-concede", str(output))
    print("original Coracle concede recorded; all four proof-required exports have recording APIs")


def check_coracle_withdraw(
    compiler: str,
    base: Path,
    *,
    proof_requested: bool,
    run: Callable[..., None],
    check_coracle_source: Callable[[str, Path], None],
) -> None:
    check_coracle_source(compiler, base)
    output = base / "coracle"
    if proof_requested:
        (output / "keys").mkdir(exist_ok=True)
        run("zkir", "compile", str(output / "zkir/withdraw.zkir"), str(output / "keys/withdraw.prover"), str(output / "keys/withdraw.verifier"))
        run("cargo", "run", "--quiet", "-p", "compact-rust-proof-smoke", "--", "--coracle-withdraw", str(output))
    print("original Coracle withdraw recorded; all four proof-required exports have recording APIs")


def check_coracle_guess(
    compiler: str,
    base: Path,
    *,
    proof_requested: bool,
    run: Callable[..., None],
    check_coracle_source: Callable[[str, Path], None],
) -> None:
    check_coracle_source(compiler, base)
    output = base / "coracle"
    if proof_requested:
        (output / "keys").mkdir(exist_ok=True)
        run("zkir", "compile", str(output / "zkir/guess.zkir"), str(output / "keys/guess.prover"), str(output / "keys/guess.verifier"))
        run("cargo", "run", "--quiet", "-p", "compact-rust-proof-smoke", "--", "--coracle-guess", str(output))
    print("original Coracle guess, concede, withdraw and start recorded")


def check_coracle_root_let(
    compiler: str,
    base: Path,
    *,
    root_let_action_return_source: Path,
    test_center_welcome_source: Path,
    proof_requested: bool,
    run: Callable[..., None],
    check_coracle_source: Callable[[str, Path], None],
) -> None:
    oracle = base / "root-let-action-return"
    run(compiler, "--target", "rust", "--rust-require-recording", "--skip-zk",
        str(root_let_action_return_source), str(oracle))
    ir = json.loads((oracle / "contract/compact-rust-ir.json").read_text())
    assert ir["schema_version"] == 20
    circuit, = ir["stateful_circuits"]
    assert circuit["name"] == "step"
    assert circuit["return_value"] == {
        "kind": "expression", "value": {"kind": "parameter", "name": "echo"}}
    outer, = circuit["actions"]
    assert outer["kind"] == "let"
    assert [(row["name"], row["value"]["kind"])
            for row in outer["bindings"]] == [("before", "cell_read")]
    inner, = outer["action"]["actions"]
    assert inner["kind"] == "let"
    assert [(row["name"], row["value"]["kind"])
            for row in inner["bindings"]] == [("after", "add")]
    write, = inner["action"]["actions"]
    assert (write["kind"], write["field"], write["value"]) == (
        "cell_write", "stored", {"kind": "parameter", "name": "after"})
    caps = json.loads((oracle / "contract/rust-capabilities.json").read_text())
    assert [(row["name"], row["proof_required"], row["recorded"], row["observed_call"])
            for row in caps["circuits"]] == [("step", True, True, True)]
    assert "pub mod recorded" in (oracle / "contract/lib.rs").read_text()
    # An assert-only root Let remains in its established expression
    # form; extracting it as an action would regress the welcome
    # source's three recorded and observed exports.
    welcome = base / "welcome"
    run(compiler, "--target", "rust", "--rust-require-recording", "--skip-zk",
        str(test_center_welcome_source), str(welcome))
    welcome_caps = json.loads((welcome / "contract/rust-capabilities.json").read_text())
    assert [(row["name"], row["recorded"], row["observed_call"])
            for row in welcome_caps["circuits"]] == [
        ("add_participant", True, True),
        ("add_organizer", True, True),
        ("check_in", True, True),
    ]
    check_coracle_source(compiler, base)
    if proof_requested:
        proof = base / "root-let-action-return-proof"
        run(compiler, "--target", "rust", "--rust-require-recording",
            str(root_let_action_return_source), str(proof))
        run("cargo", "run", "--quiet", "-p", "compact-rust-proof-smoke", "--",
            "--root-let-action-return", str(proof))
    print("root Let actions retained; complete Coracle compiles with guess recorded and 3 explicit recording gaps")


def check_test_center_bboard(
    compiler: str,
    base: Path,
    *,
    test_center_bboard_source: Path,
    proof_requested: bool,
    run: Callable[..., None],
) -> None:
    output = base / "test-center-bboard"
    command = [compiler, "--target", "rust", "--rust-require-recording"]
    if not proof_requested:
        command.append("--skip-zk")
    run(*command, str(test_center_bboard_source), str(output))
    ir = json.loads((output / "contract/compact-rust-ir.json").read_text())
    assert ir["schema_version"] == 20
    assert [row["name"] for row in ir["circuits"]] == ["some", "none", "public_key"]
    assert [row["name"] for row in ir["stateful_circuits"]] == ["post", "take_down"]
    taken = ir["stateful_circuits"][1]
    assert [action["kind"] for action in taken["actions"]] == ["assert", "assert", "let"]
    tail = taken["actions"][-1]
    assert [(binding["name"], binding["ty"]["kind"], binding["value"]["kind"])
            for binding in tail["bindings"]] == [
        ("former_msg", "opaque_string", "struct_field")]
    assert taken["return_value"] == {
        "kind": "expression", "value": {"kind": "parameter", "name": "former_msg"}}
    metadata = json.loads((output / "compiler/contract-info.json").read_text())
    assert [(row["name"], row["pure"], row["proof"])
            for row in metadata["circuits"]] == [
        ("post", False, True), ("take_down", False, True),
        ("public_key", True, False)]
    capabilities = json.loads((output / "contract/rust-capabilities.json").read_text())
    assert [(row["name"], row["proof_required"], row["recorded"], row["observed_call"])
            for row in capabilities["circuits"]] == [
        ("post", True, True, True), ("take_down", True, True, True)]
    assert all("recording_unavailable" not in row for row in capabilities["circuits"])
    if proof_requested:
        run("cargo", "run", "--quiet", "-p", "compact-rust-proof-smoke", "--",
            "--bboard", str(output))
    print("complete test-center bboard recorded/observed calls accepted")
