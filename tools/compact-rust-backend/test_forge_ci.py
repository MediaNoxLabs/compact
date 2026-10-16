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

"""Orchestration boundaries; these fakes make no real-proof qualification claim."""
import argparse
import json
from pathlib import Path
import tempfile
import sys
import unittest
from unittest.mock import patch

import forge_ci as ci


class ForgeCiTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.q = ci.load(ci.QUALIFICATION)
        self.public_q = json.loads(json.dumps(self.q))
        self.q['parameters'] = {'bls_midnight_2p10': ci.adapter.digest(__file__)}
        self.q['bootstrap_circuits'] = {'bls_midnight_2p10': dict(compact_source=self.q['compact_source'], circuit='raw_after_write')}
        self.q['inputs'] = ['Cargo.lock']
        self.q['input_trees'] = ['tools']
        (self.root / 'tools').mkdir()
        (self.root / 'tools' / 'forge_ci.py').write_text('current orchestration')
        (self.root / 'Cargo.lock').write_text('current consumer lock')
        self.params = self.root / 'params'
        self.params.mkdir()
        self.param = self.params / 'bls_midnight_2p10'
        self.param.write_bytes(Path(__file__).read_bytes())
        self.qualifications = self.root / 'qualification.json'
        ci.write(self.qualifications, self.q)
        self.addCleanup(patch.stopall)
        patch.object(ci, 'ROOT', self.root).start()
        patch.object(ci, 'QUALIFICATION', self.qualifications).start()
        patch.object(ci.adapter, 'validate_consumer').start()
        self.compiler, self.proof, self.helper, self.forge = [self.root / x for x in
                                                          ('compactc', 'proof-runner', 'helper', 'forge')]
        for p in (self.compiler, self.compiler.with_name('compactc-scheme'), self.proof, self.helper, self.forge):
            p.write_text(p.name + ' executable bytes')
        self.phases = []
        self.commands = {}
        self.bad_marker = False
        self.negative_succeeds = False
        self.refuse_restore = False
        self.real_execute = ci.execute
        patch.object(ci, 'execute', side_effect=self.fake_execute).start()

    def fake_execute(self, argv, directory, phase, *, env=None, failure=False):
        argv = list(map(str, argv))
        self.phases.append(phase)
        self.commands[phase] = argv
        if phase == 'profile':
            return json.dumps(dict(profile=ci.adapter.PROFILE, ledger_revision=ci.adapter.REVISION,
                                   build_fingerprint=self.q['build_fingerprint']))
        if phase == 'key':
            value = dict(partition='partition', workload=argv[argv.index('--workload') + 1],
                         parameters=[dict(sha256=x) for x in self.q['parameters'].values()])
            ci.write(Path(argv[argv.index('--output') + 1]), value)
            return json.dumps(value)
        if phase == 'restore' and self.refuse_restore:
            raise ValueError('official helper refused malformed snapshot')
        if phase == 'baseline':
            ci.write(Path(argv[argv.index('--output') + 1]), {'official_helper_baseline': True})
        if phase == 'compile' or phase.startswith('bootstrap-compile-'):
            output = Path(argv[-1])
            for tree, files in {'zkir': ['compose.zkir'], 'keys': ['compose.verifier', 'compose.prover']}.items():
                (output / tree).mkdir(parents=True)
                for name in files:
                    (output / tree / name).write_text('current bytes')
        if phase.startswith('bootstrap-bls'):
            (self.params / phase.removeprefix('bootstrap-')).write_bytes(Path(__file__).read_bytes())
        if phase == 'proof':
            return '\n'.join(self.q['success_markers'][:-1] if self.bad_marker else self.q['success_markers'])
        if phase == 'corrupt-verifier':
            self.assertTrue(failure)
            output = Path(argv[-1])
            self.assertEqual((output / 'keys' / 'compose.verifier').read_bytes(), b'invalid verifier')
            if self.negative_succeeds:
                raise ValueError('corrupt-verifier accepted corrupt verifier')
        return '{}'

    def bootstrap_args(self, fetch=False):
        return argparse.Namespace(compiler=self.compiler, forge=self.forge, params_dir=self.params,
                                  output=self.root / 'bootstrap', fetch_params=fetch)

    def make_plan(self):
        output = self.root / 'plan'
        args = argparse.Namespace(compiler=self.compiler, forge=self.forge, proof_runner=self.proof,
                                  helper=self.helper, params_dir=self.params, output=output,
                                  generation='local-1', github_output=self.root / 'github-output')
        ci.plan(args)
        return output

    def run_args(self, directory, **kwargs):
        return argparse.Namespace(plan=directory, snapshot=self.root / 'snapshot', snapshot_out=None,
                                  max_bytes=1000000, uncached_zkir=None, **kwargs)

    def test_verified_parameters_skip_bootstrap_compilation(self):
        result = ci.bootstrap(self.bootstrap_args())
        self.assertTrue(result['parameters_verified'])
        self.assertFalse(result['proof_evidence'])
        self.assertEqual(self.phases, ['profile'])

    def test_missing_parameters_need_explicit_fetch_and_corruption_never_fetches(self):
        self.param.unlink()
        with self.assertRaisesRegex(ValueError, 'requires --fetch-params'):
            ci.bootstrap(self.bootstrap_args())
        self.assertEqual(self.phases, [])
        self.param.write_text('corrupt present bytes')
        args = self.bootstrap_args(True)
        args.output = self.root / 'second-bootstrap'
        with self.assertRaisesRegex(ValueError, 'digest mismatch'):
            ci.bootstrap(args)
        self.assertEqual(self.phases, [])

    def test_fetch_uses_current_ir_and_forge_private_store(self):
        self.param.unlink()
        result = ci.bootstrap(self.bootstrap_args(True))
        self.assertEqual(result['acquired'], ['bls_midnight_2p10'])
        self.assertIn('--skip-zk', self.commands['bootstrap-compile-0'])
        fetch = self.commands['bootstrap-bls_midnight_2p10']
        self.assertIn('--fetch-params', fetch)
        self.assertIn('prepare', fetch)
        self.assertTrue(Path(fetch[fetch.index('--cache-dir') + 1]).is_relative_to(self.root / 'bootstrap'))
        self.assertTrue(fetch[-1].endswith('/generated-0/zkir/raw_after_write.zkir'))
        self.assertFalse(Path(fetch[fetch.index('--cache-dir') + 1]).exists())

    def test_bootstrap_and_plan_include_uncached_smoke_parameters_grouped_by_source(self):
        smoke = ci.load(Path(__file__).with_name('forge_circuit_qualification.json'))
        for case in smoke.values():
            name = f"bls_midnight_2p{case['k']}"
            self.assertEqual(self.public_q['parameters'][name], case['parameter_digest'])
            self.assertEqual(self.public_q['bootstrap_circuits'][name],
                             dict(compact_source=case['compact_source'], circuit=case['circuit']))
        self.q['parameters'] = {name: ci.adapter.digest(__file__) for name in self.public_q['parameters']}
        self.q['bootstrap_circuits'] = self.public_q['bootstrap_circuits']
        self.qualifications.write_text(json.dumps(self.q))
        self.param.unlink()
        result = ci.bootstrap(self.bootstrap_args(True))
        self.assertEqual(set(result['acquired']), set(self.q['parameters']))
        compiles = [argv for phase, argv in self.commands.items() if phase.startswith('bootstrap-compile-')]
        self.assertEqual(len(compiles), 3)
        self.assertEqual({argv[-2] for argv in compiles},
                         {str(self.root / row['compact_source']) for row in self.q['bootstrap_circuits'].values()})
        for argv in compiles:
            self.assertIn('--skip-zk', argv)
        for name, row in self.q['bootstrap_circuits'].items():
            self.assertTrue(self.commands['bootstrap-' + name][-1].endswith('/zkir/' + row['circuit'] + '.zkir'))
        self.make_plan()
        key = self.commands['key']
        self.assertEqual({key[index + 1] for index, value in enumerate(key) if value == '--parameter'},
                         set(self.q['parameters']))
        for row in self.q['bootstrap_circuits'].values():
            self.assertIn(row['compact_source'], key)

    def test_plan_delegates_key_and_binds_external_executable_bytes(self):
        directory = self.make_plan()
        key = self.commands['key']
        self.assertIn('--github-output', key)
        self.assertEqual(key[key.index('--profile') + 1], 'ledger8-v2')
        self.assertIn('tools/forge_ci.py', key)
        workload = json.loads(key[key.index('--workload') + 1])
        self.assertEqual(workload['tools']['scheme'], ci.file_hash(self.compiler.with_name('compactc-scheme')))
        self.assertEqual(workload['proofs'], 3)
        invocation = ci.load(directory / 'invocation.json')
        self.assertEqual(invocation['tools']['proof_runner']['sha256'], ci.file_hash(self.proof))
        self.proof.write_text('changed runner')
        with self.assertRaisesRegex(ValueError, 'proof_runner changed'):
            ci.run(self.run_args(directory))
        self.assertNotIn('restore', self.phases)

    def test_run_restores_before_baseline_and_executes_current_proofs_and_control(self):
        directory = self.make_plan()
        ci.run(self.run_args(directory))
        self.assertEqual(self.phases, ['profile', 'key', 'restore', 'baseline', 'compile',
                                       'proof', 'corrupt-verifier', 'report'])
        compile = self.commands['compile']
        self.assertIn('forge_circuits.py', ' '.join(compile))
        self.assertTrue(compile[-2].endswith('effect_plan_jubjub.compact'))
        self.assertTrue(ci.load(directory / 'run.json')['passed'])
        self.assertTrue((directory / 'cache-report.json').exists())
        self.assertFalse(list(directory.glob('fixtures-*')))
        with self.assertRaises(FileExistsError):
            ci.run(self.run_args(directory))

    def test_malformed_snapshot_stops_before_baseline_and_proof(self):
        directory = self.make_plan()
        self.refuse_restore = True
        with self.assertRaisesRegex(ValueError, 'malformed snapshot'):
            ci.run(self.run_args(directory))
        self.assertNotIn('baseline', self.phases)
        self.assertNotIn('proof', self.phases)

    def test_incomplete_proof_markers_and_accepted_corrupt_verifier_fail_with_report(self):
        for field in ('bad_marker', 'negative_succeeds'):
            with self.subTest(field=field):
                # A fresh plan is required for each attempt.
                directory = self.root / field
                directory.mkdir()
                ci.write(directory / 'invocation.json', dict(forge=str(self.forge), params_dir=str(self.params),
                         tools=ci.tool_bindings(self.compiler, self.proof, self.helper)))
                invocation = ci.load(directory / 'invocation.json')
                ci.write(directory / 'key.json', dict(workload=json.dumps(dict(
                    tools={name: row['sha256'] for name, row in invocation['tools'].items()}))))
                setattr(self, field, True)
                with self.assertRaises(ValueError):
                    ci.run(self.run_args(directory))
                self.assertFalse((directory / 'run.json').exists())
                self.assertTrue((directory / 'cache-report.json').exists())
                self.assertFalse(list(directory.glob('fixtures-*')))
                setattr(self, field, False)

    def test_optional_uncached_tool_is_forwarded_and_snapshot_has_planned_scope(self):
        directory = self.make_plan()
        args = self.run_args(directory)
        args.uncached_zkir = self.root / 'preserved legacy zkir'
        args.snapshot_out = self.root / 'new-snapshot'
        ci.run(args)
        command = self.commands['uncached']
        self.assertEqual(command[command.index('--uncached-zkir') + 1], str(args.uncached_zkir))
        snapshot = self.commands['snapshot']
        self.assertEqual(snapshot[snapshot.index('--partition') + 1], 'partition')
        self.assertEqual(snapshot[snapshot.index('--parameter-digest') + 1], next(iter(self.q['parameters'].values())))
        self.assertEqual(snapshot[snapshot.index('--max-bytes') + 1], str(args.max_bytes))
        self.assertLess(self.phases.index('uncached'), self.phases.index('report'))
        self.assertLess(self.phases.index('report'), self.phases.index('snapshot'))

    def test_rewritten_invocation_cannot_change_helper_workload_tool_binding(self):
        directory = self.make_plan()
        invocation = ci.load(directory / 'invocation.json')
        self.proof.write_text('different runner')
        invocation['tools']['proof_runner']['sha256'] = ci.file_hash(self.proof)
        (directory / 'invocation.json').unlink()
        ci.write(directory / 'invocation.json', invocation)
        with self.assertRaisesRegex(ValueError, 'differ from the helper workload'):
            ci.run(self.run_args(directory))
        self.assertNotIn('restore', self.phases)

    def test_tool_replaced_during_proof_and_snapshot_inside_store_are_refused(self):
        directory = self.make_plan()
        args = self.run_args(directory)
        args.snapshot_out = directory / 'cache' / 'snapshot'
        with self.assertRaisesRegex(ValueError, 'outside cache'):
            ci.run(args)
        self.assertNotIn('restore', self.phases)
        def mutate(*argv, **kwargs):
            result = self.fake_execute(*argv, **kwargs)
            if argv[2] == 'proof':
                self.proof.write_text('runner replaced during use')
            return result
        args.snapshot_out = None
        with patch.object(ci, 'execute', side_effect=mutate):
            with self.assertRaisesRegex(ValueError, 'proof_runner changed'):
                ci.run(args)
        self.assertFalse((directory / 'run.json').exists())
        self.assertNotIn('corrupt-verifier', self.phases)
        self.assertFalse(list(directory.glob('fixtures-*')))

    def test_helper_outputs_coexist_with_subprocess_command_logs(self):
        script = "import json,pathlib,sys; pathlib.Path(sys.argv[1]).write_text(json.dumps({'helper_output':True})); print('{}')"
        for phase in ('key', 'baseline'):
            with self.subTest(phase=phase):
                output = self.root / (phase + '.json')
                self.real_execute([sys.executable, '-c', script, output], self.root, phase)
                self.assertEqual(ci.load(output), {'helper_output': True})
                log = ci.load(self.root / (phase + '.command.json'))
                self.assertEqual(log['exit_code'], 0)
                self.assertEqual(log['argv'][-1], str(output))

    def test_execute_retains_logs_on_failure_and_propagates_exit(self):
        with self.assertRaisesRegex(ValueError, 'failed'):
            self.real_execute(['/usr/bin/false'], self.root, 'failed-command')
        self.assertNotEqual(ci.load(self.root / 'failed-command.command.json')['exit_code'], 0)
        with self.assertRaisesRegex(ValueError, 'accepted corrupt verifier'):
            self.real_execute(['/usr/bin/true'], self.root, 'false-negative', failure=True)


if __name__ == '__main__':
    unittest.main()
