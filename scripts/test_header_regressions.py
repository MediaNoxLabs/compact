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

"""PR license scope controls; the original full-header validator remains strict."""
from pathlib import Path
import subprocess
import tempfile
import unittest

import check_header_regressions as gate


class HeaderRegressionTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        subprocess.run(['git', 'init', '-q', self.root], check=True)
        self.path = self.root / 'source.py'
        self.path.write_text('print("existing debt")\n')
        subprocess.run(['git', 'add', 'source.py'], cwd=self.root, check=True)
        subprocess.run(['git', '-c', 'user.name=Test', '-c', 'user.email=test@example.invalid',
                        '-c', 'commit.gpgsign=false', 'commit', '-qm', 'fixture'], cwd=self.root, check=True)
        self.base = subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=self.root, text=True).strip()

    def test_actual_base_content_is_inherited_only_when_identical(self):
        result = gate.classify(self.root, self.base, [self.path])
        self.assertTrue(result['passed'])
        self.assertEqual(len(result['inherited_unchanged']), 1)
        self.path.write_text('print("modified debt")\n')
        result = gate.classify(self.root, self.base, [self.path])
        self.assertFalse(result['passed'])
        self.assertEqual(len(result['new_or_modified_violations']), 1)

    def test_new_file_and_rename_without_header_are_violations(self):
        new = self.root / 'new.py'
        new.write_bytes(self.path.read_bytes())
        result = gate.classify(self.root, self.base, [new])
        self.assertFalse(result['passed'])
        self.assertEqual(gate.base_bytes(self.root, self.base, 'new.py'), None)

    def test_batch_handles_duplicate_blobs_and_unusual_filenames(self):
        unusual = self.root / 'spaces and\nnewline.py'
        unusual.write_bytes(self.path.read_bytes())
        subprocess.run(['git', 'add', '--all'], cwd=self.root, check=True)
        subprocess.run(['git', '-c', 'user.name=Test', '-c', 'user.email=test@example.invalid',
                        '-c', 'commit.gpgsign=false', 'commit', '-qm', 'fixture names'], cwd=self.root, check=True)
        base = subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=self.root, text=True).strip()
        values = gate.base_contents(self.root, base, ['source.py', unusual.name, 'missing.py'])
        self.assertEqual(values['source.py'], values[unusual.name])
        self.assertIsNone(values['missing.py'])
        self.assertTrue(gate.classify(self.root, base, [self.path, unusual])['passed'])

    def test_invalid_base_is_not_a_missing_file(self):
        with self.assertRaises(subprocess.CalledProcessError):
            gate.base_bytes(self.root, '0' * 40, 'source.py')

    def test_all_missing_files_accounted_without_timestamp_trust(self):
        another = self.root / 'another.py'
        another.write_text('new')
        result = gate.classify(self.root, self.base, [self.path, another])
        self.assertEqual(result['missing_full_headers'], 2)
        self.assertEqual(len(result['inherited_unchanged']) + len(result['new_or_modified_violations']), 2)
        self.assertIn('not full-tree', result['scope'])

    def test_full_header_policy_still_rejects_short_spdx(self):
        manager = gate.HeaderManager(str(gate.ROOT / 'header_config.json'), str(self.root))
        short = '# This file is part of Compact.\n# Copyright (C) 2026 Midnight Foundation\n# SPDX-License-Identifier: Apache-2.0\n'
        self.assertFalse(manager._scan_for_header('#', short))
        self.assertTrue(manager._scan_for_header('#', manager._format_license_header('#', '2026')))


if __name__ == '__main__':
    unittest.main()
