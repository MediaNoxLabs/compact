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

import { readFileSync, mkdirSync, symlinkSync, existsSync, writeFileSync } from 'node:fs';
import { resolve, dirname } from 'node:path';
import { pathToFileURL } from 'node:url';
import { createHash } from 'node:crypto';
const [kind, generatedPath, outputPath] = process.argv.slice(2);
if(!kind || !generatedPath || !outputPath) throw Error('kind generated output required');
const sourceRoot=resolve(import.meta.dirname,'../..');
const runtimePath=resolve(sourceRoot,'runtime');
const generated=resolve(generatedPath);
const modulePath=resolve(dirname(generated),'node_modules/@midnight-ntwrk/compact-runtime');
if(!existsSync(modulePath)) { mkdirSync(dirname(modulePath),{recursive:true}); symlinkSync(runtimePath,modulePath); }
const r=await import(pathToFileURL(resolve(runtimePath,'dist/index.js')).href);
const c=await import(pathToFileURL(generated).href);
const json=value=>JSON.parse(JSON.stringify(value,(_,v)=>typeof v==='bigint'?String(v):v instanceof Uint8Array?Array.from(v):v));
const hash=path=>createHash('sha256').update(readFileSync(path)).digest('hex');
const key={bytes:new Uint8Array(32)};
const contract=new c.Contract({});
const specs={
 two:[
  {id:'authentication-insert-unicode',operation:'update',relation:1,key:'référence-東京',add:true},
  {id:'authentication-duplicate',operation:'update',relation:1,key:'référence-東京',add:true,setup:[{operation:'update',relation:1,key:'référence-東京',add:true}],error:'already present'},
  {id:'authentication-remove',operation:'update',relation:1,key:'référence-東京',add:false,setup:[{operation:'update',relation:1,key:'référence-東京',add:true}]},
  {id:'authentication-missing',operation:'update',relation:1,key:'absent',add:false,error:'missing relation'},
  {id:'agreement-insert-same-key',operation:'update',relation:2,key:'référence-東京',add:true,setup:[{operation:'update',relation:1,key:'référence-東京',add:true}]},
  {id:'undefined-before-read',operation:'update',relation:0,key:'absent',add:true,error:'undefined relation'},
 ],
 four:[
  ...[1,2,3,4].flatMap(relation=>[
   {id:`branch-${relation}-insert`,operation:'update',relation,key:`id-${relation}`,add:true},
   {id:`branch-${relation}-remove`,operation:'update',relation,key:`id-${relation}`,add:false,setup:[{operation:'update',relation,key:`id-${relation}`,add:true}]},
  ]),
  {id:'undefined-before-read',operation:'update',relation:0,key:'absent',add:true,error:'undefined relation'},
  {id:'branch-4-duplicate',operation:'update',relation:4,key:'same',add:true,setup:[{operation:'update',relation:4,key:'same',add:true}],error:'already present'},
 ],
 nested:[
  {id:'missing-signing-short-circuit',operation:'check',key:'missing',agreement:false},
  {id:'missing-agreement',operation:'check',key:'missing',agreement:true,error:'missing method'},
  {id:'x25519-agreement',operation:'check',key:'x',agreement:true,setup:[{operation:'seed',key:'x',curve:1}]},
  {id:'ed25519-agreement-rejected',operation:'check',key:'ed',agreement:true,setup:[{operation:'seed',key:'ed',curve:0}],error:'wrong curve'},
  {id:'ed25519-signing',operation:'check',key:'ed',agreement:false,setup:[{operation:'seed',key:'ed',curve:0}]},
  {id:'x25519-signing-rejected',operation:'check',key:'x',agreement:false,setup:[{operation:'seed',key:'x',curve:1}],error:'wrong signing curve'},
 ]
};
if(!specs[kind])throw Error('unknown kind');
const args=s=>s.operation==='seed'?[{id:s.key,publicKeyJwk:{curve:s.curve,x:'x'}}]:kind==='nested'?[s.key,s.agreement]:[s.relation,s.key,s.add];
let gas=[];
const query=r.QueryContext.prototype.query;
r.QueryContext.prototype.query=function(...a){const out=query.apply(this,a);gas.push(json(out.gasCost));return out;};
const rows=[];
for(const spec of specs[kind]){
 const constructorMethod=kind==='nested'
  ? (()=>{const seeded=spec.setup?.find(s=>s.operation==='seed');return {id:seeded?.key??'constructor-control',publicKeyJwk:{curve:seeded?.curve??0,x:'x'}};})()
  : undefined;
 let snapshot=kind==='nested'
  ? contract.initialState({initialPrivateState:0,initialZswapLocalState:r.emptyZswapLocalState(key)},constructorMethod)
  : contract.initialState({initialPrivateState:0,initialZswapLocalState:r.emptyZswapLocalState(key)});
 let state=snapshot.currentContractState,privateState=0;
 const invoke=s=>{
  const out=contract.circuits[s.operation](r.createCircuitContext(r.dummyContractAddress(),key,state.data,privateState),...args(s));
  state.data=new r.ChargedState(out.context.currentQueryContext.state.state);
  privateState=out.context.currentPrivateState;
  return out;
 };
 for(const step of spec.setup??[])if(kind!=='nested')invoke(step);
 const row={...spec,before:Buffer.from(state.serialize()).toString('hex'),privateBefore:privateState};
 gas=[];
 try{
  const out=invoke(spec);
  Object.assign(row,{after:Buffer.from(state.serialize()).toString('hex'),privateAfter:privateState,result:json(out.result),publicTranscript:json(out.proofData.publicTranscript),privateTranscript:out.proofData.privateTranscriptOutputs.map(v=>({valueAtoms:v.value.map(a=>Array.from(a)),alignment:json(v.alignment)}))});
 }catch(e){row.error=e.message;row.after=Buffer.from(state.serialize()).toString('hex');row.privateAfter=privateState;}
 row.queries=json(gas);
 if(spec.error?!row.error?.includes(spec.error):!!row.error)throw Error(`${spec.id}: unexpected ${row.error}`);
 rows.push(row);
}
const result={format:'compact-did-relation-reducer-capture/v1',kind,sourceSha256:hash(resolve(sourceRoot,`examples/rust_backend/did_relation_${kind}_reducer.compact`)),generatedJavaScriptSha256:hash(generated),runtimeVersion:JSON.parse(readFileSync(resolve(runtimePath,'package.json'),'utf8')).version,runtimeBuiltinsSha256:hash(resolve(runtimePath,'dist/built-ins.js')),cases:rows};
writeFileSync(outputPath,JSON.stringify(result,null,2)+'\n');
console.log(kind,rows.length,hash(outputPath));
