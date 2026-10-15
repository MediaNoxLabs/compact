"""Full-gate case identity, explicit fixture inputs and failure propagation."""
import copy
import json
from pathlib import Path
import unittest

import local_parity_gate as gate
ROOT = gate.ROOT
GROUPS = ("ordinary", "earlier_inventory", "latest_inventory", "current_measurement")
REPORT = {
    "schema_version": 1, "issue": 632, "descriptor_sha256": "f" * 64,
    "status": "completed", "passed": True, "actual_case_ids_match_plan": True,
    "not_run_case_ids": {}, "failures": 0, "errors": 0, "skipped": [],
    "planned_case_ids": {g: [g + ".Case.first", g + ".Case.second"] for g in GROUPS},
    "groups": {g: {"actual_case_ids": [g + ".Case.first", g + ".Case.second"],
                   "tests_run": 2, "failures": [], "errors": [], "skipped": []} for g in GROUPS},
    "planned_case_count": 8, "tests_run": 8,
}

class FullGateDispatchTests(unittest.TestCase):
    def test_complete_dynamic_report_is_accepted(self):
        gate.validate_historical_law_report(REPORT)

    def refuse(self, mutate):
        value = copy.deepcopy(REPORT)
        mutate(value)
        with self.assertRaises(gate.GateError):
            gate.validate_historical_law_report(value)

    def test_failed_prepared_partial_and_skipped_reports_refuse(self):
        for key,value in [('passed',False),('status','prepared'),('status','failed-partial'),
                          ('failures',1),('errors',1),('skipped',['case']),
                          ('actual_case_ids_match_plan',False),('not_run_case_ids',{'ordinary':['x']})]:
            with self.subTest(key=key,value=value):
                self.refuse(lambda v: v.__setitem__(key,value))

    def test_missing_extra_duplicate_and_reordered_cases_refuse(self):
        self.refuse(lambda v: v['groups'].pop('ordinary'))
        self.refuse(lambda v: v['planned_case_ids'].__setitem__('extra',[]))
        self.refuse(lambda v: v['groups']['ordinary']['actual_case_ids'].reverse())
        def duplicate(v):
            v['planned_case_ids']['ordinary'][1]=v['planned_case_ids']['ordinary'][0]
            v['groups']['ordinary']['actual_case_ids']=v['planned_case_ids']['ordinary'][:]
        self.refuse(duplicate)

    def test_counts_and_success_flags_cannot_hide_group_failures(self):
        for key in ('planned_case_count','tests_run'):
            self.refuse(lambda v: v.__setitem__(key,128))
        for key in ('errors','failures','skipped'):
            self.refuse(lambda v: v['groups']['ordinary'].__setitem__(key,['hidden']))
        self.refuse(lambda v: v['groups']['ordinary'].__setitem__('tests_run',1))

    def test_integer_counts_and_protocol_identity_are_strict(self):
        for key in ('schema_version','issue','planned_case_count','tests_run','failures','errors'):
            self.refuse(lambda v: v.__setitem__(key,True))
        self.refuse(lambda v: v['groups']['current_measurement'].__setitem__('tests_run',True))
        self.refuse(lambda v: v.__setitem__('descriptor_sha256','unbound'))

    def test_explicit_roots_and_typed_child_receipt_are_joined(self):
        import tempfile
        with tempfile.TemporaryDirectory() as tmp:
            directory=Path(tmp); archive=directory/'archive';graphs=directory/'graphs'
            environment={'explicit':'configuration'};receipt={};calls=[]
            def command(argv,label,where,owner,*,env):
                calls.append(argv)
                self.assertEqual(label,'parity-harness-tests')
                self.assertIs(owner,receipt);self.assertIs(env,environment)
                self.assertEqual(where,directory)
                (directory/'historical-laws.json').write_text(json.dumps(REPORT))
            gate.run_historical_laws(directory,archive,graphs,environment,receipt,command=command)
            argv=calls[0]
            self.assertEqual(argv[argv.index('--root')+1],str(ROOT))
            self.assertEqual(argv[argv.index('--archived-conformance-root')+1],str(archive.resolve()))
            self.assertEqual(argv[argv.index('--guarded-point-artifacts')+1],str(graphs.resolve()))
            self.assertNotIn('discover',argv)
            self.assertEqual(receipt['historical_laws']['tests_run'],8)
            self.assertEqual(receipt['historical_laws']['sha256'],gate.sha256(directory/'historical-laws.json'))

    def test_command_failure_does_not_read_an_old_receipt(self):
        import tempfile
        with tempfile.TemporaryDirectory() as tmp:
            directory=Path(tmp);receipt={}
            (directory/'historical-laws.json').write_text(json.dumps(REPORT))
            def command(*args,**kwargs):
                raise gate.GateError('child failed')
            with self.assertRaisesRegex(gate.GateError,'child failed'):
                gate.run_historical_laws(directory,directory,directory,{},receipt,command=command)
            self.assertNotIn('historical_laws',receipt)

if __name__=='__main__':
    unittest.main()
