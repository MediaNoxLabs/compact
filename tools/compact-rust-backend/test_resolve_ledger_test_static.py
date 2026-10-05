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

import contextlib
import copy
import hashlib
import io
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import resolve_ledger_test_static as resolver


class LedgerStaticTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="ledger fixture test ")
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.crate = self.root / "registry package"
        dust = self.crate / "static/dust"
        dust.mkdir(parents=True)
        (self.crate / "Cargo.toml").write_text('[package]\nname = "midnight-ledger"\nversion = "8.0.3"\n')
        self.vcs = self.crate / ".cargo_vcs_info.json"
        self.vcs.write_text(json.dumps({"git": {"sha1": resolver.REVISION}, "path_in_vcs": "ledger"}))
        hashes = {}
        for name in resolver.FIXTURE_HASHES:
            data = ("test fixture " + name).encode()
            hashes[name] = hashlib.sha256(data).hexdigest()
            (dust / name).write_bytes(data)
            (dust / (name + ".sha256")).write_text(hashes[name] + "  " + name)
        (dust / "spend.prover.sha256").write_text(resolver.PROVER_HASH + "  spend.prover")
        # Unit fixtures exercise validation independently of the 8.0.3 bytes;
        # the separate real locked-package preflight checks the published pins.
        self.patch = patch.object(resolver, "FIXTURE_HASHES", hashes)
        self.patch.start()
        self.addCleanup(self.patch.stop)
        self.package = {"name": "midnight-ledger", "version": resolver.VERSION,
                        "source": resolver.SOURCE, "manifest_path": str(self.crate / "Cargo.toml")}
        self.metadata = {"packages": [self.package]}
        self.lock = {"package": [{"name": "midnight-ledger", "version": resolver.VERSION,
                                  "source": resolver.SOURCE, "checksum": resolver.CHECKSUM}]}

    def test_locked_package_and_space_path(self):
        result = resolver.resolve(self.metadata, self.lock)
        self.assertEqual(result["static_directory"], str((self.crate / "static").resolve()))
        self.assertEqual(result["vcs_revision"], resolver.REVISION)

    def test_missing_and_duplicate_packages(self):
        for packages in ([], [self.package, self.package]):
            with self.subTest(packages=len(packages)), self.assertRaisesRegex(ValueError, "one midnight-ledger"):
                resolver.resolve({"packages": packages}, self.lock)

    def test_wrong_version_source_and_lock_checksum(self):
        for key, value in (("version", "8.0.2"), ("source", "git+file:///different")):
            for target in ("metadata", "lock"):
                metadata, lock = copy.deepcopy(self.metadata), copy.deepcopy(self.lock)
                (metadata["packages"] if target == "metadata" else lock["package"])[0][key] = value
                with self.subTest(key=key, target=target), self.assertRaisesRegex(ValueError, "version/source"):
                    resolver.resolve(metadata, lock)
        self.lock["package"][0]["checksum"] = "0" * 64
        with self.assertRaisesRegex(ValueError, "checksum"):
            resolver.resolve(self.metadata, self.lock)

    def test_manifest_and_vcs_identity(self):
        (self.crate / "Cargo.toml").write_text('[package]\nname="other"\nversion="8.0.3"\n')
        with self.assertRaisesRegex(ValueError, "manifest identity"):
            resolver.resolve(self.metadata, self.lock)
        (self.crate / "Cargo.toml").write_text('[package]\nname="midnight-ledger"\nversion="8.0.3"\n')
        self.vcs.write_text(json.dumps({"git": {"sha1": "0" * 40}, "path_in_vcs": "ledger"}))
        with self.assertRaisesRegex(ValueError, "VCS identity"):
            resolver.resolve(self.metadata, self.lock)

    def test_corrupted_missing_and_false_digest_fixtures(self):
        path = self.crate / "static/dust/spend.bzkir"
        data = path.read_bytes()
        path.write_bytes(data + b"tampered")
        with self.assertRaisesRegex(ValueError, "digest mismatch"):
            resolver.resolve(self.metadata, self.lock)
        path.unlink()
        with self.assertRaises(FileNotFoundError):
            resolver.resolve(self.metadata, self.lock)
        path.write_bytes(data)
        (path.parent / "spend.bzkir.sha256").write_text("0" * 64)
        with self.assertRaisesRegex(ValueError, "declaration mismatch"):
            resolver.resolve(self.metadata, self.lock)

    def test_prover_digest_is_not_a_required_prover_file(self):
        self.assertFalse((self.crate / "static/dust/spend.prover").exists())
        resolver.resolve(self.metadata, self.lock)
        (self.crate / "static/dust/spend.prover.sha256").write_text("0" * 64)
        with self.assertRaisesRegex(ValueError, "prover digest"):
            resolver.resolve(self.metadata, self.lock)

    def test_environment_path_rejects_line_injection(self):
        original = self.crate
        for character in ("\r", "\n"):
            renamed = self.root / ("package" + character + "INJECTED=value")
            original.rename(renamed)
            self.package["manifest_path"] = str(renamed / "Cargo.toml")
            with self.subTest(character=repr(character)), self.assertRaisesRegex(ValueError, "unsafe environment path"):
                resolver.resolve(self.metadata, self.lock)
            renamed.rename(original)

    def test_cli_appends_valid_environment_and_provenance(self):
        metadata, lock = self.root / "metadata.json", self.root / "Cargo.lock"
        metadata.write_text(json.dumps(self.metadata))
        lock.write_text('[[package]]\n' + "".join(f'{key} = "{value}"\n' for key, value in self.lock["package"][0].items()))
        provenance, env = self.root / "provenance.json", self.root / "github-env"
        env.write_text("EXISTING=value\n")
        with contextlib.redirect_stdout(io.StringIO()):
            code = resolver.main(["--metadata", str(metadata), "--lock", str(lock),
                                  "--provenance", str(provenance), "--github-env", str(env)])
        self.assertEqual(code, 0)
        self.assertEqual(env.read_text(), f"EXISTING=value\nMIDNIGHT_LEDGER_TEST_STATIC_DIR={(self.crate / 'static').resolve()}\n")
        self.assertEqual(json.loads(provenance.read_text())["cargo_lock_sha256"],
                         hashlib.sha256(lock.read_bytes()).hexdigest())

    def test_cli_validation_failure_preserves_outputs(self):
        metadata, lock = self.root / "metadata.json", self.root / "Cargo.lock"
        metadata.write_text(json.dumps(self.metadata))
        lock.write_text('[[package]]\nname="midnight-ledger"\nversion="8.0.2"\nsource="wrong"\n')
        provenance, env = self.root / "provenance.json", self.root / "github-env"
        provenance.write_text("prior provenance")
        env.write_text("EXISTING=value\n")
        with contextlib.redirect_stderr(io.StringIO()):
            code = resolver.main(["--metadata", str(metadata), "--lock", str(lock),
                                  "--provenance", str(provenance), "--github-env", str(env)])
        self.assertEqual(code, 1)
        self.assertEqual(provenance.read_text(), "prior provenance")
        self.assertEqual(env.read_text(), "EXISTING=value\n")


if __name__ == "__main__":
    unittest.main()
