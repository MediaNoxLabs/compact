#!/usr/bin/env python3
# This file is part of Compact.
# Copyright (C) 2026 contributors to Minokawa Compact
# SPDX-License-Identifier: Apache-2.0

"""Opt-in sequential Forge trial; ordinary proof gates remain uncached.

Only two exact circuits have cross-version artifact qualification. A new input,
profile, parameter set or serialization must be qualified, never guessed.
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

ROOT = Path(__file__).resolve().parents[2]
FORGE = Path.home() / ".local/bin/midnight-zkir-forge"
PROFILE = ("ledger8-8.1.3/zkir-2.1.1/transient-2.1.1/base-1.0.2/"
           "serialize-1.1.1/proofs-0.7.1/circuits-6.1.0/stdlib-1.1.0/"
           "curves-0.2.0/tagged-v1/forge-recipe-v1")
BUILD = "340e0b3bd94394d987c1b2a073bb411afb755e7164934ca15e8d8cb02b77b879"
CONSUMER = {"midnight-ledger": "8.0.3", "midnight-zkir": "2.1.0",
            "midnight-transient-crypto": "2.0.1", "midnight-base-crypto": "1.0.0",
            "midnight-serialize": "1.0.0", "midnight-proofs": "0.7.0",
            "midnight-circuits": "6.0.0", "midnight-zk-stdlib": "1.0.0"}
QUALIFICATIONS = Path(__file__).with_name("forge_circuit_qualification.json")


class CompatibilityError(ValueError):
    """The selected artifacts have no matching consumer qualification."""


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def require(condition, message):
    if not condition:
        raise CompatibilityError(message)


def validate_export(value, source, qualification, params):
    require(value.get("schema") == 1 and value.get("profile") == PROFILE
            and value.get("build_fingerprint") == BUILD, "unqualified Forge profile/build")
    require(digest(source) == qualification["source"], "current ZKIR is not qualified")
    base = Path(value["export"]["base"])
    require(base.is_absolute(), "Forge export base must be absolute")
    manifest = json.loads((base / "manifest.json").read_text())
    require(manifest["profile"] == PROFILE and manifest["build_fingerprint"] == BUILD
            and manifest["artifact_set_id"] == value["artifact_set_id"],
            "Forge export manifest identity differs")
    require(manifest["source"]["digest"] == qualification["source"]
            and manifest["k"] == qualification["k"]
            and manifest["parameter_digest"] == qualification["parameter_digest"]
            and digest(params / f"bls_midnight_2p{qualification['k']}")
            == qualification["parameter_digest"], "unqualified source/parameters")
    paths = {}
    for item in value["export"]["files"]:
        kind, path = item["kind"], Path(item["path"])
        require(kind not in paths, "duplicate Forge artifact kind")
        require(path.is_absolute() and path.resolve().is_relative_to(base.resolve()),
                "Forge artifact must be inside its persistent export")
        require(path.stat().st_size == item["bytes"] and digest(path) == item["sha256"]
                == qualification[kind], f"unqualified or corrupt {kind}")
        paths[kind] = path
    require(set(paths) == {"source", "binary_ir", "prover", "verifier"},
            "incomplete Forge export")
    require(paths["source"].read_bytes() == source.read_bytes(), "source changed during prepare")
    return paths


def prepare(source, qualification, params, *, forge=FORGE, cache_dir=None):
    source = source.resolve(strict=True)
    # Fresh input check on every invocation, before asking Forge to do any work.
    require(digest(source) == qualification["source"], "current ZKIR is not qualified")
    argv = [str(forge)]
    if cache_dir is not None:
        argv += ["--cache-dir", str(cache_dir)]
    argv += ["--params-dir", str(params), "prepare", str(source)]
    start = time.perf_counter()
    result = subprocess.run(argv, check=True, capture_output=True, text=True)
    wall = time.perf_counter() - start
    value = json.loads(result.stdout)
    paths = validate_export(value, source, qualification, params)
    return paths, {"prepare_wall_seconds": wall, "cache": value["cache"],
                   "artifact_set_id": value["artifact_set_id"],
                   "persistent_export": value["export"]["base"]}


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
    child_env = dict(os.environ, MIDNIGHT_PP=str(args.params_dir))
    packages = tomllib.loads((ROOT / "Cargo.lock").read_text())["package"]
    for name, version in CONSUMER.items():
        require({p["version"] for p in packages if p["name"] == name} == {version},
                f"unqualified consumer dependency: {name}")
    qualifications = json.loads(QUALIFICATIONS.read_text())
    report = {"scope": "two exact circuits; one sequential Python unittest controller",
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
                    for kind, path in paths.items():
                        self.assertEqual(digest(path), q[kind], kind)
                else:
                    paths, row["forge"] = prepare(source, q, args.params_dir,
                                                  forge=args.forge, cache_dir=args.cache_dir)
                    (output / "keys").mkdir(exist_ok=True)
                    for kind, directory in (("binary_ir", "zkir"), ("prover", "keys"),
                                            ("verifier", "keys")):
                        (output / directory / paths[kind].name).symlink_to(paths[kind])
                proof = run([args.proof_runner, q["selector"], output], "proof_and_assertions")
                self.assertIn(q["success_marker"], proof.stdout)
                # Dereference export links into PRIVATE files before mutation.
                negative = Path(temp) / "negative"
                shutil.copytree(output, negative, symlinks=False)
                (negative / "keys" / f"{circuit}.verifier").write_bytes(b"invalid verifier")
                run([args.proof_runner, q["selector"], negative], "corrupt_verifier_control",
                    expected_success=False)
                for kind, path in paths.items():
                    self.assertEqual(digest(path), q[kind], "artifact mutated by consumer")
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
