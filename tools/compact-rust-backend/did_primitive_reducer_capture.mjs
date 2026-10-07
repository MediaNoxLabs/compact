// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//   http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

import {cases as reviewedCases, argumentsFor} from './did_primitive_reducer_cases.mjs';
import { readFileSync, realpathSync, mkdirSync, symlinkSync, existsSync } from 'node:fs';
import { resolve, dirname } from 'node:path';
import { pathToFileURL, fileURLToPath } from 'node:url';
import { createHash } from 'node:crypto';
const [kind, generated] = process.argv.slice(2);
const repo=resolve(dirname(fileURLToPath(import.meta.url)), '../..');
const registry=JSON.parse(readFileSync(resolve(repo,'tools/compact-rust-backend/did_primitive_reducers.json'),'utf8'));
const selected=registry.reducers.find(row=>row.kind===kind);
if (!selected || process.argv.length !== 4) throw Error('expected reviewed kind and generated index.js');
const runtimePath = resolve(repo, 'runtime');
const modulePath = resolve(dirname(generated),'node_modules/@midnight-ntwrk/compact-runtime');
if (!existsSync(modulePath)) { mkdirSync(dirname(modulePath),{recursive:true}); symlinkSync(runtimePath,modulePath); }
if (realpathSync(modulePath)!==realpathSync(runtimePath)) throw Error('wrong selected runtime realpath');
const r = await import(pathToFileURL(resolve(runtimePath,'dist/index.js')).href);
const c = await import(pathToFileURL(resolve(generated)).href);
const json = value => JSON.parse(JSON.stringify(value,(_,v)=>typeof v==='bigint'?String(v):v instanceof Uint8Array?Array.from(v):v));
const hash = path => createHash('sha256').update(readFileSync(path)).digest('hex');
const key={bytes:new Uint8Array(32)};
let gas=[],witnesses=[],admit=true;
const query=r.QueryContext.prototype.query;
r.QueryContext.prototype.query=function(...args){const out=query.apply(this,args);gas.push(json(out.gasCost));return out;};
const contract=new c.Contract({authorize:({privateState}, message)=>{witnesses.push(message.toString(16).padStart(64,'0').match(/../g).reverse().join(''));return [privateState+1,admit];}});
const arg=(scalar)=>r.ecMulGenerator(BigInt(scalar));
const init=()=>contract.initialState({initialPrivateState:0,initialZswapLocalState:r.emptyZswapLocalState(key)},...(kind==='point-digest'?[arg(1)]:kind==='point-guard'?[arg(1),arg(2)]:[]));
const args=spec=>argumentsFor(kind,spec,r);
const specs=reviewedCases(kind);
const cases=[];
for(const spec of specs){
 let state=init().currentContractState,privateState=0;
 const invoke=s=>{const out=contract.circuits[s.operation](r.createCircuitContext(r.dummyContractAddress(),key,state.data,privateState),...args(s));state.data=new r.ChargedState(out.context.currentQueryContext.state.state);privateState=out.context.currentPrivateState;return out;};
 admit=true; for(const step of spec.setup??[]) invoke(step);
 const row={...spec,before:Buffer.from(state.serialize()).toString('hex'),privateBefore:privateState};
 if(['point-digest','point-guard','alias-set'].includes(kind)) delete row.setup; delete row.error;
 gas=[];witnesses=[];admit=spec.admit??true;
 try{const out=invoke(spec);Object.assign(row,{after:Buffer.from(state.serialize()).toString('hex'),privateAfter:privateState,result:json(out.result),publicTranscript:json(out.proofData.publicTranscript),privateTranscript:out.proofData.privateTranscriptOutputs.map(v=>({valueAtoms:v.value.map(a=>Array.from(a)),alignment:json(v.alignment)}))});}
 catch(e){row.error=e.message;}
 row.queries=json(gas);row.witnessInputs=json(witnesses);row.admit=admit;
 if(spec.error ? !row.error?.includes(spec.error) : row.error)throw Error(`${spec.id}: unexpected result ${row.error}`);
 cases.push(row);
}
process.stdout.write(JSON.stringify({format:'compact-did-primitive-reducer-capture/v1',kind,generatedJavaScriptSha256:hash(generated),runtimeJavaScriptSha256:hash(resolve(runtimePath,'dist/built-ins.js')),captureSha256:hash(process.argv[1]),runtimeVersion:JSON.parse(readFileSync(resolve(runtimePath,'package.json'),'utf8')).version,provenance:[selected.source,`${selected.fixture}/lib.rs`,`${selected.fixture}/tests/behavior.rs`,`${selected.fixture}/tests/provenance.rs`,'tools/compact-rust-backend/did_primitive_reducers.json','tools/compact-rust-backend/did_primitive_reducer_capture.mjs','tools/compact-rust-backend/did_primitive_reducer_cases.mjs','runtime/src/built-ins.ts','runtime/src/compact-types.ts'].map(path=>({path,sha256:hash(resolve(repo,path))})),cases},null,2)+'\n');
