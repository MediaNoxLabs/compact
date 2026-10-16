# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
"""Workflow planning, scoped restoration and partial-failure controls (no remote writes)."""
import copy
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import workflow_control as control


def row(identifier, name, state='active'):
    return dict(id=identifier, path='.github/workflows/' + name, state=state, name=name)


class Client:
    repo = 'betsudo/compact'

    def __init__(self, rows):
        self.rows = {r['id']: copy.deepcopy(r) for r in rows}
        self.writes = []
        self.fail = None

    def get(self, identifier):
        return self.rows[identifier]

    def set(self, identifier, state):
        if identifier == self.fail:
            raise ValueError('simulated API failure')
        self.writes.append((identifier, state))
        self.rows[identifier]['state'] = state


class WorkflowControlTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.snapshot = Path(self.temporary.name) / 'before.json'
        self.rows = [row(1, 'build-vscplugin.yml'), row(2, 'compiler-fuzzer.yml'),
                     row(3, 'rust-backend-checks.yml'), row(4, 'unknown.yml')]
        self.client = Client(self.rows)
        self.changes = control.plan(self.rows, {'build-vscplugin.yml': 'disabled_manually',
                                                'compiler-fuzzer.yml': 'disabled_manually'})

    def test_focus_changes_only_named_optional_workflows(self):
        with patch.object(control.GitHub, 'inventory', return_value=self.rows), \
             patch.object(control.GitHub, 'set') as write:
            control.main(['--repo', self.client.repo, 'focus'])
        write.assert_not_called()
        self.assertEqual({r['id'] for r in self.changes}, {1, 2})
        self.assertFalse(self.snapshot.exists())
        self.assertFalse(control.PROTECTED & control.PAUSED.keys())

    def test_apply_then_restore_changes_only_original_operations(self):
        control.apply(self.client, self.rows, self.changes, self.snapshot)
        snapshot = json.loads(self.snapshot.read_text())
        self.assertEqual(snapshot['inventory'], self.rows)
        reverse = control.restore_plan(snapshot, self.client.repo, list(self.client.rows.values()))
        restored = self.snapshot.with_name('restore.json')
        control.apply(self.client, list(self.client.rows.values()), reverse, restored)
        self.assertTrue(all(r['state'] == 'active' for r in self.client.rows.values()))
        self.assertEqual({r[0] for r in self.client.writes}, {1, 2})
        self.assertEqual(control.restore_plan(snapshot, self.client.repo, list(self.client.rows.values())), [])

    def test_snapshot_required_and_never_overwritten(self):
        with self.assertRaisesRegex(ValueError, 'snapshot'):
            control.apply(self.client, self.rows, self.changes, None)
        self.snapshot.write_text('retained')
        with self.assertRaises(FileExistsError):
            control.apply(self.client, self.rows, self.changes, self.snapshot)
        self.assertEqual(self.snapshot.read_text(), 'retained')
        self.assertEqual(self.client.writes, [])

    def test_partial_failure_keeps_restorable_snapshot(self):
        self.client.fail = 2
        with self.assertRaisesRegex(ValueError, 'API failure'):
            control.apply(self.client, self.rows, self.changes, self.snapshot)
        self.assertTrue(self.snapshot.exists())
        self.client.fail = None
        reverse = control.restore_plan(json.loads(self.snapshot.read_text()), self.client.repo,
                                       list(self.client.rows.values()))
        self.assertEqual([r['id'] for r in reverse], [1])

    def test_state_drift_prevents_write(self):
        self.client.rows[1]['state'] = 'disabled_inactivity'
        with self.assertRaisesRegex(ValueError, 'changed after planning'):
            control.apply(self.client, self.rows, self.changes, self.snapshot)
        self.assertEqual(self.client.writes, [])

    def test_changed_identity_prevents_restore(self):
        control.apply(self.client, self.rows, self.changes, self.snapshot)
        snapshot = json.loads(self.snapshot.read_text())
        self.client.rows[1]['path'] = '.github/workflows/replacement.yml'
        with self.assertRaisesRegex(ValueError, 'identity changed'):
            control.restore_plan(snapshot, self.client.repo, list(self.client.rows.values()))

    def test_wrong_repo_schema_and_duplicate_restore_refused(self):
        control.apply(self.client, self.rows, self.changes, self.snapshot)
        snapshot = json.loads(self.snapshot.read_text())
        for amendment in ({'repo': 'another/repo'}, {'schema': 2},
                          {'changes': snapshot['changes'] * 2}):
            with self.subTest(amendment=amendment), self.assertRaises(ValueError):
                control.restore_plan(dict(snapshot, **amendment), self.client.repo,
                                     list(self.client.rows.values()))

    def test_protected_missing_ambiguous_and_unknown_states_refused(self):
        for rows, desired in [
            (self.rows, {'rust-backend-checks.yml': 'disabled_manually'}),
            (self.rows, {'absent.yml': 'active'}),
            (self.rows + [row(5, 'unknown.yml')], {'unknown.yml': 'active'}),
            ([row(6, 'paused.yml', 'disabled_inactivity')], {'paused.yml': 'active'}),
            (self.rows, {'unknown.yml': 'bogus'}),
        ]:
            with self.subTest(desired=desired), self.assertRaises(ValueError):
                control.plan(rows, desired)

    def test_unconfirmed_mutation_fails(self):
        with patch.object(self.client, 'set'):
            with self.assertRaisesRegex(ValueError, 'did not confirm'):
                control.apply(self.client, self.rows, self.changes, self.snapshot)

    def test_noop_plan_and_explicit_repository_validation(self):
        self.assertEqual(control.plan(self.rows, {'unknown.yml': 'active'}), [])
        for repo in ('', 'origin', '../repo', 'owner/repo/extra', 'owner/repo?x=y'):
            with self.subTest(repo=repo), self.assertRaises(ValueError):
                control.GitHub(repo)

    def test_gh_uses_explicit_repo_and_pagination_and_propagates_failure(self):
        result = control.subprocess.CompletedProcess([], 0, json.dumps([{'workflows': self.rows}]), '')
        with patch.object(control.subprocess, 'run', return_value=result) as run:
            rows = control.GitHub(self.client.repo).inventory()
        self.assertEqual(rows, self.rows)
        argv = run.call_args.args[0]
        self.assertIn('repos/betsudo/compact/actions/workflows?per_page=100', argv)
        self.assertIn('--paginate', argv)
        self.assertIn('--slurp', argv)
        result.returncode = 1
        result.stderr = 'permission denied'
        with patch.object(control.subprocess, 'run', return_value=result), \
             self.assertRaisesRegex(ValueError, 'permission denied'):
            control.GitHub(self.client.repo).inventory()


if __name__ == '__main__':
    unittest.main()
