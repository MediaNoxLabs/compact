# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0

"""Scoped test-launcher, matching-profile and materialization boundaries."""
import json
import os
from pathlib import Path
import subprocess
import shutil
import sys
import unittest
from unittest.mock import patch

import forge_circuits as cache
import test_forge_cache as fixtures


class ForgeHarnessTests(unittest.TestCase):
    setUp = fixtures.ForgeCacheBoundaryTests.setUp
    validate = fixtures.ForgeCacheBoundaryTests.validate

    def test_new_current_circuit_is_not_limited_to_old_two_fixtures(self):
        self.source.write_bytes(b'new current circuit')
        (self.base / 'source').write_bytes(self.source.read_bytes())
        pin = cache.digest(self.source)
        self.value['export']['files'][0].update(sha256=pin, bytes=self.source.stat().st_size)
        self.manifest['source'].update(digest=pin, size=self.source.stat().st_size)
        (self.base / 'manifest.json').write_text(json.dumps(self.manifest))
        paths, _ = self.validate()
        self.assertEqual(paths['source'].read_bytes(), b'new current circuit')

    def test_exact_shared_git_and_registry_consumer_graph_with_optional_parser(self):
        lock = self.root / 'Cargo.lock'
        rows = [(name, version, cache.SOURCE if name in cache.GIT_PACKAGES else cache.REGISTRY)
                for name, version in cache.CONSUMER.items() if name != 'midnight-zkir']
        text = ''.join(f'[[package]]\nname="{n}"\nversion="{v}"\nsource="{s}"\n' for n, v, s in rows)
        lock.write_text(text)
        self.consumer_validator(lock)
        for wrong in (text.replace('8.1.3', '8.0.3'), text.replace(cache.SOURCE, cache.REGISTRY),
                      text.replace('version="0.7.1"', 'version="0.7.0"')):
            lock.write_text(wrong)
            with self.assertRaises(cache.CompatibilityError):
                self.consumer_validator(lock)

    def test_manifest_identity_and_duplicate_json_refuse(self):
        self.manifest['bundle_id'] = 'different'
        (self.base / 'manifest.json').write_text(json.dumps(self.manifest))
        with self.assertRaises(cache.CompatibilityError):
            self.validate()
        with self.assertRaises(cache.CompatibilityError):
            cache.read_json('{"profile":1,"profile":2}')

    def test_export_symlink_refuses_before_consumption(self):
        path = self.base / 'prover'
        path.unlink()
        path.symlink_to(self.base / 'verifier')
        with self.assertRaises(cache.CompatibilityError):
            self.validate()

    def test_context_restores_environment_and_routes_snapshot_compiles_only(self):
        actual = self.root / 'actual-zkir'
        actual.write_text('tool')
        env = cache.configure_environment({'PATH': '/unchanged', 'MIDNIGHT_ZKIR_FORGE_DIR': '/cache'},
                                          self.root / 'harness', real_zkir=actual)
        self.assertEqual(env['COMPACT_ZKIR_REAL'], str(actual))
        self.assertEqual(env['MIDNIGHT_ZKIR_FORGE_DIR'], '/cache')
        self.assertEqual(cache.command(['/snapshot/zkir', 'compile', 'a', 'b', 'c'], env)[0], env['COMPACT_ZKIR_SHIM'])
        self.assertEqual(cache.command(['zkir-v3', 'compile', 'a', 'b', 'c'], env)[0], 'zkir-v3')
        self.assertEqual(cache.command(['/snapshot/zkir', '--version'], env)[0], '/snapshot/zkir')
        before = dict(os.environ)
        with patch.object(cache, 'configure_environment', return_value=dict(before, COMPACT_TEST_SENTINEL='scope')):
            with self.assertRaisesRegex(RuntimeError, 'exit'):
                with cache.test_environment(self.root / 'unused'):
                    self.assertEqual(os.environ['COMPACT_TEST_SENTINEL'], 'scope')
                    raise RuntimeError('exit')
        self.assertEqual(dict(os.environ), before)

    def test_uncached_mode_leaves_original_keygen_argv_unchanged(self):
        env = {'COMPACT_ZKIR_MODE': 'uncached', 'PATH': '/unchanged'}
        self.assertEqual(cache.configure_environment(env, self.root / 'not-created'), env)
        self.assertFalse((self.root / 'not-created').exists())
        argv = ['/snapshot/zkir', 'compile', 'a', 'b', 'c']
        self.assertEqual(cache.command(argv, env), argv)
        with self.assertRaises(cache.CompatibilityError):
            cache.configure_environment({'COMPACT_ZKIR_MODE': 'fallback'}, self.root / 'bad')

    def test_nested_harness_selects_actual_tool_not_outer_shim(self):
        env = {'COMPACT_ZKIR_REAL': '/real/zkir', 'PATH': '/outer/shim'}
        with patch.object(cache.shutil, 'which') as which:
            self.assertEqual(cache.zkir_executable(env), '/real/zkir')
        which.assert_not_called()

    def test_generated_launcher_really_forwards_non_keygen_cli(self):
        actual = self.root / 'actual-zkir'
        actual.write_text(f'#!{sys.executable}\nimport sys\nprint("actual " + " ".join(sys.argv[1:]))\n')
        actual.chmod(0o700)
        env = cache.configure_environment(dict(os.environ, COMPACT_ZKIR_MODE='forge'), self.root / 'harness', real_zkir=actual)
        result = subprocess.run([env['COMPACT_ZKIR_SHIM'], '--version'], env=env, check=True, capture_output=True, text=True)
        self.assertEqual(result.stdout.strip(), 'actual --version')

    def test_actual_bundle_launcher_preserves_scoped_shim_and_uncached_bundle_priority(self):
        bundle = self.root / 'bundle with spaces'
        binary, library = bundle / 'bin', bundle / 'lib'
        binary.mkdir(parents=True)
        library.mkdir()
        launcher = binary / 'compactc'
        shutil.copyfile(Path(cache.__file__).with_name('compactc-launcher.sh'), launcher)
        launcher.chmod(0o700)
        frontend = binary / 'compactc.bin'
        frontend.write_text('#!/usr/bin/env bash\nexec zkir "$@"\n')
        frontend.chmod(0o700)
        bundled = library / 'zkir'
        bundled.write_text('#!/usr/bin/env bash\nprintf "bundled %s\\n" "$*"\n')
        bundled.chmod(0o700)
        actual = self.root / 'actual-zkir'
        actual.write_text('#!/usr/bin/env bash\nprintf "scoped %s\\n" "$*"\n')
        actual.chmod(0o700)
        env = cache.configure_environment(dict(os.environ, COMPACT_ZKIR_MODE='forge'), self.root / 'harness', real_zkir=actual)
        cached = subprocess.run([str(launcher), '--version'], env=env, capture_output=True, text=True, check=True)
        self.assertEqual(cached.stdout.strip(), 'scoped --version')
        uncached = subprocess.run([str(launcher), '--version'], env=dict(env, COMPACT_ZKIR_MODE='uncached'),
                                  capture_output=True, text=True, check=True)
        self.assertEqual(uncached.stdout.strip(), 'bundled --version')
        refused = subprocess.run([str(launcher), '--version'], env=dict(env, COMPACT_ZKIR_SHIM='relative/zkir'),
                                 capture_output=True, text=True)
        self.assertEqual(refused.returncode, 2)
        self.assertIn('absolute zkir executable', refused.stderr)

    def test_many_uses_current_json_once_and_refuses_binary_only(self):
        sources = self.root / 'circuits'
        sources.mkdir()
        for name in ('b', 'a'):
            (sources / f'{name}.zkir').write_text('current')
        (sources / 'a.bzkir').write_text('older')
        with patch.dict(os.environ, COMPACT_ZKIR_REAL='/actual/zkir', COMPACT_ZKIR_MODE='forge'), patch.object(cache, 'compile_circuit') as compile:
            self.assertEqual(cache._shim(['compile-many', str(sources), str(self.root / 'keys')]), 0)
            self.assertEqual([row.args[0].name for row in compile.call_args_list], ['a.zkir', 'b.zkir'])
            (sources / 'c.bzkir').write_text('stale')
            with self.assertRaises(cache.CompatibilityError):
                cache._shim(['compile-many', str(sources), str(self.root / 'keys')])

    def test_empty_many_keeps_original_directory_semantics_without_preparation(self):
        sources = self.root / 'empty'
        sources.mkdir()
        with patch.dict(os.environ, COMPACT_ZKIR_REAL='/actual/zkir', COMPACT_ZKIR_MODE='forge'), patch.object(cache, 'compile_circuit') as compile:
            cache._shim(['compile-many', str(sources), str(self.root / 'keys')])
        self.assertTrue((self.root / 'keys').is_dir())
        compile.assert_not_called()

    def material(self):
        paths, _ = self.validate()
        return paths, {'persistent_export': str(self.base), 'artifacts': self.value['export']['files'],
                       'cache': {'hit': True}, 'k': 6, 'source_sha256': cache.digest(self.source)}

    def test_fresh_model_and_serialization_then_private_copies_preserve_exports(self):
        paths, receipt = self.material()
        def actual_cli(argv, **kwargs):
            if argv[1] == '--version':
                return subprocess.CompletedProcess(argv, 0, 'zkir 2.1.1\n', '')
            self.assertEqual(argv[1], 'mock-compile')
            self.source.with_suffix('.bzkir').write_bytes(paths['binary_ir'].read_bytes())
            return subprocess.CompletedProcess(argv, 0, '', 'Mock compiling circuit (k=6, rows=19)\n')
        prover, verifier = self.root / 'keys/circuit.prover', self.root / 'keys/circuit.verifier'
        with patch.object(cache, 'prepare', return_value=(paths, receipt)), patch.object(cache.subprocess, 'run', side_effect=actual_cli):
            result = cache.compile_circuit(self.source, prover, verifier, '/actual/zkir', {})
        self.assertEqual(result['rows'], 19)
        self.assertIn('not keygen', result['model_provider'])
        self.assertEqual(prover.read_bytes(), paths['prover'].read_bytes())
        prover.write_bytes(b'private corruption')
        self.assertEqual(cache.digest(paths['prover']), self.q['prover'])
        paths['prover'].write_bytes(b'corrupt export')
        with self.assertRaises(cache.CompatibilityError):
            cache._copy(paths['prover'], prover, receipt, {})

    def test_bad_serialization_or_destination_alias_refuses_without_keys(self):
        paths, receipt = self.material()
        self.source.with_suffix('.bzkir').write_bytes(b'wrong serialization')
        results = [subprocess.CompletedProcess([], 0, 'zkir 2.1.1', ''),
                   subprocess.CompletedProcess([], 0, '', '(k=6, rows=19)')]
        prover = self.root / 'keys/circuit.prover'
        with patch.object(cache, 'prepare', return_value=(paths, receipt)), patch.object(cache.subprocess, 'run', side_effect=results):
            with self.assertRaises(cache.CompatibilityError):
                cache.compile_circuit(self.source, prover, self.root/'keys/circuit.verifier', '/actual/zkir', {})
        self.assertFalse(prover.exists())
        with self.assertRaises(cache.CompatibilityError):
            cache.compile_circuit(self.source, self.source, self.root/'verifier', '/actual/zkir', {})


if __name__ == '__main__':
    unittest.main()
