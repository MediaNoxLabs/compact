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

"""Shielded intent, kernel, and qualified-coin target checks.

The CLI owns flag precedence, environment setup, and temporary-directory lifetime.
Sources and command runners are explicit inputs; each check retains its own
acceptance assertions, failure order, and optional proof commands.
"""

from collections.abc import Callable
import json
from pathlib import Path
from subprocess import CompletedProcess


def check_transient_receive_send(
    compiler: str,
    base: Path,
    *,
    root: Path,
    proof_requested: bool,
    run: Callable[..., None],
) -> None:
    source = root / "examples/rust_backend/transient_receive_send_oracle.compact"
    output = base / "transient-receive-send"
    run(compiler, "--target", "rust", "--rust-require-recording", "--skip-zk", str(source), str(output))
    info = json.loads((output / "compiler/contract-info.json").read_text())
    report = json.loads((output / "contract/rust-capabilities.json").read_text())
    assert {c["name"] for c in info["circuits"] if c["proof"]} == {"receive_then_send"}
    assert len(report["circuits"]) == 1
    row = report["circuits"][0]
    assert row["name"] == "receive_then_send" and row["proof_required"] and row["recorded"] and row["observed_call"]
    run("cargo", "+1.99.0", "check", "--offline", "--manifest-path", str(output / "contract/Cargo.toml"))
    if proof_requested:
        (output / "keys").mkdir(exist_ok=True)
        run("zkir", "compile", str(output / "zkir/receive_then_send.zkir"), str(output / "keys/receive_then_send.prover"), str(output / "keys/receive_then_send.verifier"))
        run("cargo", "run", "--quiet", "-p", "compact-rust-proof-smoke", "--", "--transient-receive-send", str(output))
    print("unchanged receive/full immediate-send: one proof-required recorded API; optional strict wallet/transient proof")


def check_canonical_output_order(
    compiler: str,
    base: Path,
    *,
    root: Path,
    proof_requested: bool,
    run: Callable[..., None],
) -> None:
    source = root / "examples/rust_backend/canonical_output_order_oracle.compact"
    output = base / "canonical-output-order"
    run(compiler, "--target", "rust", "--rust-require-recording", "--skip-zk", str(source), str(output))
    info = json.loads((output / "compiler/contract-info.json").read_text())
    report = json.loads((output / "contract/rust-capabilities.json").read_text())
    names = {"distribute", "read_coin"}
    assert {c["name"] for c in info["circuits"] if c["proof"]} == names
    assert {c["name"] for c in report["circuits"]} == names
    assert all(c["proof_required"] and c["recorded"] and c["observed_call"] for c in report["circuits"])
    run("cargo", "+1.99.0", "check", "--offline", "--manifest-path", str(output / "contract/Cargo.toml"))
    if proof_requested:
        (output / "keys").mkdir(exist_ok=True)
        run("zkir", "compile", str(output / "zkir/distribute.zkir"), str(output / "keys/distribute.prover"), str(output / "keys/distribute.verifier"))
        run("cargo", "run", "--quiet", "-p", "compact-rust-proof-smoke", "--", "--canonical-output-order", str(output))
    print("canonical output source: two proof-required recorded APIs; optional strict proof for both output sort orders")


def check_composite_zswap(
    compiler: str,
    base: Path,
    *,
    root: Path,
    proof_requested: bool,
    run: Callable[..., None],
) -> None:
    outputs = []
    for stem, names in [("stateful_struct_oracle", {"snapshot", "reverse", "nested", "planned"}), ("composite_zswap_transfer_oracle", {"transfer"})]:
        output = base / stem
        run(compiler, "--target", "rust", "--rust-require-recording", *([] if proof_requested else ["--skip-zk"]), str(root / f"examples/rust_backend/{stem}.compact"), str(output))
        report = json.loads((output / "contract/rust-capabilities.json").read_text())
        assert {row["name"] for row in report["circuits"]} == names
        assert all(row["recorded"] and row["observed_call"] and row["proof_required"] for row in report["circuits"])
        outputs.append(output)
    if proof_requested:
        run("cargo", "run", "--quiet", "-p", "compact-rust-proof-smoke", "--", "--composite-zswap", *(str(output) for output in outputs))
    print("composite Unit intents and same-frame Kernel queries admitted; exact offer policy retained")


