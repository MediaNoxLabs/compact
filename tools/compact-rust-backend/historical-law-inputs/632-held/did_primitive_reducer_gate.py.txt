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

"""Qualify selected maintained DID primitive reducers using exact local artifacts."""
import argparse
import json
import os
from pathlib import Path
import re
import shutil
import sys
import local_parity_gate as common
import resolve_ledger_test_static as ledger_static

ROOT = common.ROOT
REGISTRY = ROOT / 'tools/compact-rust-backend/did_primitive_reducers.json'


def require(condition, message):
    if not condition:
        raise common.GateError(message)


def read_json(path):
    value = json.loads(path.read_text())
    require(isinstance(value, dict), f'expected object: {path}')
    return value


def hashes(paths):
    return {str(p): common.sha256(p) for p in sorted(set(paths)) if p.is_file()}


def reviewed_rows():
    value = read_json(REGISTRY)
    require(value.get('format') == 'compact-did-primitive-reducers/v1', 'unknown registry')
    rows = value['reducers']
    require(len(rows) == len({r['kind'] for r in rows}), 'duplicate reviewed reducer')
    for row in rows:
        require(row['proof_cases'] and len(row['proof_cases']) == len({c['case'] for c in row['proof_cases']}),
                'empty or duplicate proof cases')
        operations = {c['operation'] for c in row['operations']}
        require(operations and len(operations) == len(row['operations']), 'empty or duplicate operations')
        require(all(c['operation'] in operations for c in row['proof_cases']), 'unreviewed proof operation')
        for name in ('source', 'fixture'):
            path = Path(row[name])
            require(not path.is_absolute() and '..' not in path.parts, 'invalid registry path')
    return rows


def validate_summary(value, row):
    require(value.get('format') == 'compact-did-primitive-reducer-proof/v1'
            and value.get('kind') == row['kind'], 'wrong reducer identity')
    require(value.get('status') == 'passed' and value.get('strictness') == 'default',
            'reducer failed default strictness')
    require(value.get('constructor_data_deployed') is True
            and value.get('constructor_execution_proved') is False, 'constructor boundary changed')
    calls = value.get('calls', [])
    require([{'case': c.get('case'), 'operation': c.get('operation')} for c in calls]
            == row['proof_cases'], 'missing, extra, duplicate or reordered proof case/operation')
    for call in calls:
        for flag in ('changed_binding_rejected', 'applied', 'recorded_state_matches', 'replay_unchanged'):
            require(call.get(flag) is True, f'missing {flag}')
        require(call.get('replay_refusal') == 'IntentAlreadyExists', 'wrong replay refusal')
        for count in ('proof_bytes', 'state_bytes'):
            require(type(call.get(count)) is int and call[count] > 0, f'missing {count}')


def source_inventory(rows):
    files = [REGISTRY, Path(__file__), Path(common.__file__), Path(ledger_static.__file__),
             ROOT/'Cargo.toml', ROOT/'Cargo.lock', ROOT/'tools/compact-rust-proof-smoke/Cargo.toml',
             ROOT/'tools/compact-rust-backend/did_primitive_reducer_capture.mjs',
             ROOT/'tools/compact-rust-backend/did_primitive_reducer_cases.mjs',
             ROOT/'tools/compact-rust-backend/test_did_primitive_reducer_gate.py',
             ROOT/'tools/compact-rust-backend/check_fixture_outputs.py']
    for row in rows:
        files.append(ROOT/row['source'])
        files.extend((ROOT/row['fixture']).rglob('*'))
    for folder in ('runtime-rs', 'runtime-rs-macros', 'testkit-rs', 'tools/compact-rust-proof-smoke/src'):
        files.extend(p for p in (ROOT/folder).rglob('*') if p.suffix in ('.rs','.toml','.json'))
    return hashes(files)


