# This file is part of Compact.
# Copyright (C) 2026 contributors to Minokawa Compact
# SPDX-License-Identifier: Apache-2.0
"""Public Forge descriptor contract; real proof qualification is a separate lap."""
from copy import deepcopy
import json
import unittest

import forge_circuits as cache
import test_forge_cache as legacy


class PublicForgeDescriptorTests(legacy.ForgeCacheBoundaryTests):
    """Run original boundary controls under the new descriptor-bearing build too."""

    def setUp(self):
        super().setUp()
        self.value['build_fingerprint'] = cache.PUBLIC_BUILD
        self.manifest['build_fingerprint'] = cache.PUBLIC_BUILD
        for key, char in [('bundle_id', 'a'), ('derivation_id', 'b'), ('artifact_set_id', 'c')]:
            self.value[key] = self.manifest[key] = char * 64
        self.manifest['name'] = 'unit-fixture'
        (self.base / 'manifest.json').write_text(json.dumps(self.manifest))
        self.value['descriptor'] = {
            'schema': 1, 'record': dict(deepcopy(self.manifest), generation_ms=None),
            'materialized': True, 'input_format': 'midnight-zkir-v2-json',
            'input_version': {'major': 2, 'minor': 0},
            'keygen_recipe': 'library-keygen/default-options/no-rng',
            'serialization': dict(cache.SERIALIZATION),
        }
        self.write_descriptor()

    def write_descriptor(self):
        (self.base / 'descriptor.json').write_text(json.dumps(self.value['descriptor']))

    def test_public_descriptor_accepts_matching_export(self):
        paths, manifest = self.validate()
        self.assertEqual(set(paths), cache.KINDS)
        self.assertEqual(manifest['build_fingerprint'], cache.PUBLIC_BUILD)

    def test_unknown_build_cannot_borrow_a_matching_descriptor(self):
        self.value['build_fingerprint'] = 'd' * 64
        with self.assertRaisesRegex(cache.CompatibilityError, 'unqualified Forge'):
            self.validate()

    def test_missing_descriptor_is_not_a_legacy_fallback(self):
        del self.value['descriptor']
        with self.assertRaisesRegex(cache.CompatibilityError, 'descriptor'):
            self.validate()

    def test_descriptor_policy_mutations_refuse_even_with_matching_sidecar(self):
        original = deepcopy(self.value['descriptor'])
        changes = [
            ('schema', 2), ('schema', True), ('materialized', False),
            ('input_format', 'midnight-zkir-v3-json'),
            ('input_version', {'major': 2, 'minor': 1}),
            ('input_version', {'major': 2, 'minor': False}),
            ('serialization', dict(cache.SERIALIZATION, verifier_key='verifier-key[v7]')),
            ('keygen_recipe', 'different-options'),
            ('record', dict(original['record'], parameter_digest='wrong-parameters')),
            ('record', dict(original['record'], generation_ms=0)),
        ]
        for field, value in changes:
            with self.subTest(field=field, value=value):
                self.value['descriptor'] = deepcopy(original)
                self.value['descriptor'][field] = value
                self.write_descriptor()
                with self.assertRaises(cache.CompatibilityError):
                    self.validate()

    def test_unknown_descriptor_field_refuses(self):
        self.value['descriptor']['unexpected'] = None
        self.write_descriptor()
        with self.assertRaisesRegex(cache.CompatibilityError, 'descriptor fields'):
            self.validate()

    def test_descriptor_sidecar_is_live_and_cannot_escape(self):
        sidecar = self.base / 'descriptor.json'
        original = sidecar.read_bytes()
        sidecar.write_bytes(b'{}')
        with self.assertRaisesRegex(cache.CompatibilityError, 'sidecar differs'):
            self.validate()
        sidecar.unlink()
        with self.assertRaises(FileNotFoundError):
            self.validate()
        outside = self.root / 'outside-descriptor'
        outside.write_bytes(original)
        sidecar.symlink_to(outside)
        with self.assertRaisesRegex(cache.CompatibilityError, 'descriptor symlink'):
            self.validate()

    def test_duplicate_descriptor_sidecar_fields_refuse(self):
        (self.base / 'descriptor.json').write_text('{"schema":1,"schema":1}')
        with self.assertRaisesRegex(cache.CompatibilityError, 'duplicate Forge JSON field'):
            self.validate()