def check_kernel_shielded_effects(
    compiler: str,
    base: Path,
    *,
    root: Path,
    proof_requested: bool,
    run: Callable[..., None],
    capture: Callable[..., CompletedProcess[str]],
) -> None:
    source = root / "examples/rust_backend/kernel_shielded_effects_oracle.compact"
    output = base / "kernel-shielded-effects"
    run(compiler, "--target", "rust", "--skip-zk", str(source), str(output))
    ir = json.loads((output / "contract/compact-rust-ir.json").read_text())
    assert ir["schema_version"] == 20
    assert [f["id"] for f in ir["ledger_fields"]] == ["marker"]
    encoded = json.dumps(ir)
    assert '"kernel_claim"' in encoded and '"kernel_mint_shielded"' in encoded
    report = json.loads((output / "contract/rust-capabilities.json").read_text())
    assert len(report["circuits"]) == 8
    for row in report["circuits"]:
        assert row["recorded"] and row["observed_call"]
        assert row["proof_required"]
    rejected = capture([compiler, "--target", "rust", "--rust-require-recording", "--skip-zk", str(source), str(base / "requires-recording")], cwd=root, capture_output=True, text=True)
    assert rejected.returncode == 0, rejected.stderr
    assert (base / "requires-recording/contract/lib.rs").exists()
    unsupported = base / "unsupported-kernel.compact"
    unsupported.write_text("import CompactStandardLibrary; export circuit check(): [] { return kernel.checkpoint(); }\n")
    rejected = capture([compiler, "--target", "rust", "--skip-zk", str(unsupported), str(base / "unsupported-kernel")], cwd=root, capture_output=True, text=True)
    assert rejected.returncode != 0 and "Kernel operation checkpoint" in rejected.stderr
    if proof_requested:
        proof = base / "kernel-shielded-effects-proof"
        run(compiler, "--target", "rust", "--rust-require-recording", str(source), str(proof))
        run("cargo", "run", "--quiet", "-p", "compact-rust-proof-smoke", "--",
            "--kernel-shielded-effects", str(proof))
    print("typed recorded Kernel effects admitted; unsupported operations refused")


def check_native_zswap_intents(
    compiler: str,
    base: Path,
    *,
    root: Path,
    proof_requested: bool,
    run: Callable[..., None],
) -> None:
    output = base / "native-zswap-intents"
    source = root / "examples/rust_backend/native_zswap_intents_oracle.compact"
    run(compiler, "--target", "rust", "--skip-zk", str(source), str(output))
    ir = json.loads((output / "contract/compact-rust-ir.json").read_text())
    assert ir["schema_version"] == 20
    encoded = json.dumps(ir)
    assert '"create_zswap_input"' in encoded and '"create_zswap_output"' in encoded
    report = json.loads((output / "contract/rust-capabilities.json").read_text())
    assert [(row["name"], row["recorded"], row["observed_call"], row["proof_required"]) for row in report["circuits"]] == [
        ("produce", False, False, False), ("consume", False, False, False),
        ("flow", True, True, True), ("witness_order", False, False, False),
        ("read_coin", True, True, True)]
    run(compiler, "--target", "rust", "--rust-require-recording", "--skip-zk", str(source), str(base / "requires-recording"))
    transfer_source = root / "examples/rust_backend/zswap_transfer_oracle.compact"
    transfer = base / "zswap-transfer"
    run(compiler, "--target", "rust", "--rust-require-recording", *([] if proof_requested else ["--skip-zk"]), str(transfer_source), str(transfer))
    transfer_report = json.loads((transfer / "contract/rust-capabilities.json").read_text())
    assert [(row["name"], row["recorded"], row["observed_call"], row["proof_required"]) for row in transfer_report["circuits"]] == [
        ("transfer", True, True, True), ("read_coin", True, True, True)]
    if proof_requested:
        run("cargo", "run", "--quiet", "-p", "compact-rust-proof-smoke", "--", "--zswap-transfer", str(transfer))
    print("recorded Zswap intents and exact offer-bound transfer admitted; original duplicate flow remains rejection evidence")


def check_qualified_coin_cell(
    compiler: str,
    base: Path,
    *,
    root: Path,
    proof_requested: bool,
    run: Callable[..., None],
) -> None:
    output = base / "qualified-coin-cell"
    source = root / "examples/rust_backend/qualified_coin_cell_oracle.compact"
    run(compiler, "--target", "rust", "--skip-zk", str(source), str(output))
    ir = json.loads((output / "contract/compact-rust-ir.json").read_text())
    assert ir["schema_version"] == 20
    action = next(row for row in ir["stateful_circuits"] if row["name"] == "write_coin")["actions"][0]
    assert action["kind"] == "cell_write_coin"
    report = json.loads((output / "contract/rust-capabilities.json").read_text())
    write = next(row for row in report["circuits"] if row["name"] == "write_coin")
    assert write["recorded"] and write["observed_call"]
    assert write.get("recording_unavailable") is None
    run(compiler, "--target", "rust", "--skip-zk", "--rust-require-recording", str(source), str(base / "required"))
    assert (base / "required/contract/lib.rs").exists()
    chunked = base / "qualified-coin-cell-chunked"
    run(compiler, "--target", "rust", "--skip-zk", "--rust-require-recording",
        str(root / "runtime-rs/tests/fixtures/qualified-coin-cell-chunked.compact"), str(chunked))
    generated = (chunked / "contract/lib.rs").read_text()
    assert "&[1u8,14u8]" in "".join(generated.split())
    assert ".record_write_coin(" in generated
    chunked_report = json.loads((chunked / "contract/rust-capabilities.json").read_text())
    chunked_write = next(row for row in chunked_report["circuits"] if row["name"] == "write_coin")
    assert chunked_write["recorded"] and chunked_write["observed_call"]
    if proof_requested:
        proof = base / "qualified-coin-cell-proof"
        run(compiler, "--target", "rust", "--rust-require-recording", str(source), str(proof))
        run("cargo", "run", "--quiet", "-p", "compact-rust-proof-smoke", "--",
            "--qualified-coin-cell", str(proof))
    print("qualified-coin Cell recorded/observed admission passed")


