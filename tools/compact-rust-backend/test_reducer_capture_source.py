"""Three small source-alias laws; no frontend or checker monkeypatch."""
from pathlib import Path
import shutil
import tempfile
import unittest
import local_parity_gate as common
import reducer_capture_source as inputs


class CaptureSourceTests(unittest.TestCase):
    def setUp(self):
        self.temp=tempfile.TemporaryDirectory(); self.addCleanup(self.temp.cleanup)
        self.directory=Path(self.temp.name)/'run'; self.directory.mkdir()
        self.root=Path(self.temp.name)/'repo'; self.root.mkdir()
        for source,_ in inputs.SOURCES.values():
            path=self.root/source; path.parent.mkdir(parents=True,exist_ok=True)
            shutil.copyfile(common.ROOT/source,path)

    def prepare(self, kind):
        source,expected=inputs.SOURCES[kind]
        return inputs.prepare(self.root,self.directory,{'kind':kind,'source':source},expected)

    def test_only_two_names_are_aliased_and_current_source_bytes_are_exact(self):
        for kind in inputs.SOURCES:
            entry=self.prepare(kind)
            original=Path(entry['tracked_source_path']); invocation=Path(entry['invocation_source_path'])
            self.assertEqual(original.read_bytes(),invocation.read_bytes())
            self.assertEqual(invocation.name,inputs.ALIASES.get(kind,original.name))
            self.assertEqual(invocation!=original,kind in inputs.ALIASES)
        self.assertEqual(set(inputs.ALIASES),{'point-map','nested-map'})

    def test_current_or_staged_input_drift_is_refused(self):
        entry=self.prepare('point-map')
        for field in ('tracked_source_path','invocation_source_path'):
            path=Path(entry[field]); original=path.read_bytes(); path.write_bytes(original+b'changed')
            with self.subTest(field=field),self.assertRaises(common.GateError):inputs.verify(self.root,self.directory,entry)
            path.write_bytes(original)

    def test_unknown_alias_source_substitution_and_stale_stage_refuse(self):
        with self.assertRaises(common.GateError):inputs.prepare(self.root,self.directory,{'kind':'unknown','source':'arbitrary'},'0'*64)
        source,expected=inputs.SOURCES['point-map']
        with self.assertRaises(common.GateError):inputs.prepare(self.root,self.directory,{'kind':'point-map','source':'arbitrary'},expected)
        entry=self.prepare('point-map'); entry['historical_basename_alias']='unreviewed.compact'
        with self.assertRaises(common.GateError):inputs.verify(self.root,self.directory,entry)
        with self.assertRaises(FileExistsError):self.prepare('point-map')


if __name__=='__main__':unittest.main()
