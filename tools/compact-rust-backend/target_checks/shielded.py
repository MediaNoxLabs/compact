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

"""Focused compile, capability and optional proof checks for shielded operations.

The CLI owns scenario selection, compiler/source paths and the temporary directory.
Each check retains its own acceptance assertions and ordered proof commands.
"""

from collections.abc import Callable
import json
from pathlib import Path


def check_receive(
    compiler: str, base: Path, source: Path, *, proof: bool, run: Callable[..., None]
) -> None:
    output = base / "shielded-receive"
    run(compiler, "--target", "rust", "--rust-require-recording",
        *([] if proof else ["--skip-zk"]),
        str(source), str(output))
    ir = json.loads((output / "contract/compact-rust-ir.json").read_text())
    assert ir["schema_version"] == 20
    report = json.loads((output / "contract/rust-capabilities.json").read_text())
    assert {row["name"] for row in report["circuits"]} == {"accept", "accept_renamed"}
    assert all(row["proof_required"] and row["recorded"] and row["observed_call"]
               for row in report["circuits"])
    rust = (output / "contract/lib.rs").read_text()
    assert "create_zswap_output" in rust and "CoinReceive" in rust
    assert "crate::pure_circuits::coinCommitment" in rust
    if proof:
        run("cargo", "+1.99.0", "run", "--offline", "--quiet", "-p",
            "compact-rust-proof-smoke", "--", "--shielded-receive", str(output))
        # Reuse the same source and keys for explicit wallet funding;
        # keep this default-strict acceptance in the full proof gate.
        run("cargo", "+1.99.0", "run", "--offline", "--quiet", "-p",
            "compact-rust-proof-smoke", "--", "--wallet-funded-receive", str(output))
        # Same keys, additional explicit observation-policy strict proof.
        run("cargo", "+1.99.0", "run", "--offline", "--quiet", "-p",
            "compact-rust-proof-smoke", "--", "--observational-receive", str(output))
    print("unchanged receiveShielded wrappers admitted with ordered output and audited claim")


def check_merge(
    compiler: str, base: Path, source: Path, *, proof: bool, run: Callable[..., None]
) -> None:
    output = base / "shielded-merge"
    run(compiler, "--target", "rust", "--rust-require-recording",
        *([] if proof else ["--skip-zk"]), str(source), str(output))
    report = json.loads((output / "contract/rust-capabilities.json").read_text())
    assert {row["name"] for row in report["circuits"]} == {"merge_qualified", "receive_then_merge"}
    assert all(row["proof_required"] and row["recorded"] and row["observed_call"] for row in report["circuits"])
    if proof:
        run("cargo", "+1.99.0", "run", "--offline", "--quiet", "-p",
            "compact-rust-proof-smoke", "--", "--shielded-merge", str(output))
    print("unchanged qualified and immediate merge helpers recorded with checked wide arithmetic")


def check_send(
    compiler: str, base: Path, source: Path, *, proof: bool, run: Callable[..., None]
) -> None:
    output = base / "shielded-send"
    run(compiler, "--target", "rust", "--rust-require-recording",
        *([] if proof else ["--skip-zk"]),
        str(source), str(output))
    ir = json.loads((output / "contract/compact-rust-ir.json").read_text())
    assert ir["schema_version"] == 20
    report = json.loads((output / "contract/rust-capabilities.json").read_text())
    assert {row["name"] for row in report["circuits"]} == {
        "send_to_self", "send_to_user", "send_to_contract", "send_forward"}
    assert all(row["proof_required"] and row["recorded"] and row["observed_call"]
               for row in report["circuits"])
    rust = (output / "contract/lib.rs").read_text()
    for primitive in ("create_zswap_input", "create_zswap_output", "subtract_unsigned",
                      "degrade_to_transient", "transient_hash", "upgrade_from_transient"):
        assert primitive in rust
    if proof:
        run("cargo", "+1.99.0", "run", "--offline", "--quiet", "-p",
            "compact-rust-proof-smoke", "--", "--shielded-send", str(output))
        # Same keys, additional explicit observation-policy strict proof.
        run("cargo", "+1.99.0", "run", "--offline", "--quiet", "-p",
            "compact-rust-proof-smoke", "--", "--observational-send", str(output))
    print("unchanged qualified sendShielded wrappers admitted with ordered intents and typed result")
