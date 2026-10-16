#!/usr/bin/env python3
# This file is part of Compact.
# Copyright (C) 2026 contributors to Minokawa Compact
# SPDX-License-Identifier: Apache-2.0

"""Sequential real proof smoke controls for the matching ledger8.1.3 Forge profile.

These two fixtures are examples, not a cache whitelist. All matching current
circuits may use Forge; --uncached-zkir retains fresh key-generation qualification.
"""

import argparse
import hashlib
import json
import os
from pathlib import Path
import resource
import shutil
import subprocess
import sys
import tempfile
import time
import tomllib
import unittest

from check_compactc_target import check_manifest

import forge_circuits
from forge_circuits import (BUILD, CONSUMER, FORGE, PROFILE, CompatibilityError,
                            digest, require, validate_export, prepare)

ROOT = Path(__file__).resolve().parents[2]
QUALIFICATIONS = Path(__file__).with_name("forge_circuit_qualification.json")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--compiler", type=Path, required=True)
    parser.add_argument("--proof-runner", type=Path, required=True)
    parser.add_argument("--forge", type=Path, default=FORGE)
    parser.add_argument("--params-dir", type=Path, required=True)
    parser.add_argument("--cache-dir", type=Path, help="otherwise Forge uses its normal default/env")
    parser.add_argument("--uncached-zkir", type=Path, help="qualify fresh legacy keys; bypass Forge")
    parser.add_argument("--report", type=Path, required=True)
    args = parser.parse_args()
    for name in ("compiler", "proof_runner", "params_dir"):
        setattr(args, name, getattr(args, name).resolve(strict=True))
    if not args.uncached_zkir:
        args.forge = args.forge.resolve(strict=True)
    start = time.perf_counter()
    child_env = dict(os.environ, MIDNIGHT_PP=str(args.params_dir), COMPACT_ZKIR_FORGE_BIN=str(args.forge))
    if args.cache_dir is not None:
        child_env["MIDNIGHT_ZKIR_FORGE_DIR"] = str(args.cache_dir.resolve())
    forge_circuits.validate_consumer()
    qualifications = json.loads(QUALIFICATIONS.read_text())
    report = {"scope": "two proof smoke examples of the matching profile; not a circuit whitelist",
              "mode": "uncached" if args.uncached_zkir else "forge", "cases": [],
              "consumer": CONSUMER, "forge_profile": PROFILE,
              "proof_runner_sha256": digest(args.proof_runner),
              "compiler_sha256": digest(args.compiler)}

    def stats():
        argv = [str(args.forge)]
        if args.cache_dir is not None:
            argv += ["--cache-dir", str(args.cache_dir)]
        return json.loads(subprocess.run(argv + ["stats"], check=True,
                                         capture_output=True, text=True).stdout)

    if not args.uncached_zkir:
        report["stats_before"] = stats()

    class CircuitTests(unittest.TestCase):
        def exercise(self, name):
            case_start = time.perf_counter()
            q = qualifications[name]
            row = {"circuit": name}
            report["cases"].append(row)

            def run(argv, phase, *, expected_success=True):
                t = time.perf_counter()
                result = subprocess.run([str(x) for x in argv], cwd=ROOT,
                                        env=child_env, capture_output=True, text=True)
                row[phase] = {"wall_seconds": time.perf_counter() - t,
                              "exit_code": result.returncode, "stdout": result.stdout,
                              "stderr": result.stderr}
                if expected_success:
                    result.check_returncode()
                else:
                    self.assertNotEqual(result.returncode, 0, "corrupt verifier accepted")
                return result

            with tempfile.TemporaryDirectory(prefix="compact-forge-test-") as temp:
                output = Path(temp) / "generated"
                run([args.compiler, "--target", "rust", "--skip-zk", "--rust-runtime-root",
                     ROOT, ROOT / q["compact_source"], output], "compile")
                check_manifest(output)
                circuit = q["circuit"]
                source = output / "zkir" / f"{circuit}.zkir"
                self.assertEqual(digest(args.params_dir / f"bls_midnight_2p{q['k']}"),
                                 q["parameter_digest"])
                if q.get("rust_fixture"):
                    run(["rustfmt", "--edition", "2024", output / "contract/lib.rs"], "format")
                    self.assertEqual((output / "contract/lib.rs").read_bytes(),
                                     (ROOT / q["rust_fixture"]).read_bytes())
                if args.uncached_zkir:
                    (output / "keys").mkdir(exist_ok=True)
                    run([args.uncached_zkir, "compile", source,
                         output / "keys" / f"{circuit}.prover",
                         output / "keys" / f"{circuit}.verifier"], "uncached_keygen")
                    paths = {"source": source, "binary_ir": source.with_suffix(".bzkir"),
                             "prover": output / "keys" / f"{circuit}.prover",
                             "verifier": output / "keys" / f"{circuit}.verifier"}
                else:
                    actual_zkir = forge_circuits.zkir_executable(child_env)
                    require(actual_zkir is not None, "matching actual zkir CLI required for fresh model/serialization checks")
                    row["forge"] = forge_circuits.compile_circuit(source,
                        output / "keys" / f"{circuit}.prover", output / "keys" / f"{circuit}.verifier",
                        actual_zkir, child_env)
                    paths = {"source": source, "binary_ir": source.with_suffix(".bzkir"),
                             "prover": output / "keys" / f"{circuit}.prover",
                             "verifier": output / "keys" / f"{circuit}.verifier"}
                artifact_hashes = {kind: digest(path) for kind, path in paths.items()}
                row["artifact_hashes"] = artifact_hashes
                proof = run([args.proof_runner, q["selector"], output], "proof_and_assertions")
                self.assertIn(q["success_marker"], proof.stdout)
                # Keep negative mutation private; never alter persistent Forge exports.
                negative = Path(temp) / "negative"
                shutil.copytree(output, negative, symlinks=False)
                (negative / "keys" / f"{circuit}.verifier").write_bytes(b"invalid verifier")
                run([args.proof_runner, q["selector"], negative], "corrupt_verifier_control",
                    expected_success=False)
                for kind, path in paths.items():
                    self.assertEqual(digest(path), artifact_hashes[kind], "artifact mutated by consumer")
            row["complete_case_wall_seconds"] = time.perf_counter() - case_start

        def test_counter_two_branches(self):
            self.exercise("counter")

        def test_let_return(self):
            self.exercise("let_return")

    result = unittest.TextTestRunner(verbosity=2).run(
        unittest.defaultTestLoader.loadTestsFromTestCase(CircuitTests))
    if not args.uncached_zkir:
        report["stats_after"] = stats()
    report.update(passed=result.wasSuccessful(), tests_run=result.testsRun,
                  complete_wall_seconds=time.perf_counter() - start,
                  cumulative_child_largest_process_maxrss_bytes=(
                      resource.getrusage(resource.RUSAGE_CHILDREN).ru_maxrss
                      * (1 if sys.platform == "darwin" else 1024)),
                  memory_scope="cumulative child largest-process highwater, not concurrent total")
    args.report.write_text(json.dumps(report, indent=2) + "\n")
    return 0 if result.wasSuccessful() else 1


if __name__ == "__main__":
    raise SystemExit(main())