def check_adt_set_qualified(
    compiler: str,
    base: Path,
    *,
    root: Path,
    adt_set_qualified_source: Path,
    qualified_coin_oracle_source: Path,
    proof_requested: bool,
    run: Callable[..., None],
    capture: Callable[..., CompletedProcess[str]],
) -> None:
    output = base / "adt-set-qualified"
    run(compiler, "--target", "rust", "--skip-zk",
        str(adt_set_qualified_source), str(output))
    ir = json.loads((output / "contract/compact-rust-ir.json").read_text())
    assert ir["schema_version"] == 20
    def has_coin_insert(value):
        if isinstance(value, dict):
            return value.get("kind") == "set_insert_coin" or any(
                has_coin_insert(part) for part in value.values())
        if isinstance(value, list):
            return any(has_coin_insert(part) for part in value)
        return False
    assert has_coin_insert(ir["stateful_circuits"][1])
    metadata = json.loads((output / "compiler/contract-info.json").read_text())
    assert [(row["name"], row["pure"], row["proof"])
            for row in metadata["circuits"]] == [
        ("test_QualifiedShieldedCoinInfo", False, True),
        ("test_ShieldedCoinInfo", False, True),
    ]
    capabilities = json.loads((output / "contract/rust-capabilities.json").read_text())
    assert [(row["name"], row["proof_required"], row["recorded"], row["observed_call"])
            for row in capabilities["circuits"]] == [
        ("test_QualifiedShieldedCoinInfo", True, True, True),
        ("test_ShieldedCoinInfo", True, True, True),
    ]
    assert all(row.get("recording_unavailable") is None for row in capabilities["circuits"])
    required = capture(
        [compiler, "--target", "rust", "--rust-require-recording", "--skip-zk",
         str(adt_set_qualified_source), str(base / "qualified-requires-recording")],
        cwd=root, capture_output=True, text=True,
    )
    assert required.returncode == 0, required.stderr
    oracle = base / "qualified-oracle"
    run(compiler, "--target", "rust", "--skip-zk",
        str(qualified_coin_oracle_source), str(oracle))
    oracle_capabilities = json.loads((oracle / "contract/rust-capabilities.json").read_text())
    assert [(row["name"], row["recorded"], row["observed_call"])
            for row in oracle_capabilities["circuits"]] == [
        ("insert_coin", True, True), ("contains", True, True)]
    # Keep frontend-reachable nested insertion in the maintained gate.
    # Its canonical carrier interface composes through the recording algebra;
    # runtime execution and source-bound proof are qualified separately.
    chunked = base / "qualified-coin-set-chunked"
    run(compiler, "--target", "rust", "--skip-zk",
        str(root / "runtime-rs/tests/fixtures/qualified-coin-set-chunked.compact"),
        str(chunked))
    chunked_ir = json.loads((chunked / "contract/compact-rust-ir.json").read_text())
    assert has_coin_insert(chunked_ir["stateful_circuits"])
    generated = (chunked / "contract/lib.rs").read_text()
    assert "&[1u8,14u8]" in "".join(generated.split())
    assert "ledger_slots::pot.insert_coin(" in "".join(generated.split())
    chunked_caps = json.loads((chunked / "contract/rust-capabilities.json").read_text())
    assert [(row["name"], row["recorded"], row["observed_call"])
            for row in chunked_caps["circuits"]] == [
        ("insert_coin", True, True), ("contains", True, True)]
    assert all(row.get("recording_unavailable") is None for row in chunked_caps["circuits"])
    assert "ledger_slots::pot.record_insert_coin(" in "".join(generated.split())
    if proof_requested:
        adt_proof = base / "adt-qualified-proof"
        run(compiler, "--target", "rust", "--rust-require-recording",
            str(adt_set_qualified_source), str(adt_proof))
        oracle_proof = base / "qualified-oracle-proof"
        run(compiler, "--target", "rust", "--rust-require-recording",
            str(qualified_coin_oracle_source), str(oracle_proof))
        run("cargo", "run", "--quiet", "-p", "compact-rust-proof-smoke", "--",
            "--adt-set-qualified-coin-info", str(adt_proof))
        run("cargo", "run", "--quiet", "-p", "compact-rust-proof-smoke", "--",
            "--qualified-coin-set", str(oracle_proof))
        chunked_proof = base / "qualified-chunked-proof"
        run(compiler, "--target", "rust", "--rust-require-recording",
            str(root / "runtime-rs/tests/fixtures/qualified-coin-set-chunked.compact"),
            str(chunked_proof))
        run("cargo", "run", "--quiet", "-p", "compact-rust-proof-smoke", "--",
            "--qualified-coin-set-chunked", str(chunked_proof))
    print("complete ADT qualified Set source and qualified coin oracle recorded; "
          "chunked native and recorded path admission checked")
