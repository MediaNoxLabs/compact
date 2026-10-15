# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
"""Closed source-epoch evolution, independent from test/source read projection."""
import copy
import json
from pathlib import Path
import tempfile
import unittest
import historical_law_source_transition as transition

class TransitionTests(unittest.TestCase):
    def setUp(self):
        self.temp=tempfile.TemporaryDirectory();self.addCleanup(self.temp.cleanup)
        self.root=Path(self.temp.name)
        self.original={'owner.py':transition.digest(b'old owner'), 'unchanged.rs':transition.digest(b'unchanged'), 'absent.rs':None}
        epoch={'actual_source_fingerprints':self.original}
        epoch_bytes=json.dumps(epoch).encode();(self.root/'epoch.json').write_bytes(epoch_bytes)
        (self.root/'owner.py').write_bytes(b'current owner');(self.root/'old-owner.txt').write_bytes(b'old owner')
        (self.root/'new-helper.py').write_bytes(b'current helper');(self.root/'unchanged.rs').write_bytes(b'unchanged')
        self.manifest={'schema_version':1,'issue':632,
            'epoch':{'path':'epoch.json','sha256':transition.digest(epoch_bytes),'canonical_sha256':transition.canonical(epoch)},
            'current_sources':{'owner.py':transition.digest(b'current owner'),'new-helper.py':transition.digest(b'current helper')},
            'owner_inverses':{'owner.py':{'path':'old-owner.txt','sha256':transition.digest(b'old owner')}}}
        self.descriptor={'actual_source_fingerprints':self.original}
        self.seal()

    def seal(self):
        (self.root/'transition.json').write_text(json.dumps(self.manifest))
        self.descriptor['current_source_transition']={'path':'transition.json','canonical_sha256':transition.canonical(self.manifest)}

    def test_reviewed_current_guards_preserve_old_epoch_and_never_return_old_code(self):
        expected=transition.fingerprints(self.root,self.descriptor)
        self.assertEqual(expected,{**self.original,**self.manifest['current_sources']})
        self.assertEqual((self.root/'owner.py').read_bytes(),b'current owner')
        self.assertEqual(self.descriptor['actual_source_fingerprints'],self.original)
        self.assertIsNone(expected['absent.rs'])

    def test_current_owner_and_new_helper_drift_are_live_on_every_call(self):
        for name in ('owner.py','new-helper.py'):
            original=(self.root/name).read_bytes()
            transition.fingerprints(self.root,self.descriptor)
            (self.root/name).write_bytes(original+b'changed')
            with self.assertRaisesRegex(ValueError,'owner drift'):
                transition.fingerprints(self.root,self.descriptor)
            (self.root/name).write_bytes(original)

    def test_missing_forged_or_extra_inverse_cannot_authorize_current_owner(self):
        initial=copy.deepcopy(self.manifest)
        for edit in ('missing','forged','extra'):
            self.manifest=copy.deepcopy(initial)
            if edit=='missing':self.manifest['owner_inverses'].clear()
            if edit=='forged':self.manifest['owner_inverses']['owner.py']['sha256']='f'*64
            if edit=='extra':self.manifest['owner_inverses']['new-helper.py']={'path':'old-owner.txt','sha256':transition.digest(b'old owner')}
            self.seal()
            with self.subTest(edit=edit),self.assertRaisesRegex(ValueError,'inverse'):
                transition.fingerprints(self.root,self.descriptor)

    def test_epoch_guard_mutation_and_historical_bytes_drift_refuse(self):
        (self.root/'old-owner.txt').write_bytes(b'forged old owner')
        with self.assertRaisesRegex(ValueError,'inverse'):
            transition.fingerprints(self.root,self.descriptor)
        (self.root/'old-owner.txt').write_bytes(b'old owner')
        self.descriptor['actual_source_fingerprints']={**self.original,'owner.py':'f'*64}
        with self.assertRaisesRegex(ValueError,'original source epoch'):
            transition.fingerprints(self.root,self.descriptor)

    def test_unreviewed_manifest_duplicate_fields_and_path_escape_refuse(self):
        (self.root/'transition.json').write_text('{}')
        with self.assertRaisesRegex(ValueError,'unreviewed'):
            transition.fingerprints(self.root,self.descriptor)
        (self.root/'transition.json').write_text('{"issue":632,"issue":632}')
        with self.assertRaisesRegex(ValueError,'duplicate'):
            transition.fingerprints(self.root,self.descriptor)
        for path in ('../outside','/tmp/outside',''):
            with self.subTest(path=path),self.assertRaisesRegex(ValueError,'path'):
                transition.within(self.root,path)

    def test_no_transition_preserves_original_authority_without_projection(self):
        descriptor={'actual_source_fingerprints':self.original}
        self.assertIs(transition.fingerprints(self.root,descriptor),self.original)

if __name__=='__main__':unittest.main()
