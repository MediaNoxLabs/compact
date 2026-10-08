"""Copy reviewed SDK source without building. ADR0324 validated this staging layout."""
import argparse, shutil, pathlib
p=argparse.ArgumentParser();p.add_argument('--source',required=True,type=pathlib.Path);p.add_argument('--examples',required=True,type=pathlib.Path);a=p.parse_args()
source=a.source.resolve();dest=a.examples.resolve()/'sdk'
if dest.exists():raise SystemExit('Choose a new examples directory: sdk already exists')
# Keep the original manifests and sibling paths. No generated or package code edits.
packages=['runtime-rs','runtime-rs-macros','testkit-rs']
fixtures=['counter-parameter','witnesses-oracle','set-size-oracle','witness-conditional','merkle-tree-oracle']
for relative in packages+['tests-rust-backend/'+x for x in fixtures]:
 shutil.copytree(source/relative,dest/relative,ignore=shutil.ignore_patterns('target','.git','__pycache__'))
print(dest)
