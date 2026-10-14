"""Focused laws for the existing approved two-epoch Rust provenance contract."""
import copy
import json
from pathlib import Path
import shutil
import tempfile
import unittest

import local_parity_gate as common
import reviewed_rust_capture as reviewed


class ReviewedCaptureTests(unittest.TestCase):
    fixture = 'tests-rust-backend/did-point-digest-reducer'

    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.package = self.root / self.fixture
        actual_package = common.ROOT / self.fixture
        self.binding = json.loads((actual_package/'oracle/reviewed-rust-binding.json').read_text())
        self.retained = json.loads((actual_package/'oracle/cases.json').read_text())
        paths = {reviewed.SUPPORT, f'{self.fixture}/oracle/cases.json',
                 f'{self.fixture}/oracle/reviewed-rust-binding.json'}
        paths.update(row['path'] for row in self.retained['provenance'])
        paths.update(row['path'] for row in self.binding['reviewed_artifacts'])
        for name in paths:
            destination = self.root/name
            destination.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(common.ROOT/name, destination)
        self.fresh = copy.deepcopy(self.retained)
        replacements = {row['path']: row['current_sha256'] for row in self.binding['replacements']}
        for row in self.fresh['provenance']:
            if row['path'] in replacements:
                row['sha256'] = replacements[row['path']]

    def compare(self, fresh=None):
        return reviewed.compare_reviewed_capture(self.fresh if fresh is None else fresh,
                                                  self.root, self.fixture)

    def test_exact_approved_replacements_pass_without_mutating_inputs(self):
        before = copy.deepcopy(self.fresh)
        result = self.compare()
        self.assertEqual(self.fresh, before)
        self.assertEqual(result['replaced_paths'], sorted(row['path'] for row in self.binding['replacements']))
        self.assertEqual(reviewed.digest(self.package/'oracle/cases.json'), self.binding['captures'][0]['sha256'])

    def test_behavior_gas_errors_and_scalar_type_changes_refuse(self):
        for change in ('case', 'gas', 'error', 'extra-case', 'scalar-type'):
            fresh = copy.deepcopy(self.fresh)
            if change == 'case': fresh['cases'][0]['id'] = 'unreviewed'
            elif change == 'gas': fresh['cases'][0]['queries'] = [{'invented':1}]
            elif change == 'error': fresh['cases'][0]['error'] = 'invented refusal'
            elif change == 'extra-case': fresh['cases'].append(copy.deepcopy(fresh['cases'][0]))
            else: fresh['cases'][0]['privateBefore'] = False
            with self.subTest(change=change), self.assertRaises(common.GateError):
                self.compare(fresh)

    def test_every_other_producer_field_is_exact(self):
        for field in ('generatedJavaScriptSha256','runtimeJavaScriptSha256','captureSha256','runtimeVersion','format','kind'):
            fresh = copy.deepcopy(self.fresh)
            fresh[field] = 'changed'
            with self.subTest(field=field), self.assertRaises(common.GateError): self.compare(fresh)
        fresh = copy.deepcopy(self.fresh); fresh['extraProducerField'] = 'invented'
        with self.assertRaises(common.GateError): self.compare(fresh)

    def test_immutable_capture_mutation_refuses(self):
        path = self.package/'oracle/cases.json'
        path.write_bytes(path.read_bytes()+b' ')
        with self.assertRaisesRegex(common.GateError,'immutable TS capture'): self.compare()

    def test_manifest_mutation_and_extra_replacement_refuse(self):
        path = self.package/'oracle/reviewed-rust-binding.json'
        binding = copy.deepcopy(self.binding)
        binding['replacements'].append({'path':'runtime/src/built-ins.ts','captured_sha256':'0'*64,'current_sha256':'0'*64})
        path.write_text(json.dumps(binding))
        with self.assertRaisesRegex(common.GateError,'manifest changed'): self.compare()

    def test_missing_duplicate_reordered_or_extra_fresh_provenance_refuse(self):
        for change in ('missing','duplicate','reordered','extra'):
            fresh = copy.deepcopy(self.fresh)
            if change == 'missing': fresh['provenance'].pop()
            elif change == 'duplicate': fresh['provenance'].append(copy.deepcopy(fresh['provenance'][0]))
            elif change == 'reordered': fresh['provenance'].reverse()
            else: fresh['provenance'].append({'path':'arbitrary.rs','sha256':'0'*64})
            with self.subTest(change=change), self.assertRaises(common.GateError): self.compare(fresh)

    def test_replacement_digests_and_path_range_are_exact(self):
        allowed = {row['path'] for row in self.binding['replacements']}
        for change in ('old-hash','invented-hash','path'):
            fresh = copy.deepcopy(self.fresh)
            row = next(row for row in fresh['provenance'] if row['path'] in allowed)
            if change == 'old-hash': row['sha256'] = next(v['captured_sha256'] for v in self.binding['replacements'] if v['path']==row['path'])
            elif change == 'invented-hash': row['sha256'] = '0'*64
            else: row['path'] = 'tests-rust-backend/another/lib.rs'
            with self.subTest(change=change), self.assertRaises(common.GateError): self.compare(fresh)

    def test_current_replaced_or_unchanged_artifact_drift_refuses(self):
        allowed = {row['path'] for row in self.binding['replacements']}
        for name in (next(iter(allowed)), next(row['path'] for row in self.retained['provenance'] if row['path'] not in allowed)):
            path = self.root/name; original = path.read_bytes()
            path.write_bytes(original+b'changed')
            with self.subTest(path=name), self.assertRaises(common.GateError): self.compare()
            path.write_bytes(original)

    def test_reviewed_support_and_full_artifact_list_are_authenticated(self):
        for name in [reviewed.SUPPORT, *(row['path'] for row in self.binding['reviewed_artifacts'])]:
            path = self.root/name; original = path.read_bytes(); path.write_bytes(original+b'changed')
            with self.subTest(path=name), self.assertRaises(common.GateError): self.compare()
            path.write_bytes(original)

    def test_safe_relative_paths_and_duplicate_json_keys_refuse(self):
        for name in ('/absolute','../outside','a/../b','a//b','a/./b','a\\b'):
            with self.subTest(path=name), self.assertRaises(common.GateError): reviewed.relative(name)
        path = self.root/'duplicate.json'; path.write_text('{"path":"a","path":"b"}')
        with self.assertRaises(common.GateError): reviewed.read_json(path)

    def test_registry_fixture_scope_matches_pinned_manifest_scope(self):
        registry = json.loads((common.ROOT/'tools/compact-rust-backend/did_primitive_reducers.json').read_text())
        self.assertEqual(set(reviewed.REVIEWED_BINDINGS), {row['fixture'] for row in registry['reducers']})
        for fixture, expected in reviewed.REVIEWED_BINDINGS.items():
            self.assertEqual(reviewed.digest(common.ROOT/fixture/'oracle/reviewed-rust-binding.json'), expected)

    def test_replacement_only_provenance_identity_passes_and_missing_file_refuses(self):
        other = ReviewedCaptureTests('test_exact_approved_replacements_pass_without_mutating_inputs')
        other.fixture = 'tests-rust-backend/did-alias-guard-reducer'
        try:
            other.setUp()
            provenance = f'{other.fixture}/tests/provenance.rs'
            self.assertNotIn(provenance, {row['path'] for row in other.binding['reviewed_artifacts']})
            self.assertIn(provenance, {row['path'] for row in other.binding['replacements']})
            other.compare()
            (other.root/provenance).unlink()
            with self.assertRaises(FileNotFoundError): other.compare()
        finally:
            other.doCleanups()


if __name__ == '__main__':
    unittest.main()
