#!/usr/bin/env python3

# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#   http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.

"""Run a local Rust parity slice against one frozen compiler and write a receipt.

Focused mode is the default. It compiles selected Compact fixtures, checks their
generated Rust and capabilities, then runs only their Cargo packages. --full is
the explicit broad workspace, consumer and proof gate. Neither mode uses GitHub.
"""

import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import time
import tomllib

sys.dont_write_bytecode = True
import parity_inventory as inventory


ROOT = Path(__file__).resolve().parents[2]
SOURCES = ROOT / "examples/rust_backend"
FIXTURES = ROOT / "tests-rust-backend"
EXTRA = {
    SOURCES / "digital-passport-credential/src/digital-passport-credential.compact":
        FIXTURES / "passport-dogfood/lib.rs",
    ROOT / "examples/bugs/pm-19252/example_ten.compact":
        FIXTURES / "pm-19252-own-public-key/lib.rs",
    ROOT / "examples/adt/tests/set_enum.compact":
        FIXTURES / "adt-set-enum/lib.rs",
    ROOT / "examples/adt/tests/set_vector.compact":
        FIXTURES / "adt-set-vector/lib.rs",
    ROOT / "examples/adt/tests/list_field.compact":
        FIXTURES / "adt-list-field/lib.rs",
    ROOT / "examples/adt/tests/list_enum.compact":
        FIXTURES / "adt-list-enum/lib.rs",
    ROOT / "test-center/test-contracts/counter.compact":
        FIXTURES / "test-center-counter/lib.rs",
    ROOT / "test-center/test-contracts/bboard.compact":
        FIXTURES / "test-center-bboard/lib.rs",
    ROOT / "test-center/test-contracts/welcome.compact":
        FIXTURES / "test-center-welcome/lib.rs",
    ROOT / "examples/adt/tests/list_vector_field_4.compact":
        FIXTURES / "adt-list-vector-field-4/lib.rs",
    ROOT / "examples/adt/tests/list_bytes.compact":
        FIXTURES / "adt-list-bytes/lib.rs",
    ROOT / "examples/bugs/pm-19252/example_seven.compact":
        FIXTURES / "pm-19252-unused-read-seven" / "lib.rs",
    ROOT / "examples/bugs/pm-19252/example_eight_a.compact":
        FIXTURES / "pm-19252-unused-read-eight-a" / "lib.rs",
    ROOT / "examples/bugs/pm-19252/example_eight_b.compact":
        FIXTURES / "pm-19252-unused-read-eight-b" / "lib.rs",
}