def run_gate(directory, compiler, scheme, target, selected, environment=None):
    rows = reviewed_rows()
    if selected:
        require(set(selected) <= {r['kind'] for r in rows}, 'unknown requested reducer')
        require(len(selected) == len(set(selected)), 'duplicate requested reducer')
        rows = [r for r in rows if r['kind'] in selected]
    require(rows, 'no reviewed reducers selected')
    directory = directory.resolve()
    directory.mkdir(parents=True, exist_ok=False, mode=0o700)
    receipt = {'format':'compact-did-primitive-reducer-gate/v1', 'status':'failed',
               'commands':[], 'expected_reducers':[r['kind'] for r in rows], 'reducers':[]}
    env = dict(os.environ if environment is None else environment)
    try:
        for key in ('MIDNIGHT_PP','MIDNIGHT_LEDGER_TEST_STATIC_DIR'):
            require(bool(env.get(key)) and Path(env[key]).is_absolute() and Path(env[key]).is_dir(),
                    f'{key} must select an absolute existing directory')
        zkir = shutil.which('zkir', path=env.get('PATH'))
        require(zkir is not None, 'zkir missing')
        dust_files=[]
        for name,digest in ledger_static.FIXTURE_HASHES.items():
            path=Path(env['MIDNIGHT_LEDGER_TEST_STATIC_DIR'])/'dust'/name
            require(path.is_file() and common.sha256(path)==digest, f'Dust fixture mismatch: {name}')
            declaration=path.with_name(path.name+'.sha256')
            require(declaration.read_text().split()[0]==digest, f'Dust declaration mismatch: {name}')
            dust_files += [path,declaration]
        receipt['dust_fixture_hashes']=hashes(dust_files)
        receipt['source_hashes']=source_inventory(rows)
        receipt['git_head']=common.git_head()
        (directory/'bin').mkdir()
        components=receipt['components']={}
        for name,path in (('compactc',compiler),('compactc-scheme',scheme),('zkir',Path(zkir))):
            components[name]=common.stable_copy(path,directory/'bin'/name)
        env.update({'COMPACTC_SCHEME':components['compactc-scheme']['snapshot'],
                    'COMPACT_RUST_RUNTIME_DIR':str(ROOT),'CARGO_TARGET_DIR':str(target.resolve()),
                    'CARGO_INCREMENTAL':'0','RUSTUP_TOOLCHAIN':'1.99.0','PYTHONDONTWRITEBYTECODE':'1'})
        def execute(argv,label,stdout_path=None):
            common.run([str(a) for a in argv],label,directory,receipt,env=env,stdout_path=stdout_path)
        packages=[arg for row in rows for arg in ('-p',row['package'])]
        execute(['cargo','+1.99.0','test','--locked','--offline',*packages], 'test-reducer-behavior')
        execute(['cargo','+1.99.0','build','--locked','--offline','-p','compact-rust-proof-smoke'],'build-proof-runner')
        components['proof-runner']=common.stable_copy(target.resolve()/'debug/compact-rust-proof-smoke',directory/'bin/compact-rust-proof-smoke')
        runner=components['proof-runner']['snapshot']
        material=directory/'proof-material.json'
        execute([runner,'--prepare-proof-material'],'prepare-proof-material',material)
        prepared=read_json(material)
        require(prepared.get('format')=='compact-proof-material/v1' and prepared.get('mode')=='prepare'
                and Path(prepared['cache_directory']).resolve()==Path(env['MIDNIGHT_PP']).resolve(),
                'proof material selection changed')
        receipt['proof_material']=prepared
        for row in rows:
            kind=row['kind']; output=directory/kind
            execute([components['compactc']['snapshot'],'--target','ts','--target','rust','--skip-zk',
                     '--rust-runtime-root',ROOT,ROOT/row['source'],output], f'{kind}-compile')
            execute(['rustfmt','--edition','2024',output/'contract/lib.rs'],f'{kind}-format')
            require((output/'contract/lib.rs').read_bytes()==(ROOT/row['fixture']/'lib.rs').read_bytes(),
                    f'{kind}: generated fixture stale')
            report=read_json(output/'contract/rust-capabilities.json')
            common.validate_report(report,ROOT/row['source'])
            cap=common.proof_cross_tab(report,read_json(output/'compiler/contract-info.json'),ROOT/row['source'])
            for operation in row['operations']:
                selected_cap=[c for c in cap if c['name']==operation['operation']]
                require(len(selected_cap)==1 and all(selected_cap[0].get(k) is True for k in ('proof','recorded','observed_call')),
                        f'{kind}: selected operation is not proof/recording applicable')
            capture=directory/f'{kind}-capture.json'
            execute(['node',ROOT/'tools/compact-rust-backend/did_primitive_reducer_capture.mjs',kind,
                     output/'contract/index.js'],f'{kind}-capture',capture)
            require(read_json(capture)==read_json(ROOT/row['fixture']/'oracle/cases.json'),f'{kind}: oracle changed')
            (output/'keys').mkdir(exist_ok=True)
            artifacts=[]
            for selection in row['operations']:
                operation=selection['operation']
                execute([components['zkir']['snapshot'],'compile',output/f'zkir/{operation}.zkir',
                         output/f'keys/{operation}.prover',output/f'keys/{operation}.verifier'],f'{kind}-{operation}-keygen')
                model=re.search(r'\(k=(\d+), rows=(\d+)\)',Path(receipt['commands'][-1]['log']).read_text())
                require(model is not None and (int(model[1]),int(model[2]))==(selection['k'],selection['rows']),f'{kind}: key shape changed')
                artifacts.extend(output/f'{folder}/{operation}.{ext}' for folder,ext in [('keys','prover'),('keys','verifier'),('zkir','zkir'),('zkir','bzkir')])
            require(all(p.is_file() and p.stat().st_size>0 for p in artifacts),f'{kind}: missing proof material')
            before=hashes(artifacts)
            execute([runner,'--did-primitive-reducer',kind,output],f'{kind}-prove')
            summary=read_json(output/'proof-result.json');validate_summary(summary,row)
            results=[output/'proof-result.json']
            for call in summary['calls']:
                state=output/f"{call['case']}-state.bin"
                require(state.is_file() and state.stat().st_size==call['state_bytes'],f'{kind}: missing state')
                results.append(state)
            require(hashes(artifacts)==before,f'{kind}: proof material changed')
            receipt['reducers'].append({'kind':kind,'summary':summary,'capabilities':report,
                                        'artifact_hashes':before,'result_hashes':hashes(results+[capture])})
        require([r['kind'] for r in receipt['reducers']]==receipt['expected_reducers'],'incomplete reducer results')
        for result in receipt['reducers']:
            for field in ('artifact_hashes','result_hashes'):
                require(hashes([Path(name) for name in result[field]])==result[field], 'retained evidence changed')
        require(source_inventory(rows)==receipt['source_hashes'] and common.git_head()==receipt['git_head'],
                'source or HEAD changed during gate')
        require(all(common.sha256(Path(r['snapshot']))==r['sha256'] for r in components.values()),'tool changed')
        receipt['status']='passed'
    except (common.GateError,OSError,ValueError,KeyError,IndexError,TypeError) as error:
        receipt['error']=str(error)
    finally:
        (directory/'receipt.json').write_text(json.dumps(receipt,indent=2)+'\n')
    return receipt


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    for flag in ('compiler','scheme','target','output'):
        parser.add_argument('--'+flag,type=Path,required=True)
    parser.add_argument('--only',action='append',default=[])
    args=parser.parse_args()
    receipt=run_gate(args.output,args.compiler,args.scheme,args.target,args.only)
    print(json.dumps({'status':receipt['status'],'error':receipt.get('error')}))
    return 0 if receipt['status']=='passed' else 1


if __name__=='__main__':
    sys.exit(main())
