# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
"""Encoding-only authority cannot hide changes to admission or historical evidence."""
import contextlib
import io
import json
import unittest
import recording_admission_inventory as inventory
import recording_admission_inventory_encoding as encoding
import test_recording_admission_inventory as previous
import test_recording_admission_inventory_aligned_test as prior
import test_recording_admission_inventory_funded_mint as funded

class EncodingExtensionTests(unittest.TestCase):
    def setUp(self):
        prior.AlignedTestExtensionTests.setUp(self)
        self.encoding_extension = json.loads(previous.ENCODING_EXTENSION.read_text())
        (self.root / encoding.OWNER).write_bytes((inventory.ROOT / encoding.OWNER).read_bytes())

    def report(self):
        return funded.FundedMintTransitionTests.report(self,
            keccak_test_extension=self.keccak_extension,
            aligned_test_extension=self.aligned_extension,
            encoding_extension=self.encoding_extension)

    def test_exact_encoding_preserves_routes_and_all_admission_bytes(self):
        report = self.report()
        self.assertTrue(report['passed'], report['errors'])
        self.assertEqual(report['route_count'], 84)
        self.assertEqual(len(report['retired_routes']), 19)
        self.assertEqual(report['current_selector'], encoding.SELECTOR)
        self.assertEqual(report['reviewed_encoding_extension']['new_retired_routes'], [])
        restored = encoding.predecessor_source(self.root, self.encoding_extension)
        current = (self.root / encoding.OWNER).read_text()
        self.assertEqual(encoding.admission_prefix(current), encoding.admission_prefix(restored))
        self.assertEqual(inventory.digest(restored.encode()), encoding.PREVIOUS_SHA)

    def test_missing_forged_or_relabelled_manifest_fails_closed(self):
        for replacement in [None, {**self.encoding_extension,'claim':'forged'},
                            {**self.encoding_extension,'new_retired_routes':['selector.unit-composition']}]:
            original,self.encoding_extension=self.encoding_extension,replacement
            self.assertFalse(self.report()['passed'])
            self.encoding_extension=original

    def test_admission_body_rendering_and_partial_rollback_cannot_hide(self):
        p=self.root/encoding.OWNER; original=p.read_text()
        for changed in [original.replace('arguments.len() != declaration.parameters.len()', 'arguments.len() == declaration.parameters.len()',1),
                        original.replace('Type::Unit | Type::OpaqueBytes','Type::Unit',1),
                        original.replace('frame.#frame_method(', 'frame.try_witness_metered(',1),
                        original+'\n// source drift\n']:
            self.assertNotEqual(original,changed);p.write_text(changed)
            self.assertFalse(self.report()['passed'])
        p.write_text(original)

    def test_actual_selector_order_and_other_production_remain_live(self):
        for owner in [inventory.SELECTOR, inventory.RECORDED]:
            p=self.root/owner;before=p.read_text();p.write_text(before+'\n// drift\n')
            self.assertFalse(self.report()['passed'],owner);p.write_text(before)
        with self.assertRaises(ValueError):
            encoding.checkpoint(self.root,self.encoding_extension,self.aligned_extension,
                                {**encoding.SELECTOR,'ordered_calls':[]})

    def test_full_predecessor_chain_required(self):
        for field in ['aligned_extension','keccak_extension','funded_transition','inequality_extension','transition']:
            original=getattr(self,field)
            for replacement in [None,{**original,'claim':'forged'}]:
                setattr(self,field,replacement);self.assertFalse(self.report()['passed'],field)
            setattr(self,field,original)

    def test_rehashed_source_and_arbitrary_overlay_cannot_create_authority(self):
        p=self.root/encoding.OWNER;p.write_text(p.read_text()+'\n// changed\n')
        self.encoding_extension['current_sha256']=inventory.digest(p.read_bytes())
        self.assertFalse(self.report()['passed'])
        with self.assertRaises(ValueError):encoding._EncodingCheckpoint(self.root,'forged')

    def test_cli_protects_manifest(self):
        before=previous.ENCODING_EXTENSION.read_bytes()
        with contextlib.redirect_stderr(io.StringIO()),self.assertRaises(SystemExit):
            inventory.main(['--encoding-extension',str(previous.ENCODING_EXTENSION),'--output',str(previous.ENCODING_EXTENSION)])
        self.assertEqual(previous.ENCODING_EXTENSION.read_bytes(),before)

if __name__=='__main__':unittest.main()