class GateError(Exception):
    pass


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as file:
        for chunk in iter(lambda: file.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def git_head() -> str:
    return subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip()


def dirty_paths() -> list[str]:
    output = subprocess.check_output(["git", "status", "--porcelain", "--untracked-files=normal"],
                                     cwd=ROOT, text=True)
    return output.splitlines()


def fixture_map() -> dict[Path, Path]:
    mapping = {source.resolve(): FIXTURES / source.stem.replace("_", "-") / "lib.rs"
               for source in SOURCES.glob("*.compact")}
    mapping.update({source.resolve(): fixture for source, fixture in EXTRA.items()})
    return mapping


def select_sources(names: list[str], full: bool) -> list[tuple[Path, Path]]:
    mapping = fixture_map()
    if full:
        if names:
            raise GateError("--full cannot be combined with --source")
        return sorted(mapping.items())
    names = names or ["counter_parameter"]
    selected = []
    for name in names:
        argument = Path(name)
        if not argument.suffix:
            source = SOURCES / f"{name}.compact"
        elif argument.parent == Path("."):
            source = SOURCES / argument
        else:
            source = ROOT / argument
        source = source.resolve()
        if source not in mapping:
            raise GateError(f"no generated fixture for selected Compact source: {name}")
        selected.append((source, mapping[source]))
    if len({source for source, _ in selected}) != len(selected):
        raise GateError("duplicate --source selection")
    return sorted(selected)


def stable_copy(source: Path, destination: Path) -> dict:
    if not source.is_file() or not os.access(source, os.X_OK):
        raise GateError(f"compiler component is not executable: {source}")
    for _ in range(3):
        before = sha256(source)
        temporary = destination.with_suffix(".partial")
        shutil.copyfile(source, temporary)
        after = sha256(source)
        copied = sha256(temporary)
        if before == after == copied:
            temporary.chmod(0o555)
            temporary.replace(destination)
            return {"source": str(source), "snapshot": str(destination),
                    "sha256": copied, "size": destination.stat().st_size}
        temporary.unlink(missing_ok=True)
    raise GateError(f"compiler changed during snapshot: {source}")


def run(command: list[str], label: str, directory: Path, receipt: dict,
        *, env: dict[str, str] | None = None) -> None:
    log = directory / "logs" / f"{len(receipt['commands']):03d}-{label}.log"
    log.parent.mkdir(parents=True, exist_ok=True)
    started = time.monotonic()
    with log.open("w") as output:
        result = subprocess.run(command, cwd=ROOT, env=env, stdout=output,
                                stderr=subprocess.STDOUT, check=False)
    receipt["commands"].append({"label": label, "argv": command,
                                "exit_code": result.returncode,
                                "seconds": round(time.monotonic() - started, 3),
                                "log": str(log)})
    if result.returncode:
        tail = "\n".join(log.read_text(errors="replace").splitlines()[-12:])
        raise GateError(f"{label} failed ({result.returncode}):\n{tail}")


def validate_report(report: dict, source: Path) -> None:
    if report.get("schema_version") != 3:
        raise GateError(f"{source.name}: expected capability schema 3")
    for circuit in report["circuits"]:
        proof_required = circuit.get("proof_required")
        if type(proof_required) is not bool:
            raise GateError(f"{source.name}.{circuit['name']}: missing proof applicability")
        expected_status = ("not_applicable" if not proof_required else
                           "available" if circuit["recorded"] and circuit["observed_call"] else
                           "unavailable")
        if circuit.get("recording_status") != expected_status:
            raise GateError(f"{source.name}.{circuit['name']}: recording status invariant")
        for available, unavailable in (("recorded", "recording_unavailable"),
                                       ("observed_call", "observed_call_unavailable")):
            gap = circuit.get(unavailable)
            if circuit[available] == (gap is not None):
                raise GateError(f"{source.name}.{circuit['name']}: {available} reason invariant")
            if gap is not None and not all(gap.get(key) for key in
                                           ("code", "ir_node", "path", "detail")):
                raise GateError(f"{source.name}.{circuit['name']}: incomplete {unavailable}")


def proof_cross_tab(report: dict, contract_info: dict, source: Path) -> list[dict]:
    capabilities = report["circuits"]
    compiler_circuits = contract_info["circuits"]
    recorded_names = [circuit["name"] for circuit in capabilities]
    compiler_names = [circuit["name"] for circuit in compiler_circuits]
    if (len(set(recorded_names)) != len(recorded_names)
            or len(set(compiler_names)) != len(compiler_names)
            or not set(recorded_names).issubset(compiler_names)):
        raise GateError(f"{source.name}: exported capability names do not join contract-info")
    proof_by_name = {}
    for circuit in compiler_circuits:
        if type(circuit.get("proof")) is not bool:
            raise GateError(f"{source.name}.{circuit['name']}: missing boolean proof flag")
        proof_by_name[circuit["name"]] = circuit["proof"]
    rows = []
    for circuit in capabilities:
        proof = proof_by_name[circuit["name"]]
        if circuit["proof_required"] is not proof:
            raise GateError(f"{source.name}.{circuit['name']}: proof applicability disagrees with contract-info")
        if circuit["recorded"] and not proof:
            raise GateError(f"{source.name}.{circuit['name']}: recorded nonproof circuit")
        rows.append({"name": circuit["name"], "proof": proof,
                     "recorded": circuit["recorded"],
                     "observed_call": circuit["observed_call"]})
    return rows


def compare_baseline(contracts: list[dict], selected: set[str], full: bool) -> None:
    baseline = json.loads(inventory.DEFAULT_BASELINE.read_text())
    if not full:
        baseline = [row for row in baseline if row["source"] in selected]
    current = inventory.baseline_rows([{"source": contract["source"], **declaration}
                                       for contract in contracts
                                       for declaration in contract["declarations"]])
    if current != baseline:
        raise GateError("selected Compact declaration identities differ from parity_baseline.json")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source", action="append", default=[],
                        help="fixture source stem or repository-relative .compact path; repeatable")
    parser.add_argument("--full", action="store_true",
                        help="all fixtures, workspace tests/Clippy, consumer and proof gate")
    parser.add_argument("--compiler", type=Path,
                        default=Path(os.environ.get("COMPACTC", ROOT / "target/debug/compactc")))
    parser.add_argument("--scheme", type=Path,
                        help="Scheme frontend; defaults to COMPACTC_SCHEME or compiler sibling")
    parser.add_argument("--run-dir", type=Path,
                        help="new persistent receipt directory (default: unique /tmp directory)")
    parser.add_argument("--cargo-target-dir", type=Path,
                        default=ROOT / "target/compact-rust-parity-gate",
                        help="dedicated reusable Cargo target (default: target/compact-rust-parity-gate)")
    parser.add_argument("--expect-head", help="fail unless git HEAD has this SHA prefix")
    parser.add_argument("--skip-cargo", action="store_true",
                        help="focused mode: compile and compare fixtures without package tests")
    args = parser.parse_args()
    receipt = {"format_version": 1, "status": "failed", "mode": "full" if args.full else "focused",
               "commands": [], "fixtures": [], "capabilities": [], "proof_cross_tab": []}
    directory = None
    try:
        if args.full and args.skip_cargo:
            raise GateError("--skip-cargo is only available in focused mode")
        selected = select_sources(args.source, args.full)
        head = git_head()
        if args.expect_head and not head.startswith(args.expect_head):
            raise GateError(f"HEAD {head} does not match {args.expect_head}")
        if args.run_dir:
            proposed = args.run_dir.resolve()
            proposed.mkdir(parents=True, exist_ok=False)
            directory = proposed
        else:
            directory = Path(tempfile.mkdtemp(prefix="compact-local-parity-")).resolve()
        (directory / "bin").mkdir()
        (directory / "compiled").mkdir()
        compiler = args.compiler.resolve()
        scheme = (args.scheme or Path(os.environ.get("COMPACTC_SCHEME", compiler.with_name("compactc-scheme")))).resolve()
        receipt["git_head"] = head
        receipt["working_tree_before"] = dirty_paths()
        receipt["run_dir"] = str(directory)
        receipt["compiler"] = stable_copy(compiler, directory / "bin/compactc")
        receipt["scheme"] = stable_copy(scheme, directory / "bin/compactc-scheme")
        snapshot = directory / "bin/compactc"
        environment = os.environ.copy()
        environment.update({"COMPACTC": str(snapshot),
                            "COMPACTC_SCHEME": str(directory / "bin/compactc-scheme"),
                            "COMPACT_RUST_RUNTIME_DIR": str(ROOT),
                            "CARGO_TARGET_DIR": str(args.cargo_target_dir.resolve()),
                            "PYTHONDONTWRITEBYTECODE": "1",
                            "CARGO_TERM_COLOR": "never",
                            "RUSTUP_TOOLCHAIN": "1.99.0"})
        receipt["cargo_target_dir"] = environment["CARGO_TARGET_DIR"]
        receipt["rust_toolchain"] = environment["RUSTUP_TOOLCHAIN"]
        selected_paths = {inventory.relative_source(source, ROOT) for source, _ in selected}
        paths = inventory.source_paths(ROOT, []) if args.full else [source for source, _ in selected]
        contracts = [inventory.parse_source(path, ROOT) for path in paths]
        compare_baseline(contracts, selected_paths, args.full)
        receipt["metadata"] = inventory.receipt_metadata(ROOT, snapshot, contracts)
        packages = []
        for source, fixture in selected:
            relative = inventory.relative_source(source, ROOT)
            output = directory / "compiled" / source.stem
            run([str(snapshot), "--target", "rust", "--skip-zk", "--rust-runtime-root",
                 str(ROOT), str(source), str(output)], f"compile-{source.stem}", directory,
                receipt, env=environment)
            generated = output / "contract/lib.rs"
            report = json.loads((output / "contract/rust-capabilities.json").read_text())
            validate_report(report, source)
            contract_info = json.loads((output / "compiler/contract-info.json").read_text())
            proof_rows = proof_cross_tab(report, contract_info, source)
            run(["rustfmt", "--edition", "2024", str(generated)],
                f"format-{source.stem}", directory, receipt, env=environment)
            if not fixture.is_file() or generated.read_bytes() != fixture.read_bytes():
                raise GateError(f"stale or missing generated fixture: {fixture.relative_to(ROOT)}")
            package = tomllib.loads((fixture.parent / "Cargo.toml").read_text())["package"]["name"]
            packages.append(package)
            receipt["fixtures"].append({"source": relative, "source_sha256": sha256(source),
                                        "fixture": str(fixture.relative_to(ROOT)),
                                        "fixture_sha256": sha256(fixture), "package": package})
            receipt["capabilities"].append({"source": relative, "report": report})
            receipt["proof_cross_tab"].append({"source": relative, "circuits": proof_rows})
        circuits = [circuit for entry in receipt["capabilities"] for circuit in entry["report"]["circuits"]]
        proof_rows = [row for entry in receipt["proof_cross_tab"] for row in entry["circuits"]]
        receipt["capability_summary"] = {
            "exported": len(circuits),
            "recorded": sum(circuit["recorded"] for circuit in circuits),
            "observed_call": sum(circuit["observed_call"] for circuit in circuits),
            "recording_missing": sum(not circuit["recorded"] for circuit in circuits),
            "proof_eligible": sum(row["proof"] for row in proof_rows),
            "provable_missing": sum(row["proof"] and not row["recorded"] for row in proof_rows),
            "nonproof_native_only": sum(not row["proof"] for row in proof_rows),
        }
        if args.full:
            run(["cargo", "+1.99.0", "fmt", "--all", "--check"],
                "workspace-format", directory, receipt, env=environment)
            run([sys.executable, str(ROOT / "tools/compact-rust-backend/check_rejections.py")],
                "compiler-rejections", directory, receipt, env=environment)
            run([sys.executable, str(ROOT / "tools/compact-rust-backend/check_oracle_acceptance.py")],
                "oracle-acceptance", directory, receipt, env=environment)
            run([sys.executable, str(ROOT / "tools/compact-rust-backend/check_positive_source_scope.py"),
                 "--manifest", str(ROOT / "tools/compact-rust-backend/parity_positive_top_level_sources.json"),
                 "--compiler", str(snapshot), "--output", str(directory / "top-level-source-scope.json")],
                "top-level-source-scope", directory, receipt, env=environment)
            run([sys.executable, str(ROOT / "tools/compact-rust-backend/check_positive_source_scope.py"),
                 "--manifest", str(ROOT / "tools/compact-rust-backend/parity_positive_original_election_zerocash_sources.json"),
                 "--compiler", str(snapshot), "--output", str(directory / "original-election-zerocash-source-scope.json")],
                "original-election-zerocash-source-scope", directory, receipt, env=environment)
            run([sys.executable, str(ROOT / "tools/compact-rust-backend/check_positive_source_scope.py"),
                 "--manifest", str(ROOT / "tools/compact-rust-backend/parity_positive_test_center_counter_sources.json"),
                 "--compiler", str(snapshot), "--output", str(directory / "test-center-counter-source-scope.json")],
                "test-center-counter-source-scope", directory, receipt, env=environment)
            run([sys.executable, str(ROOT / "tools/compact-rust-backend/check_positive_source_scope.py"),
                 "--manifest", str(ROOT / "tools/compact-rust-backend/parity_positive_test_center_welcome_sources.json"),
                 "--compiler", str(snapshot), "--output", str(directory / "test-center-welcome-source-scope.json")],
                "test-center-welcome-source-scope", directory, receipt, env=environment)
            run([sys.executable, str(ROOT / "tools/compact-rust-backend/check_positive_source_scope.py"),
                 "--manifest", str(ROOT / "tools/compact-rust-backend/parity_positive_test_center_bboard_sources.json"),
                 "--compiler", str(snapshot), "--output", str(directory / "test-center-bboard-source-scope.json")],
                "test-center-bboard-source-scope", directory, receipt, env=environment)
            # The compactup integration tests require the machine's installed
            # compiler and the mutable GitHub release list. They are outside
            # this compiler/backend parity gate and cannot give a repeatable
            # local receipt.
            run(["cargo", "+1.99.0", "test", "--workspace", "--exclude", "compact",
                 "--all-targets", "--all-features", "--locked"],
                "backend-workspace-tests", directory, receipt, env=environment)
            run(["cargo", "+1.99.0", "test", "-p", "compact", "--lib", "--bins",
                 "--all-features", "--locked"],
                "compact-cli-unit-tests", directory, receipt, env=environment)
            run(["cargo", "+1.99.0", "clippy", "--workspace", "--exclude", "compact",
                 "--all-targets", "--all-features", "--locked", "--", "-D", "warnings"],
                "backend-workspace-clippy", directory, receipt,
                env=environment)
            run([sys.executable, str(ROOT / "tools/compact-rust-backend/check_compactc_target.py"),
                 "--consumer", "--proof"], "consumer-proof-ledger", directory, receipt,
                env=environment)
        elif not args.skip_cargo:
            run(["cargo", "+1.99.0", "test", "--locked", *sum((["-p", name] for name in packages), [])],
                "selected-cargo-tests", directory, receipt, env=environment)
        if git_head() != head:
            raise GateError("git HEAD changed during gate")
        for component in ("compiler", "scheme"):
            if sha256(Path(receipt[component]["snapshot"])) != receipt[component]["sha256"]:
                raise GateError(f"{component} snapshot changed during gate")
        for entry in receipt["fixtures"]:
            for key, hash_key in (("source", "source_sha256"),
                                  ("fixture", "fixture_sha256")):
                if sha256(ROOT / entry[key]) != entry[hash_key]:
                    raise GateError(f"{entry[key]} changed during gate")
        receipt["working_tree_after"] = dirty_paths()
        receipt["status"] = "passed"
        print(f"local parity {receipt['mode']} passed: {len(selected)} fixtures, "
              f"{receipt['capability_summary']['recorded']}/{len(circuits)} recorded; "
              f"receipt {directory / 'receipt.json'}")
        return 0
    except (GateError, OSError, ValueError, KeyError, json.JSONDecodeError,
            subprocess.CalledProcessError) as error:
        receipt["error"] = str(error)
        print(f"local parity gate: {error}", file=sys.stderr)
        return 1
    finally:
        if directory is not None:
            (directory / "receipt.json").write_text(json.dumps(receipt, indent=2, sort_keys=True) + "\n")


if __name__ == "__main__":
    raise SystemExit(main())
