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

import copy
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
import did_primitive_reducer_gate as gate


class EvidenceTests(unittest.TestCase):
    def setUp(self):
        self.row={'kind':'alias-set','operations':[{'operation':'mutate'}],'proof_cases':[{'case':'insert','operation':'mutate'},{'case':'remove','operation':'mutate'}]}
        self.good={'format':'compact-did-primitive-reducer-proof/v1','kind':'alias-set',
                   'status':'passed','strictness':'default','constructor_data_deployed':True,
                   'constructor_execution_proved':False,'calls':[]}
        for case in self.row['proof_cases']:
            self.good['calls'].append({**case,'proof_bytes':2912,
                                      'state_bytes':100,'changed_binding_rejected':True,
                                      'applied':True,'recorded_state_matches':True,
                                      'replay_unchanged':True,'replay_refusal':'IntentAlreadyExists'})

    def test_complete_rows_pass(self):
        gate.validate_summary(self.good,self.row)

    def test_missing_duplicate_extra_or_wrong_operation_refuse(self):
        for change in ('missing','duplicate','extra','reordered','operation'):
            value=copy.deepcopy(self.good)
            if change=='missing':value['calls'].pop()
            elif change=='duplicate':value['calls'][1]=copy.deepcopy(value['calls'][0])
            elif change=='extra':value['calls'].append(copy.deepcopy(value['calls'][0]))
            elif change=='reordered':value['calls'].reverse()
            else:value['calls'][1]['operation']='another'
            with self.subTest(change=change), self.assertRaises(gate.common.GateError):
                gate.validate_summary(value,self.row)

    def test_each_required_flag_and_nonempty_proof_is_required(self):
        for key in ('changed_binding_rejected','applied','recorded_state_matches','replay_unchanged',
                    'proof_bytes','state_bytes','replay_refusal'):
            for bad in (None,False,0):
                value=copy.deepcopy(self.good);value['calls'][1][key]=bad
                with self.subTest(key=key,bad=bad), self.assertRaises(gate.common.GateError):
                    gate.validate_summary(value,self.row)
        value=copy.deepcopy(self.good);value['calls'][0]['proof_bytes']=True
        with self.assertRaises(gate.common.GateError):gate.validate_summary(value,self.row)

    def test_wrong_profile_and_constructor_claim_refuse(self):
        for key,bad in [('kind','point-guard'),('status','failed'),('strictness','relaxed'),
                        ('constructor_data_deployed',False),('constructor_execution_proved',True)]:
            value=copy.deepcopy(self.good);value[key]=bad
            with self.subTest(key=key), self.assertRaises(gate.common.GateError):
                gate.validate_summary(value,self.row)

    def test_failed_command_retains_failed_receipt_and_no_completed_rows(self):
        with tempfile.TemporaryDirectory() as temp:
            root=Path(temp);out=root/'gate'
            row={**self.row,'package':'fixture'}
            def failed(*args,**kwargs):raise gate.common.GateError('simulated failed consumer compile')
            with patch.object(gate,'reviewed_rows',return_value=[row]), \
                 patch.object(gate,'source_inventory',return_value={}), \
                 patch.object(gate.ledger_static,'FIXTURE_HASHES',{}), \
                 patch.object(gate.shutil,'which',return_value='/usr/bin/true'), \
                 patch.object(gate.common,'git_head',return_value='frozen'), \
                 patch.object(gate.common,'stable_copy',return_value={'snapshot':'frozen','sha256':'hash'}), \
                 patch.object(gate.common,'run',side_effect=failed):
                result=gate.run_gate(out,root,root,root,[],
                                    {'MIDNIGHT_PP':str(root),'MIDNIGHT_LEDGER_TEST_STATIC_DIR':str(root)})
            self.assertEqual(result['status'],'failed')
            self.assertEqual(result['reducers'],[])
            self.assertIn('simulated failed',result['error'])
            self.assertEqual(json.loads((out/'receipt.json').read_text()),result)


if __name__=='__main__':
    unittest.main()
