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

"""Pin the bounded literal Bytes-to-Field frontend normalization (ADR168)."""
import json
import os
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[2]
COMPILER = os.environ["COMPACTC"]
MAX_FIELD = 52435875175126190479447740508185965837690552500527637822603658699938581184512


def compile_source(source, output, target="rust"):
    return subprocess.run([COMPILER, "--target", target, "--skip-zk", str(source), str(output)],
                          cwd=ROOT, capture_output=True, text=True)


with tempfile.TemporaryDirectory(prefix="compact-literal-bytes-field-") as directory:
    base = Path(directory)
    positive = compile_source(ROOT / "examples/rust_backend/literal_bytes_field_oracle.compact", base / "positive")
    assert positive.returncode == 0, positive.stderr
    ir = json.loads((base / "positive/contract/compact-rust-ir.json").read_text())
    assert all(row["body"]["kind"] == "field_literal" for row in ir["circuits"])
    assert {row["name"]: row["body"]["value"] for row in ir["circuits"]} == {
        "little_endian": "16961", "zero": "0", "boundary": str(MAX_FIELD),
        "domain": str(int.from_bytes(b"midnight:kernel:nonce_evolve", "little")),
        "domain_two": str(int.from_bytes(b"midnight:kernel:nonce_evolve/2", "little")),
    }
    report = json.loads((base / "positive/contract/rust-capabilities.json").read_text())
    assert all(row["recorded"] and row["observed_call"] for row in report["circuits"])
    cases = {
        "dynamic": ('export pure circuit cast(x: Bytes<32>): Field { return x as Field; }', "requires literal bytes"),
        "dynamic-vector": ('export pure circuit cast(x: Uint<8>): Field { return ([x, 0] as Bytes<2>) as Field; }', "requires literal bytes"),
        "empty": ('export pure circuit cast(): Field { return "" as Field; }', "cannot cast"),
        "uint-target": ('export pure circuit cast(): Uint<16> { return "AB" as Uint<16>; }', "literal Bytes-to-Field casts only"),
    }
    for name, number in [("overflow", MAX_FIELD + 1), ("all-ff", 2**256 - 1)]:
        vector = list(number.to_bytes(32, "little"))
        cases[name] = (f'export pure circuit cast(): Field {{ return ({vector} as Bytes<32>) as Field; }}', "exceeds maximum value")
    for name, (source_text, diagnostic) in cases.items():
        source = base / f"{name}.compact"
        source.write_text(source_text + "\n")
        rejected = compile_source(source, base / name)
        assert rejected.returncode != 0, name
        assert diagnostic in rejected.stderr, (name, rejected.stderr)
        assert not (base / name / "contract/lib.rs").exists(), name
        if name in ("overflow", "all-ff"):
            # TS accepts the source, then its generated checked cast rejects on evaluation.
            ts = compile_source(source, base / f"{name}-ts", "ts")
            assert ts.returncode == 0, ts.stderr
            modules = base / f"{name}-ts/node_modules/@midnight-ntwrk"
            modules.mkdir(parents=True)
            (modules / "compact-runtime").symlink_to(Path(os.environ.get("COMPACT_TS_RUNTIME_DIR", ROOT / "runtime")), target_is_directory=True)
            probe = subprocess.run(["node", "--input-type=module", "-e",
                f'import {{pureCircuits}} from "{(base / f"{name}-ts/contract/index.js").as_uri()}"; '
                'try { pureCircuits.cast(); process.exit(1); } catch (error) { '
                'if (!error.message.includes("maximum")) throw error; }'], capture_output=True, text=True)
            assert probe.returncode == 0, probe.stderr
    # Direct stateful return shares normalization, while recording retains its existing gap.
    source = base / "state-return.compact"
    source.write_text('export ledger x: Field; export circuit cast(): Field { assert(x == 0, "zero"); return "AB" as Field; }\n')
    result = compile_source(source, base / "state-return")
    assert result.returncode == 0, result.stderr
    ir = json.loads((base / "state-return/contract/compact-rust-ir.json").read_text())
    assert ir["stateful_circuits"][0]["return_value"] == {"kind": "expression", "value": {"kind": "field_literal", "value": "16961"}}
print("literal Bytes-to-Field: canonical values, recording composition, six rejection guards, TS overflow timing, stateful return passed")
