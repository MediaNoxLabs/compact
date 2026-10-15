# This file is part of Compact.
# Copyright (C) 2026 contributors to Minokawa Compact
# SPDX-License-Identifier: Apache-2.0

"""Fail-closed glue tests; actual compatibility is exercised by the opt-in runner."""

import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch

import forge_circuits as cache


class ForgeCacheBoundaryTests(unittest.TestCase):
    def setUp(self):
        temp = tempfile.TemporaryDirectory()
        self.addCleanup(temp.cleanup)
        self.root = Path(temp.name).resolve()
        self.source = self.root / "current input.zkir"
        self.source.write_bytes(b"current input")
        self.params = self.root / "params"
        self.params.mkdir()
        (self.params / "bls_midnight_2p6").write_bytes(b"parameters")
        self.base = self.root / "persistent export"
        self.base.mkdir()
        self.q = {"k": 6, "parameter_digest": cache.digest(self.params / "bls_midnight_2p6")}
        self.consumer_validator = cache.validate_consumer
        guard = patch.object(cache, "validate_consumer")
        guard.start()
        self.addCleanup(guard.stop)
        files = []
        for kind in ("source", "binary_ir", "prover", "verifier"):
            path = self.base / kind
            path.write_bytes(self.source.read_bytes() if kind == "source" else kind.encode())
            self.q[kind] = cache.digest(path)
            files.append({"kind": kind, "path": str(path), "sha256": self.q[kind],
                          "bytes": path.stat().st_size})
        self.value = {"schema": 1, "profile": cache.PROFILE, "build_fingerprint": cache.BUILD,
                      "ledger_revision": cache.REVISION, "bundle_id": "fixture-bundle",
                      "derivation_id": "fixture-recipe", "artifact_set_id": "fixture-set", "cache": {"hit": True},
                      "export": {"base": str(self.base), "lifetime": "persistent_until_owner_removal", "files": files}}
        self.manifest = {key: self.value[key] for key in ("profile", "build_fingerprint", "ledger_revision",
                         "bundle_id", "derivation_id", "artifact_set_id")}
        self.manifest["schema"] = 1
        self.manifest.update(source={"kind": "source", "digest": self.q["source"], "size": self.source.stat().st_size},
                             k=6, parameter_digest=self.q["parameter_digest"],
                             artifacts=[{"kind": row["kind"], "digest": row["sha256"], "size": row["bytes"]}
                                        for row in files if row["kind"] != "source"])
        (self.base / "manifest.json").write_text(json.dumps(self.manifest))

    def validate(self):
        return cache.validate_export(self.value, self.source, self.params)

    def test_absolute_argument_array_and_persistent_paths(self):
        result = subprocess.CompletedProcess([], 0, json.dumps(self.value), "")
        with patch.object(cache.subprocess, "run", return_value=result) as run:
            paths, receipt = cache.prepare(self.source, self.params,
                                            forge=Path("/configured/forge"),
                                            cache_dir=self.root / "private cache")
        self.assertEqual(run.call_args.args[0], ["/configured/forge", "--cache-dir",
                         str(self.root / "private cache"), "--params-dir", str(self.params),
                         "prepare", str(self.source.resolve()), "--name", "compact-" + self.q["source"][:24]])
        self.assertTrue(run.call_args.kwargs["check"])
        self.assertNotIn("shell", run.call_args.kwargs)
        self.assertEqual(paths["prover"], self.base / "prover")
        self.assertTrue(receipt["cache"]["hit"])

    def test_changed_current_input_refuses_stale_export_without_fallback(self):
        self.source.write_bytes(b"changed input")
        response = subprocess.CompletedProcess([], 0, json.dumps(self.value), "")
        with patch.object(cache.subprocess, "run", return_value=response) as run:
            with self.assertRaises(cache.CompatibilityError):
                cache.prepare(self.source, self.params)
        run.assert_called_once()

    def test_default_store_is_delegated_to_forge(self):
        result = subprocess.CompletedProcess([], 0, json.dumps(self.value), "")
        with patch.object(cache.subprocess, "run", return_value=result) as run:
            cache.prepare(self.source, self.params)
        self.assertNotIn("--cache-dir", run.call_args.args[0])

    def test_nonzero_failure_propagates_without_fallback(self):
        failure = subprocess.CalledProcessError(2, ["forge"], stderr="corrupt cache")
        with patch.object(cache.subprocess, "run", side_effect=failure):
            with self.assertRaises(subprocess.CalledProcessError) as caught:
                cache.prepare(self.source, self.params)
        self.assertIs(caught.exception, failure)

    def test_malformed_response_is_not_a_hit(self):
        result = subprocess.CompletedProcess([], 0, "not JSON", "")
        with patch.object(cache.subprocess, "run", return_value=result):
            with self.assertRaises(json.JSONDecodeError):
                cache.prepare(self.source, self.params)

    def test_export_corruption_and_missing_file_refuse(self):
        (self.base / "verifier").write_bytes(b"corrupt")
        with self.assertRaises(cache.CompatibilityError):
            self.validate()
        (self.base / "verifier").unlink()
        with self.assertRaises(FileNotFoundError):
            self.validate()

    def test_profile_and_parameter_drift_refuse(self):
        self.value["profile"] = "different profile"
        with self.assertRaises(cache.CompatibilityError):
            self.validate()
        self.value["profile"] = cache.PROFILE
        (self.params / "bls_midnight_2p6").write_bytes(b"different parameters")
        with self.assertRaises(cache.CompatibilityError):
            self.validate()

    def test_duplicate_and_missing_artifact_kinds_refuse(self):
        self.value["export"]["files"].append(self.value["export"]["files"][0])
        with self.assertRaises(cache.CompatibilityError):
            self.validate()
        self.value["export"]["files"] = self.value["export"]["files"][:3]
        with self.assertRaises(cache.CompatibilityError):
            self.validate()

    def test_relative_or_foreign_artifact_path_refuses(self):
        item = self.value["export"]["files"][0]
        for path in ("relative/source", str(self.source)):
            with self.subTest(path=path):
                item["path"] = path
                with self.assertRaises(cache.CompatibilityError):
                    self.validate()

    def test_input_change_during_prepare_refuses(self):
        def mutate(*args, **kwargs):
            self.source.write_bytes(b"changed during invocation")
            return subprocess.CompletedProcess([], 0, json.dumps(self.value), "")
        with patch.object(cache.subprocess, "run", side_effect=mutate):
            with self.assertRaises(cache.CompatibilityError):
                cache.prepare(self.source, self.params)


if __name__ == "__main__":
    unittest.main()
