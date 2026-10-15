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

"""Recorded value, assertion, and lexical-return target checks.

The CLI owns flag precedence, environment setup, and temporary-directory lifetime.
Sources and command runners are explicit inputs; each check retains its own
acceptance assertions, failure order, and optional proof commands.
"""

from collections.abc import Callable
import json
from pathlib import Path
from subprocess import CompletedProcess


def check_stateful_assert(
    compiler: str,
    base: Path,
    *,
    root: Path,
    proof_requested: bool,
    run: Callable[..., None],
) -> None:
    source = root / "examples/rust_backend/stateful_assert_oracle.compact"
    output = base / "stateful-assert"
    run(compiler, "--target", "rust", "--skip-zk", str(source), str(output))
    report = json.loads((output / "contract/rust-capabilities.json").read_text())
    assert {row["name"] for row in report["circuits"]} == {"checked", "unit_result"}
    for row in report["circuits"]:
        assert row["proof_required"] and row["recorded"] and row["observed_call"]
    run(compiler, "--target", "rust", "--rust-require-recording", "--skip-zk", str(source), str(base / "strict-assert"))
    if proof_requested:
        proof = base / "stateful-assert-proof"
        run(compiler, "--target", "rust", "--rust-require-recording", str(source), str(proof))
        run("cargo", "run", "--quiet", "-p", "compact-rust-proof-smoke", "--", "--stateful-assert", str(proof))
    original = base / "original-dao"
    run(compiler, "--target", "rust", "--skip-zk", str(root / "test-center/test-contracts/micro-dao.compact"), str(original))
    info = json.loads((original / "compiler/contract-info.json").read_text())
    report = json.loads((original / "contract/rust-capabilities.json").read_text())
    assert len(info["circuits"]) == 11
    assert sum(c["proof"] for c in info["circuits"]) == 7
    assert len(report["circuits"]) == 7
    assert all(c["proof_required"] for c in report["circuits"])
    assert {c["name"] for c in report["circuits"] if c["recorded"]} == {"advance", "vote_reveal", "dao_voting_token", "set_topic", "buy_in", "cash_out", "vote_commit"}
    run("cargo", "+1.99.0", "check", "--offline", "--manifest-path", str(original / "contract/Cargo.toml"))
    print("nested stateful assertions admitted; original micro-dao native crate checks, all seven proof-required APIs recorded; advance, vote_reveal, dao_voting_token, set_topic, buy_in, cash_out and vote_commit recorded")


def check_terminal_lexical_return(
    compiler: str,
    base: Path,
    *,
    root: Path,
    proof_requested: bool,
    run: Callable[..., None],
) -> None:
    source = root / "examples/rust_backend/terminal_lexical_return_oracle.compact"
    output = base / "terminal-lexical-return"
    run(compiler, "--target", "rust", "--rust-require-recording", "--skip-zk", str(source), str(output))
    info = json.loads((output / "compiler/contract-info.json").read_text())
    report = json.loads((output / "contract/rust-capabilities.json").read_text())
    names = {"two", "three", "echo", "nested", "observed"}
    assert {c["name"] for c in info["circuits"] if c["proof"]} == names
    assert {c["name"] for c in report["circuits"]} == names
    assert all(c["proof_required"] and c["recorded"] and c["observed_call"] for c in report["circuits"])
    run("cargo", "+1.99.0", "check", "--offline", "--manifest-path", str(output / "contract/Cargo.toml"))
    if proof_requested:
        (output / "keys").mkdir(exist_ok=True)
        for name in ["two", "three", "nested", "observed"]:
            run("zkir", "compile", str(output / f"zkir/{name}.zkir"), str(output / f"keys/{name}.prover"), str(output / f"keys/{name}.verifier"))
        run("cargo", "run", "--quiet", "-p", "compact-rust-proof-smoke", "--", "--terminal-lexical-return", str(output))
    print("terminal lexical returns: five proof-required recorded APIs; four new strict Dust proof cases when requested")


def check_field_observation(
    compiler: str,
    base: Path,
    *,
    root: Path,
    proof_requested: bool,
    run: Callable[..., None],
) -> None:
    source = root / "examples/rust_backend/field_observation_oracle.compact"
    output = base / "field-observation"
    run(compiler, "--target", "rust", "--rust-require-recording",
        *([] if proof_requested else ["--skip-zk"]), str(source), str(output))
    report = json.loads((output / "contract/rust-capabilities.json").read_text())
    assert {row["name"] for row in report["circuits"]} == {
        "direct", "snapshot", "via_field_helper", "via_snapshot_helper",
        "pair", "selected", "optional"}
    assert all(row["recorded"] and row["observed_call"] and row["proof_required"]
               for row in report["circuits"])
    if proof_requested:
        run("cargo", "run", "--quiet", "-p", "compact-rust-proof-smoke", "--",
            "--field-observation", str(output))
    print("Field observations compose into typed structs and scoped helpers; empty selected path refuses preparation")


