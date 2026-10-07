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

"""Exact relation proof inventory and retained-material refusal controls."""
import copy
from pathlib import Path
import tempfile
import unittest

import did_relation_gate as gate


class RelationGateTests(unittest.TestCase):
    def original(self, scenario):
        selector, cases, operations = gate.SCENARIOS[scenario]
        return {'format': 'compact-did-proof-result/v1', 'scenario': scenario,
                'selector': selector, 'strictness': 'default', 'deployment_applied': True,
                'installed_operations': (["setVerificationMethod", "removeVerificationMethod",
                                          gate.RELATION] if scenario == "relations" else
                                         ["setSchnorrJubjubVerificationMethod",
                                          "verifySchnorrJubjubDigestSignature", gate.RELATION]),
                'constructor_execution_proved': False,
                'final_state_file': f'did-{scenario}-final-state.bin',
                'calls': [{'case': case, 'operation': operation, 'proof_bytes': 1,
                           'applied': True, 'changed_binding_rejected': True,
                           'replay_refusal': 'IntentAlreadyExists'}
                          for case, operation in zip(cases, operations)]}

    def reducer(self, kind):
        cases = {'relation-two': ('authentication-insert', 'authentication-remove'),
                 'relation-four': ('delegation-insert', 'delegation-remove'),
                 'relation-nested': ('missing-signing-short-circuit', 'x25519-check')}[kind]
        return {'format': 'compact-did-primitive-reducer-proof/v1', 'kind': kind,
                'status': 'passed', 'strictness': 'default',
                'constructor_data_deployed': True, 'constructor_execution_proved': False,
                'calls': [{'case': case, 'operation': gate.REDUCERS[kind][2],
                           'proof_bytes': 1, 'state_bytes': 1, 'applied': True,
                           'changed_binding_rejected': True, 'recorded_state_matches': True,
                           'replay_unchanged': True, 'replay_refusal': 'IntentAlreadyExists'}
                          for case in cases]}

    def test_exact_original_nineteen_inventory(self):
        self.assertEqual(sum(len(cases) for _, cases, _ in gate.SCENARIOS.values()), 19)
        for scenario in gate.SCENARIOS:
            with self.subTest(scenario=scenario):
                gate.validate_original(self.original(scenario), scenario)

    def test_original_missing_reordered_or_weakened_call_rejected(self):
        for scenario in gate.SCENARIOS:
            base = self.original(scenario)
            mutants = []
            for key, value in [('applied', False), ('changed_binding_rejected', False),
                               ('replay_refusal', 'accepted'), ('proof_bytes', 0)]:
                mutant = copy.deepcopy(base); mutant['calls'][0][key] = value; mutants.append(mutant)
            mutant = copy.deepcopy(base); mutant['calls'].pop(); mutants.append(mutant)
            mutant = copy.deepcopy(base); mutant['calls'].reverse(); mutants.append(mutant)
            mutant = copy.deepcopy(base); mutant['selector'] = '--did-proof-lifecycle'; mutants.append(mutant)
            mutant = copy.deepcopy(base); mutant['strictness'] = 'unbalanced'; mutants.append(mutant)
            for mutant in mutants:
                with self.subTest(scenario=scenario, mutant=mutant):
                    with self.assertRaises(gate.common.GateError):
                        gate.validate_original(mutant, scenario)

    def test_exact_six_reducer_calls_and_failure_controls(self):
        self.assertEqual(2 * len(gate.REDUCERS), 6)
        for kind in gate.REDUCERS:
            base = self.reducer(kind)
            gate.validate_reducer(base, kind)
            for key, value in [('applied', False), ('recorded_state_matches', False),
                               ('replay_unchanged', False), ('proof_bytes', 0)]:
                mutant = copy.deepcopy(base); mutant['calls'][0][key] = value
                with self.subTest(kind=kind, key=key), self.assertRaises(gate.common.GateError):
                    gate.validate_reducer(mutant, kind)
            mutant = copy.deepcopy(base); mutant['calls'].reverse()
            with self.assertRaises(gate.common.GateError):
                gate.validate_reducer(mutant, kind)

    def test_full_original_compact_import_closure_is_hashed(self):
        import json
        base = gate.ROOT / "examples/rust_backend/did_adoption"
        manifest = base / "source-manifest.json"
        inventory = gate.source_inventory()
        self.assertIn(str(manifest), inventory)
        for source in json.loads(manifest.read_text())["files"]:
            path = base / source["path"]
            self.assertEqual(inventory[str(path)], source["sha256"])

    def test_uninvoked_installed_verifier_is_part_of_material_inventory(self):
        # The generic relation chain installs removal even though its 16 calls
        # only insert methods, then add/remove relations.
        self.assertIn("removeVerificationMethod", gate.ORIGINAL_OPERATIONS)
        self.assertEqual(len(set(gate.ORIGINAL_OPERATIONS)), 5)

    def test_missing_prerequisites_leave_failed_receipt_without_execution(self):
        import json
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            def forbidden(*args, **kwargs):
                self.fail("proof execution must not begin without prerequisites")
            result = gate.run_gate(root / "run", root / "compiler", root / "scheme",
                                   root / "target", environment={}, command=forbidden)
            self.assertEqual(result["status"], "failed")
            self.assertEqual(result["commands"], [])
            self.assertEqual(json.loads((root / "run/receipt.json").read_text()), result)

    def test_missing_empty_and_linked_material_rejected(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            missing = root / 'missing'
            with self.assertRaises(gate.common.GateError):
                gate.checked_file(missing)
            target = root / 'material'; target.write_bytes(b'')
            with self.assertRaises(gate.common.GateError):
                gate.checked_file(target)
            target.write_bytes(b'key')
            linked = root / 'linked'; linked.symlink_to(target)
            with self.assertRaises(gate.common.GateError):
                gate.checked_file(linked)
            self.assertEqual(gate.checked_file(target), target)

    def test_original_import_closure_is_part_of_frozen_inventory(self):
        closure = gate.did.source_inventory()
        inventory = gate.source_inventory()
        self.assertEqual({path: inventory[path] for path in closure}, closure)
        self.assertIn(str(gate.ROOT / 'examples/rust_backend/did_adoption/packages/jubjub-schnorr/src/schnorr.compact'), inventory)


if __name__ == '__main__':
    unittest.main()
