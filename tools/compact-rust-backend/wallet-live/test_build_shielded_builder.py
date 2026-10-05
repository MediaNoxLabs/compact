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

import importlib.util
import json
from pathlib import Path
import shutil
import tempfile
import unittest

spec = importlib.util.spec_from_file_location(
    'build_shielded_builder', Path(__file__).with_name('build-shielded-builder.py'))
builder = importlib.util.module_from_spec(spec)
spec.loader.exec_module(builder)


class RuntimeIdentity(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.reference = self.root / 'reference'
        (self.reference / 'src').mkdir(parents=True)
        (self.reference / 'Cargo.toml').write_text('[package]\nname="midnight-compact-runtime"\nversion="0.1.0"\n[dependencies]\nmidnight-compact-runtime-macros={path="../runtime-rs-macros"}\n')
        (self.reference / 'src/lib.rs').write_text('pub const ABI: u32 = 49;\n')
        self.generated = self.root / 'contract'
        self.generated.mkdir()
        self.bundled = self.generated / 'runtime-rs'
        shutil.copytree(self.reference, self.bundled)
        self.macros = self.root / 'runtime-rs-macros'
        (self.macros / 'src').mkdir(parents=True)
        (self.macros / 'Cargo.toml').write_text('[package]\nname="midnight-compact-runtime-macros"\nversion="0.1.0"\n')
        (self.macros / 'src/lib.rs').write_text('extern crate proc_macro;\n')
        shutil.copytree(self.macros, self.generated / 'runtime-rs-macros')

    def dependency(self, path):
        (self.generated / 'Cargo.toml').write_text(
            '[dependencies]\nmidnight-compact-runtime={path=' + json.dumps(str(path)) + '}\n')

    def test_portable_relative_and_explicit_absolute_share_generated_identity(self):
        for path, expected in [('runtime-rs', self.bundled), (self.reference, self.reference)]:
            self.dependency(path)
            actual, inventory = builder.resolve_generated_runtime(self.generated, self.reference)
            self.assertEqual(actual, expected.resolve())
            self.assertEqual(inventory, builder.runtime_dependency_inventory(self.reference))

    def test_changed_missing_or_extra_runtime_source_refused(self):
        self.dependency('runtime-rs')
        original = (self.bundled / 'src/lib.rs').read_text()
        (self.bundled / 'src/lib.rs').write_text('pub const ABI: u32 = 50;')
        with self.assertRaisesRegex(ValueError, 'differ'):
            builder.resolve_generated_runtime(self.generated, self.reference)
        (self.bundled / 'src/lib.rs').write_text(original)
        (self.bundled / 'src/extra.txt').write_text('an included resource')
        with self.assertRaisesRegex(ValueError, 'differ'):
            builder.resolve_generated_runtime(self.generated, self.reference)
        (self.bundled / 'src/extra.txt').unlink()
        (self.bundled / 'src/lib.rs').unlink()
        with self.assertRaisesRegex(ValueError, 'empty'):
            builder.resolve_generated_runtime(self.generated, self.reference)

    def test_macro_dependency_drift_refused(self):
        self.dependency('runtime-rs')
        (self.generated / 'runtime-rs-macros/src/lib.rs').write_text('changed macro implementation')
        with self.assertRaisesRegex(ValueError, 'differ'):
            builder.resolve_generated_runtime(self.generated, self.reference)

    def test_nonlocal_or_wrong_package_refused(self):
        (self.generated / 'Cargo.toml').write_text('[dependencies]\nmidnight-compact-runtime="0.1.0"\n')
        with self.assertRaisesRegex(ValueError, 'local runtime path'):
            builder.resolve_generated_runtime(self.generated, self.reference)
        self.dependency('runtime-rs')
        (self.bundled / 'Cargo.toml').write_text('[package]\nname="different-runtime"\n')
        with self.assertRaisesRegex(ValueError, 'package identity'):
            builder.resolve_generated_runtime(self.generated, self.reference)


if __name__ == '__main__':
    unittest.main()
