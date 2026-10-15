# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
"""Source-role and consumer-origin controls; no compiler or Cargo execution."""
import copy
from collections import Counter
import hashlib
import json
from pathlib import Path
import tempfile
import tomllib
import unittest
from unittest import mock

import fixture_inventory as inventory


def fingerprint(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


class CurrentSourceRegistryTests(unittest.TestCase):
    def test_generated_wrappers_are_consumers_and_special_targets_stay_exact(self):
        registrations = inventory.source_registry()
        self.assertEqual(Counter(row.role for row in registrations), {
            inventory.SourceRole.FLAT_LIBRARY: 169,
            inventory.SourceRole.GENERATED_CONSUMER: 86,
            inventory.SourceRole.COMPILER_PROBE: 2,
        })
        self.assertEqual(sum(row.build_script is not None for row in registrations), 87)
        rows = {row.source: row for row in registrations}
        flat_sources = inventory.fixture_map()
        for source in ('examples/adt/tests/set_enum.compact',
                       'examples/adt/tests/set_qualified_coin_info.compact',
                       'test-center/test-contracts/bboard.compact',
                       'test-center/test-contracts/coracle.compact',
                       'test-center/test-contracts/micro-dao.compact'):
            row = rows[inventory.ROOT / source]
            with self.subTest(source=source):
                self.assertEqual(row.role, inventory.SourceRole.GENERATED_CONSUMER)
                self.assertIsNone(row.flat_fixture)
                self.assertEqual(row.library, inventory.extra_sources()[row.source])
                self.assertEqual(row.consumer, row.library)
                self.assertEqual(row.generated_artifact, 'contract.rs')
                self.assertTrue(row.retained_ir.is_file())
                self.assertNotIn(row.source, flat_sources)
        call = rows[inventory.ROOT / 'examples/rust_backend/call_arg_declared_type.compact']
        self.assertEqual(call.role, inventory.SourceRole.GENERATED_CONSUMER)
        self.assertIn(inventory.ROOT / 'tests-rust-backend/call-arg-declared-type/build/product.rs',
                      {path for path, _ in call.input_fingerprints})
        self.assertIn('synthetic product.rs', call.qualification)

    def test_missing_flat_destinations_are_real_consumers_or_accepted_probes(self):
        rows = {row.source.name: row for row in inventory.source_registry()}
        hashes = rows['effect_plan_aligned_hashes.compact']
        self.assertEqual(hashes.role, inventory.SourceRole.GENERATED_CONSUMER)
        self.assertEqual(hashes.package, 'compact-rust-effect-plan-aligned-hashes-source-fixture')
        self.assertEqual(hashes.library.name, 'fixture.rs')
        self.assertEqual(hashes.generated_artifact, 'contract.rs')
        self.assertIsNone(hashes.flat_fixture)
        self.assertTrue(hashes.retained_ir.is_file())
        keccak = rows['effect_plan_unit_keccak.compact']
        self.assertEqual(keccak.frontend_flags, ('--feature-zkir-v3',))
        for name in ('recorded_tuple12_result.compact', 'recorded_tuple13_result.compact'):
            probe = rows[name]
            self.assertEqual(probe.role, inventory.SourceRole.COMPILER_PROBE)
            self.assertIsNone(probe.manifest)
            self.assertIsNone(probe.library)
            self.assertIsNone(probe.flat_fixture)
            self.assertEqual(len(probe.compiler_tests), 2)
            self.assertIn('not a frontend-negative', probe.qualification)
        flat = rows['effectful_return_oracle.compact']
        self.assertEqual(flat.library.name, 'fixture.rs')
        self.assertEqual(flat.flat_fixture.name, 'lib.rs')
        self.assertNotEqual(flat.library, flat.flat_fixture)
        self.assertEqual(flat.generated_artifact, 'contract.rs')
        self.assertEqual(flat.consumer, flat.library)
        self.assertTrue(flat.build_script.is_file())
        self.assertTrue(flat.retained_ir.is_file())
        self.assertNotIn(hashes.source, inventory.fixture_map())
        self.assertEqual(inventory.fixture_map()[flat.source], flat.flat_fixture)

    def test_secondary_outputs_preserve_their_actual_host_placement(self):
        rows = {row.source.name: row for row in inventory.source_registry()}
        merkle = rows['effect_plan_merkle_nested.compact']
        self.assertEqual((merkle.generated_artifact, merkle.namespace), ('nested.rs', 'nested'))
        self.assertEqual(merkle.consumer.name, 'fixture.rs')
        opaque = rows['opaque_observed_nested.compact']
        self.assertEqual((opaque.generated_artifact, opaque.namespace), ('nested.rs', None))
        self.assertEqual(opaque.consumer.parts[-2:], ('tests', 'nested.rs'))
        constructor = rows['opaque_constructor_order.compact']
        self.assertEqual(constructor.generated_artifact, 'constructor_order.rs')
        self.assertEqual(constructor.consumer.parts[-2:], ('tests', 'constructor_order.rs'))


class Ledger813RegistryTransitionTests(unittest.TestCase):
    """The finite reviewed manifest migration cannot authorize other inputs."""

    def setUp(self):
        directory = inventory.ROOT / 'tools/compact-rust-backend'
        previous = directory / 'fixture_registry.ledger-8.0.3.json'
        current = directory / 'fixture_registry.json'
        transition = directory / 'fixture_registry.ledger-8.1.3-transition.json'
        self.assertEqual(fingerprint(previous),
                         '690e370bc4729f5d39918a1086e956e3fd5663a76f443f7fc7282d8385409c08')
        self.assertEqual(fingerprint(current),
                         '912b0ec66f313869b0df061233974316ee4b4069682a1be805ff5e6b6e194e35')
        self.assertEqual(fingerprint(transition),
                         '32b985bffdfaade21f57dfd36d5ba688e8a4a0fa8d0f0afb845f97fcb922791e')
        self.previous = json.loads(previous.read_text())
        self.current = json.loads(current.read_text())
        self.transition = json.loads(transition.read_text())
        self.assertEqual(self.transition['predecessor']['sha256'], fingerprint(previous))
        self.assertEqual(self.transition['successor']['sha256'], fingerprint(current))
        self.assertEqual(self.transition['reviewed_upgrade_commit'],
                         '535e4470e44cbed57e0cfe8db27a665d12c2093f')
        self.assertEqual(self.transition['pre_upgrade_commit'],
                         'f8450e32b4eabd462df42cbae024b12d690f0eea')

    def test_only_the_exact_73_manifest_pins_change(self):
        changes = {row['path']: row for row in self.transition['manifest_changes']}
        self.assertEqual(len(changes), 73)
        self.assertEqual(len(self.transition['manifest_changes']), len(changes))
        self.assertEqual(list(changes), sorted(changes))
        expected = copy.deepcopy(self.previous)
        affected = []
        for row in expected['sources']:
            binding = row.get('build_binding', row)
            for path, pin in binding.get('consumer_input_sha256', {}).items():
                if path not in changes:
                    continue
                change = changes[path]
                self.assertEqual(pin, change['before_sha256'])
                self.assertTrue(path.startswith('tests-rust-backend/'))
                self.assertEqual(Path(path).name, 'Cargo.toml')
                self.assertEqual(fingerprint(inventory.ROOT / path), change['after_sha256'])
                binding['consumer_input_sha256'][path] = change['after_sha256']
                affected.append(path)
        self.assertEqual(len(affected), 73)
        self.assertEqual(set(affected), set(changes))
        # Every source/IR pin, role, position and unrelated input stays exact.
        self.assertEqual(expected, self.current)
        self.assertEqual(len(self.current['sources']), 257)

    def test_historical_registry_cannot_authorize_the_current_manifest_cohort(self):
        historical = inventory.ROOT / 'tools/compact-rust-backend/fixture_registry.ledger-8.0.3.json'
        with self.assertRaisesRegex(ValueError, 'registry source binding changed'):
            inventory.source_registry(registry_path=historical)

    def test_each_manifest_has_only_the_official_ledger_and_msrv_delta(self):
        versions = {
            'midnight-onchain-state': ('=3.0.0', '=3.0.2'),
            'midnight-serialize': ('=1.0.0', '=1.1.1'),
            'midnight-storage': ('=2.0.1', '=2.0.3'),
            'midnight-base-crypto': ('=1.0.0', '=1.0.2'),
            'midnight-coin-structure': ('=2.0.1', '=2.0.2'),
            'midnight-transient-crypto': ('=2.0.1', '=2.1.1'),
            'midnight-zswap': ('=8.0.3', '=8.1.3'),
        }
        official = {'git': 'https://github.com/midnightntwrk/midnight-ledger',
                    'rev': 'b5d3e965a18e4cac17fac4f38446fc55c524439a'}
        self.assertEqual(self.transition['official_ledger'], official)
        counts = Counter()
        for change in self.transition['manifest_changes']:
            with self.subTest(manifest=change['path']):
                before = change['before_toml'].encode()
                self.assertEqual(hashlib.sha256(before).hexdigest(), change['before_sha256'])
                expected = tomllib.loads(before.decode())
                touched = []
                if expected['package'].get('rust-version') == '1.88':
                    expected['package']['rust-version'] = '1.89'
                    touched.append('minimum-rust-version')
                    counts['minimum_rust_version_manifests'] += 1
                dependencies = expected.get('dev-dependencies', {})
                for package, (old, new) in versions.items():
                    if package in dependencies:
                        self.assertEqual(dependencies[package], old)
                        dependencies[package] = {'version': new, **official}
                        touched.append(package)
                        counts[package] += 1
                self.assertTrue(touched)
                self.assertCountEqual(touched, change['reviewed_changes'])
                if any(package in versions for package in touched):
                    counts['dependency_manifests'] += 1
                current = inventory.ROOT / change['path']
                self.assertEqual(fingerprint(current), change['after_sha256'])
                # Whole-document equality excludes unrelated dependency,
                # feature, package, source, path or workspace changes.
                self.assertEqual(tomllib.loads(current.read_text()), expected)
        self.assertEqual(counts['minimum_rust_version_manifests'], 12)
        self.assertEqual(counts['dependency_manifests'], 66)
        self.assertEqual(sum(counts[name] for name in versions), 210)
        self.assertEqual(self.transition['counts']['dependency_entries_by_package'],
                         {name: counts[name] for name in versions})


class SourceRegistryRefusalTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name).resolve()
        self.registry = self.root / 'registry.json'
        self.enterContext(mock.patch.object(inventory, 'SPECIAL_FIXTURES', {}))
        self.source = self.write('examples/rust_backend/example.compact', 'export pure circuit x(): Boolean { return true; }\n')
        self.ir = self.write('tests/example-ir.json', '{"schema_version":20}\n')
        self.manifest = self.write('consumer/Cargo.toml', '[package]\nname="fixture"\nversion="0.1.0"\n[lib]\npath="fixture.rs"\n')
        self.library = self.write('consumer/fixture.rs', 'include!(concat!(env!("OUT_DIR"), "/contract.rs"));\n')
        self.build = self.write('consumer/build.rs', '// reads retained IR and writes contract.rs\n')
        self.row = {
            'source': self.relative(self.source), 'source_sha256': fingerprint(self.source),
            'role': 'generated_consumer', 'frontend_outcome': 'accepted', 'frontend_flags': [],
            'qualification': 'Reviewed positive source consumer, not proof evidence.',
            'retained_ir': self.relative(self.ir), 'retained_ir_sha256': fingerprint(self.ir),
            'manifest': self.relative(self.manifest), 'library': self.relative(self.library),
            'build_script': self.relative(self.build), 'generated_artifact': 'contract.rs',
            'consumer': self.relative(self.library), 'namespace': None,
            'consumer_input_sha256': {
                self.relative(path): fingerprint(path)
                for path in (self.manifest, self.library, self.build)},
        }

    def write(self, relative, source):
        path = self.root / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(source)
        return path

    def relative(self, path):
        return str(path.relative_to(self.root))

    def registrations(self, rows=None):
        self.registry.write_text(json.dumps({'schema_version': 1, 'sources': rows or [self.row]}))
        return inventory.source_registry(self.root, registry_path=self.registry)

    def test_valid_consumer_keeps_origin_and_does_not_invent_a_flat_file(self):
        row, = self.registrations()
        self.assertEqual(row.retained_ir, self.ir)
        self.assertEqual(row.consumer, self.library)
        self.assertEqual(row.package, 'fixture')
        self.assertIsNone(row.flat_fixture)
        self.assertFalse((self.root / 'tests-rust-backend/example/lib.rs').exists())
        self.assertEqual({path for path, _ in row.input_fingerprints}, {
            self.source, self.ir, self.manifest, self.library, self.build})

    def test_manifest_bytes_remain_live_even_when_toml_semantics_are_unchanged(self):
        self.registrations()
        original = self.manifest.read_bytes()
        self.manifest.write_bytes(original + b'\n# unreviewed manifest change\n')
        with self.assertRaisesRegex(ValueError, 'registry source binding changed'):
            self.registrations()
        self.manifest.write_bytes(original)
        self.assertEqual(len(self.registrations()), 1)

    def test_flat_golden_cannot_hide_a_different_cargo_entry(self):
        golden = self.write('consumer/lib.rs', '// generated flat golden\n')
        row = {key: self.row[key] for key in ('source', 'source_sha256', 'role',
               'frontend_outcome', 'frontend_flags', 'qualification', 'manifest', 'library')}
        row.update(role='flat_library', generated_library=self.relative(golden))
        with self.assertRaisesRegex(ValueError, 'without a build binding'):
            self.registrations([row])
        row['build_binding'] = {key: self.row[key] for key in ('retained_ir',
            'retained_ir_sha256', 'build_script', 'generated_artifact', 'consumer',
            'namespace', 'consumer_input_sha256')}
        registration, = self.registrations([row])
        self.assertEqual(registration.flat_fixture, golden)
        self.assertEqual(registration.consumer, self.library)
        self.build.write_text('// drift\n')
        with self.assertRaisesRegex(ValueError, 'binding changed'):
            self.registrations([row])

    def test_out_dir_wrapper_cannot_be_registered_as_a_flat_golden(self):
        row = {key: self.row[key] for key in ('source', 'source_sha256', 'role',
               'frontend_outcome', 'frontend_flags', 'qualification', 'manifest', 'library')}
        row.update(role='flat_library', generated_library=self.relative(self.library))
        with self.assertRaisesRegex(ValueError, 'OUT_DIR wrapper is not a flat golden'):
            self.registrations([row])

    def test_special_source_accepts_actual_generated_library_and_rejects_substitution(self):
        special = self.write('test-center/test-contracts/example.compact', self.source.read_text())
        self.source.unlink()
        row = copy.deepcopy(self.row)
        row.update(source=self.relative(special), source_sha256=fingerprint(special))
        targets = {self.relative(special): self.relative(self.library)}
        with mock.patch.object(inventory, 'SPECIAL_FIXTURES', targets):
            registration, = self.registrations([row])
            self.assertEqual(registration.library, self.library)
            self.assertIsNone(registration.flat_fixture)
            self.assertEqual(inventory.extra_sources(self.root), {special: self.library})
            test = self.write('tests/probe.rs', '// probe cannot replace the consumer\n')
            keys = {'source', 'source_sha256', 'frontend_outcome', 'frontend_flags',
                    'qualification', 'retained_ir', 'retained_ir_sha256'}
            probe = {key: value for key, value in row.items() if key in keys}
            probe.update(role='compiler_probe', compiler_tests=[self.relative(test)],
                         compiler_test_sha256={self.relative(test): fingerprint(test)})
            with self.assertRaisesRegex(ValueError, 'special fixture consumer target changed'):
                self.registrations([probe])
        wrong = self.write('consumer/unrelated.rs', '// unrelated output\n')
        with mock.patch.object(inventory, 'SPECIAL_FIXTURES',
                               {self.relative(special): self.relative(wrong)}):
            with self.assertRaisesRegex(ValueError, 'special fixture consumer target changed'):
                self.registrations([row])

    def test_supplemental_build_input_is_pinned_and_drift_refuses(self):
        helper = self.write('consumer/build/helper.rs', '// package build helper\n')
        row = copy.deepcopy(self.row)
        row['supplemental_build_inputs'] = [self.relative(helper)]
        row['consumer_input_sha256'][self.relative(helper)] = fingerprint(helper)
        registration, = self.registrations([row])
        self.assertIn(helper, {path for path, _ in registration.input_fingerprints})
        helper.write_text('// changed helper\n')
        with self.assertRaisesRegex(ValueError, 'registry source binding changed'):
            self.registrations([row])

    def test_supplemental_authority_requires_both_declaration_and_exact_pin_set(self):
        helper = self.write('consumer/build/helper.rs', '// package build helper\n')
        undeclared = copy.deepcopy(self.row)
        undeclared['consumer_input_sha256'][self.relative(helper)] = fingerprint(helper)
        with self.assertRaisesRegex(ValueError, 'incomplete consumer input authority'):
            self.registrations([undeclared])
        unpinned = copy.deepcopy(self.row)
        unpinned['supplemental_build_inputs'] = [self.relative(helper)]
        with self.assertRaisesRegex(ValueError, 'incomplete consumer input authority'):
            self.registrations([unpinned])
        for paths, diagnostic in (([], 'invalid supplemental'),
                                  ([self.relative(helper)] * 2, 'invalid supplemental'),
                                  ([self.relative(self.library)], 'duplicate supplemental')):
            row = copy.deepcopy(self.row)
            row['supplemental_build_inputs'] = paths
            with self.subTest(paths=paths), self.assertRaisesRegex(ValueError, diagnostic):
                self.registrations([row])

    def test_new_source_requires_registration_even_if_a_flat_file_exists(self):
        self.write('examples/rust_backend/unreviewed.compact', 'new source\n')
        self.write('tests-rust-backend/unreviewed/lib.rs', '// plausible output\n')
        with self.assertRaisesRegex(ValueError, 'unclassified/missing'):
            self.registrations()

    def test_duplicate_source_or_artifact_roles_refuse(self):
        with self.assertRaisesRegex(ValueError, 'duplicate or invalid'):
            self.registrations([self.row, copy.deepcopy(self.row)])
        other = copy.deepcopy(self.row)
        source = self.write('examples/rust_backend/other.compact', 'other source\n')
        other.update(source=self.relative(source), source_sha256=fingerprint(source))
        with self.assertRaisesRegex(ValueError, 'duplicate generated consumer artifact'):
            self.registrations([self.row, other])

    def test_missing_library_is_not_reclassified_as_a_probe(self):
        self.library.unlink()
        with self.assertRaisesRegex(ValueError, 'missing registry input'):
            self.registrations()

    def test_unreviewed_negative_or_unknown_role_cannot_hide_a_positive_source(self):
        for key, value, diagnostic in (
                ('role', 'frontend_negative', 'unreviewed source role'),
                ('frontend_outcome', 'refused', 'only reviewed accepted'),
                ('frontend_flags', ['--skip-typecheck'], 'unreviewed frontend flags')):
            row = copy.deepcopy(self.row)
            row[key] = value
            with self.subTest(key=key), self.assertRaisesRegex(ValueError, diagnostic):
                self.registrations([row])

    def test_source_ir_and_consumer_drift_are_separate_binding_failures(self):
        for path in (self.source, self.ir, self.build):
            original = path.read_bytes()
            path.write_bytes(original + b'\n')
            with self.subTest(path=path), self.assertRaisesRegex(ValueError, 'source binding changed'):
                self.registrations()
            path.write_bytes(original)

    def test_partial_consumer_authority_and_wrong_manifest_library_refuse(self):
        row = copy.deepcopy(self.row)
        row['consumer_input_sha256'].pop(self.relative(self.build))
        with self.assertRaisesRegex(ValueError, 'incomplete consumer input authority'):
            self.registrations([row])
        self.manifest.write_text(self.manifest.read_text().replace('fixture.rs', 'another.rs'))
        row = copy.deepcopy(self.row)
        row['consumer_input_sha256'][self.relative(self.manifest)] = fingerprint(self.manifest)
        with self.assertRaisesRegex(ValueError, 'disagrees with Cargo manifest'):
            self.registrations([row])

    def test_compiler_probe_requires_test_authority_without_a_fake_consumer(self):
        test = self.write('tests/compiler_probe.rs', '// retained source acceptance law\n')
        keys = {'source', 'source_sha256', 'frontend_outcome', 'frontend_flags',
                'qualification', 'retained_ir', 'retained_ir_sha256'}
        row = {key: value for key, value in self.row.items() if key in keys}
        row.update(role='compiler_probe', compiler_tests=[self.relative(test)],
                   compiler_test_sha256={self.relative(test): fingerprint(test)})
        probe, = self.registrations([row])
        self.assertEqual(probe.compiler_tests, (test,))
        self.assertIsNone(probe.library)
        self.assertIsNone(probe.manifest)
        self.assertIsNone(probe.flat_fixture)
        row['compiler_test_sha256'].clear()
        with self.assertRaisesRegex(ValueError, 'compiler-probe authority required'):
            self.registrations([row])

    def test_artifact_namespace_and_input_paths_cannot_escape(self):
        for key, value, diagnostic in (
                ('generated_artifact', '../other.rs', 'invalid OUT_DIR artifact'),
                ('namespace', 'nested::extra', 'invalid generated namespace'),
                ('retained_ir', '../outside.json', 'unsafe registry path')):
            row = copy.deepcopy(self.row)
            row[key] = value
            with self.subTest(key=key), self.assertRaisesRegex(ValueError, diagnostic):
                self.registrations([row])
        outside = self.root.parent / (self.root.name + '-external.json')
        outside.write_text('{"schema_version":20}')
        self.addCleanup(outside.unlink)
        self.ir.unlink()
        self.ir.symlink_to(outside)
        with self.assertRaisesRegex(ValueError, 'escapes root'):
            self.registrations()


if __name__ == '__main__':
    unittest.main()