def check_stateful_struct(
    compiler: str,
    base: Path,
    *,
    root: Path,
    proof_requested: bool,
    run: Callable[..., None],
    capture: Callable[..., CompletedProcess[str]],
) -> None:
    source = root / "examples/rust_backend/stateful_struct_oracle.compact"
    output = base / "stateful-struct"
    run(compiler, "--target", "rust", "--skip-zk", str(source), str(output))
    ir = json.loads((output / "contract/compact-rust-ir.json").read_text())
    assert ir["schema_version"] == 20
    encoded = json.dumps(ir)
    assert '"struct_literal"' in encoded and '"witness_call"' in encoded
    assert '"create_zswap_output"' in encoded and '"kernel_self"' in encoded
    report = json.loads((output / "contract/rust-capabilities.json").read_text())
    assert len(report["circuits"]) == 4
    for row in report["circuits"]:
        assert row["recorded"]
        assert row["observed_call"] == row["recorded"]
        assert row["proof_required"]
    strict = capture([compiler, "--target", "rust", "--rust-require-recording", "--skip-zk", str(source), str(base / "strict-struct")], cwd=root, capture_output=True,text=True)
    assert strict.returncode == 0, strict.stderr
    if proof_requested:
        proof = base / "stateful-struct-proof"
        run(compiler,"--target","rust",str(source),str(proof))
        run("cargo","run","--quiet","-p","compact-rust-proof-smoke","--","--stateful-struct",str(proof))
    original = capture([compiler, "--target", "rust", "--skip-zk", str(root / "test-center/test-contracts/micro-dao.compact"), str(base / "original-dao")], cwd=root, capture_output=True, text=True)
    assert original.returncode == 0, original.stderr
    assert (base / "original-dao/contract/lib.rs").is_file()
    print("ordered typed stateful structs admitted; original micro-dao emits native Rust")


def check_effectful_return(
    compiler: str,
    base: Path,
    *,
    effectful_return_source: Path,
    test_center_welcome_source: Path,
    test_center_bboard_source: Path,
    election_source: Path,
    proof_requested: bool,
    run: Callable[..., None],
    check_coracle_source: Callable[[str, Path], None],
) -> None:
    output = base / "effectful-return"
    run(compiler, "--target", "rust", "--skip-zk",
        str(effectful_return_source), str(output))
    ir = json.loads((output / "contract/compact-rust-ir.json").read_text())
    assert ir["schema_version"] == 20
    circuit, = ir["stateful_circuits"]
    assert circuit["name"] == "choose" and circuit["actions"] == []
    returned = circuit["return_value"]
    assert returned["kind"] == "effectful"
    outer = returned["body"]
    assert outer["kind"] == "let"
    assert [(row["name"], row["value"]["kind"]) for row in outer["bindings"]] == [
        ("before", "cell_read")]
    sequence = outer["result"]
    assert sequence["kind"] == "sequence" and len(sequence["actions"]) == 1
    branch = sequence["result"]
    assert branch["kind"] == "conditional"
    assert branch["condition"]["left"]["kind"] == "cell_read"
    assert branch["then"]["kind"] == "let"
    assert branch["then"]["bindings"][0]["name"] == "selected"
    assert branch["then"]["result"]["actions"][0]["kind"] == "cell_write"
    assert branch["otherwise"]["kind"] == "let"
    assert branch["otherwise"]["bindings"][0]["name"] == "offset"
    assert branch["otherwise"]["result"]["actions"][0]["kind"] == "let"
    caps = json.loads((output / "contract/rust-capabilities.json").read_text())
    assert [(row["name"], row["recorded"], row["observed_call"])
            for row in caps["circuits"]] == [("choose", True, True)]
    run(compiler, "--target", "rust", "--rust-require-recording", "--skip-zk",
        str(effectful_return_source), str(base / "strict"))
    for name, source, expected in [
        ("welcome", test_center_welcome_source, 3),
        ("bboard", test_center_bboard_source, 2),
        ("election", election_source, 5),
    ]:
        existing = base / name
        run(compiler, "--target", "rust", "--skip-zk", str(source), str(existing))
        previous = json.loads((existing / "contract/rust-capabilities.json").read_text())
        assert len(previous["circuits"]) == expected
        assert all(row["recorded"] and row["observed_call"] for row in previous["circuits"])
        existing_ir = json.loads((existing / "contract/compact-rust-ir.json").read_text())
        assert all(row["return_value"]["kind"] != "effectful"
                   for row in existing_ir["stateful_circuits"])
    check_coracle_source(compiler, base)
    if proof_requested:
        proof = base / "effectful-return-proof"
        run(compiler, "--target", "rust", "--rust-require-recording",
            str(effectful_return_source), str(proof))
        run("cargo", "run", "--quiet", "-p", "compact-rust-proof-smoke", "--",
            "--effectful-return", str(proof))
    print("typed effectful Field returns record selected effects and scoped branch results")
