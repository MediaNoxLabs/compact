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

import {readFileSync, writeFileSync, realpathSync, existsSync, mkdirSync, symlinkSync, readdirSync, statSync} from 'node:fs';
import {createHash} from 'node:crypto';
import {createRequire} from 'node:module';
import {fileURLToPath, pathToFileURL} from 'node:url';
import {dirname, resolve} from 'node:path';
import assert from 'node:assert/strict';
import {parseArgs} from 'node:util';
// --reference rechecks retained original-0.35 expectations with fresh ledger8 TS.
// Without it, --oracle executes the original generated pure circuits directly.
const {values:options}=parseArgs({options:{...Object.fromEntries(['generated','oracle','reference','boundaries','source','original-source','oracle-packages','provenance'].map(name=>[name,{type:'string'}])), 'runtime-inventory':{type:'boolean'}}});
const repo=resolve(dirname(fileURLToPath(import.meta.url)),'../..');
const runtimeRoot=resolve(repo,'runtime');
const hash=path=>createHash('sha256').update(readFileSync(path)).digest('hex');
function runtimeInventory() {
 const files=new Set(), packages=new Set();
 function walk(folder) {
  for(const entry of readdirSync(folder,{withFileTypes:true})) {
   if(['node_modules','.git'].includes(entry.name)) continue;
   const path=resolve(folder,entry.name);
   if(entry.isDirectory()) walk(path);
   else if(statSync(path).isFile()) files.add(realpathSync(path));
  }
 }
 function visit(folder) {
  folder=realpathSync(folder);
  if(packages.has(folder)) return;
  packages.add(folder); walk(folder);
  const manifest=resolve(folder,'package.json');
  const req=createRequire(manifest);
  for(const name of Object.keys(JSON.parse(readFileSync(manifest,'utf8')).dependencies??{})) {
   const candidate=req.resolve.paths(name).map(path=>resolve(path,name,'package.json')).find(existsSync);
   assert.ok(candidate,`missing runtime dependency ${name}`); visit(dirname(candidate));
  }
 }
 visit(runtimeRoot);
 return Object.fromEntries([...files].sort().map(path=>[path,hash(path)]));
}
if(options['runtime-inventory']) {
 console.log(JSON.stringify(runtimeInventory(),null,2));
 process.exit(0);
}
for(const name of ['generated','boundaries','source','provenance']) assert.ok(options[name],`missing --${name}`);
assert.ok(Boolean(options.reference)!==Boolean(options.oracle),'select exactly one of --reference or --oracle');
if(!options.reference) for(const name of ['original-source','oracle-packages']) assert.ok(options[name],`missing --${name}`);
const wrapper=resolve(options.generated);
const modulePath=resolve(dirname(wrapper),'node_modules/@midnight-ntwrk/compact-runtime');
if(!existsSync(modulePath)) {mkdirSync(dirname(modulePath),{recursive:true});symlinkSync(runtimeRoot,modulePath);}
assert.equal(realpathSync(modulePath),realpathSync(runtimeRoot),'wrong ledger8 runtime');
const runtimePath=createRequire(pathToFileURL(wrapper)).resolve('@midnight-ntwrk/compact-runtime');
const r=await import(pathToFileURL(runtimePath));
const c=await import(pathToFileURL(wrapper));
const oracle=options.oracle?resolve(options.oracle):undefined;
const originalRuntimePath=oracle?createRequire(pathToFileURL(oracle)).resolve('@midnight-ntwrk/compact-runtime'):undefined;
const originalRuntime=oracle?await import(pathToFileURL(originalRuntimePath)):r;
const p=oracle?(await import(pathToFileURL(oracle))).pureCircuits:undefined;
const reference=options.reference?JSON.parse(readFileSync(resolve(options.reference),'utf8')):undefined;
const runtimeHashes=runtimeInventory();
const boundaries=JSON.parse(readFileSync(resolve(options.boundaries),'utf8'));
assert.equal(boundaries.values.length,7);
assert.deepEqual(boundaries.values.map(v=>v.name),['zero','one','eight','q_minus_one','q','q_plus_one','native_max']);
const json=value=>JSON.parse(JSON.stringify(value,(_,v)=>typeof v==='bigint'?String(v):v instanceof Uint8Array?Array.from(v):v instanceof Map?Array.from(v.entries()):v));
const hex=x=>x.toString(16).padStart(64,'0').match(/../g).reverse().join('');
const point=(q,runtime)=>({x:hex(q.x),y:hex(q.y),fab:runtime.CompactTypeJubjubPoint.toValue(q).map(v=>typeof v==='bigint'?hex(v):Buffer.from(v).toString('hex'))});
// Decode retained point coordinates, then verify their independent FAB encoding.
const decodePoint=value=>{
 const decode=hex=>{assert.match(hex,/^[0-9a-f]{64}$/);return BigInt('0x'+hex.match(/../g).reverse().join(''));};
 const decoded={x:decode(value.x),y:decode(value.y)};
 assert.deepEqual(point(decoded,r),value,'retained point/FAB mismatch');
 return decoded;
};
if(reference) {
 assert.equal(reference.format,'compact-acc-jubjub-wrapper-capture/v1');
 assert.equal(reference.kind,'acc-jubjub-cell');
 assert.deepEqual(reference.cases.map(v=>v.id),boundaries.values.map(v=>v.name));
 assert.deepEqual(reference.rawCases.map(v=>v.id),['raw-q_minus_one','raw-q']);
 for(const [index,row] of reference.cases.entries()) {
  assert.equal(row.operation,'apply');
  assert.equal(row.scalar,boundaries.values[index].hex_le);
  assert.equal(row.scalarDecimal,boundaries.values[index].decimal);
  assert.deepEqual(row.point,reference.cases[0].point);
 }
}
const key={bytes:new Uint8Array(32)};
const contract=new c.Contract({});
const supplied=reference?decodePoint(reference.cases[0].point):p.generator(7n);
assert.deepEqual(point(supplied,originalRuntime),point(r.ecMulGenerator(7n),r));
let active;
const query=r.QueryContext.prototype.query;
r.QueryContext.prototype.query=function(...args){
 const beforeEffects=json(this.effects);
 const result=query.apply(this,args);
 if(active)active.push({program:json(args[0]),gasCost:json(result.gasCost),events:json(result.events),beforeEffects,afterEffects:json(result.context.effects)});
 return result;
};
const cases=[],refusals=[],rawCases=[];
try {
 for(const boundary of boundaries.values){
  const scalar=BigInt(boundary.decimal);
  // The original 0.35 circuits perform the cast/reduction themselves; do not
  // call ledger8 reduction to construct the expected point.
  const retained=reference?.cases.find(row=>row.id===boundary.name);
  const generator=retained?decodePoint(retained.oracleGenerator):p.generator(scalar);
  const product=retained?decodePoint(retained.oracleProduct):p.multiply(supplied,scalar);
  const expected=retained?decodePoint(retained.expected):originalRuntime.ecAdd(generator,product);
  const initial=contract.initialState({initialPrivateState:17,initialZswapLocalState:r.emptyZswapLocalState(key)});
  const state=initial.currentContractState;
  const before=Buffer.from(state.serialize()).toString('hex');
  const context=r.createCircuitContext(r.dummyContractAddress(),key,state.data,initial.currentPrivateState);
  const beforeEffects=json(context.currentQueryContext.effects);
  active=[];
  const out=contract.circuits.apply(context,supplied,scalar);
  const queryTrace=active;active=undefined;
  state.data=new r.ChargedState(out.context.currentQueryContext.state.state);
  const result=point(out.result,r);
  assert.deepEqual(result,point(expected,originalRuntime),boundary.name);
  assert.deepEqual(point(c.ledger(state.data).result,r),result,`${boundary.name}: Cell result`);
  assert.equal(out.context.currentPrivateState,17);
  assert.equal(queryTrace.length,1,`${boundary.name}: single checked Cell write`);
  assert.deepEqual(json(out.proofData.privateTranscriptOutputs),[]);
  const queries=queryTrace.map(q=>q.gasCost);
  const totalGas=Object.fromEntries(['readTime','computeTime','bytesWritten','bytesDeleted'].map(k=>[k,queries.reduce((sum,q)=>sum+BigInt(q[k]),0n).toString()]));
  assert.deepEqual(json(out.gasCost),totalGas);
  assert.deepEqual(queryTrace.flatMap(q=>q.program),json(out.proofData.publicTranscript));
  cases.push({id:boundary.name,operation:'apply',scalar:boundary.hex_le,scalarDecimal:boundary.decimal,
   point:point(supplied,r),oracleGenerator:point(generator,originalRuntime),oracleProduct:point(product,originalRuntime),expected:point(expected,originalRuntime),result,
   before,after:Buffer.from(state.serialize()).toString('hex'),privateBefore:17,privateAfter:out.context.currentPrivateState,
   effectsBefore:beforeEffects,effectsAfter:json(out.context.currentQueryContext.effects),
   publicTranscript:json(out.proofData.publicTranscript),privateTranscript:json(out.proofData.privateTranscriptOutputs),
   publicInput:json(out.proofData.input),publicOutput:json(out.proofData.output),queries,totalGas,queryTrace,witnessInputs:[]});
 }
 const q=BigInt(boundaries.jubjub_order);
 for(const [id,call] of [['raw-q-generator',()=>r.ecMulGenerator(q)],['raw-q-point',()=>r.ecMul(supplied,q)]]){
  active=[];let error;
  try{call();}catch(e){error=String(e);}
  assert.ok(error,`${id}: canonical primitive must refuse q`);
  assert.equal(active.length,0);
  refusals.push({id,error,queries:active,scope:'raw canonical primitive; no wrapper call or accepted mutation'});active=undefined;
 }
 for(const [id,scalar] of [['negative',-1n],['native-modulus',BigInt(boundaries.native_modulus)],['number-not-bigint',1]]){
  const initial=contract.initialState({initialPrivateState:17,initialZswapLocalState:r.emptyZswapLocalState(key)});
  const state=initial.currentContractState;
  const before=Buffer.from(state.serialize()).toString('hex');
  const context=r.createCircuitContext(r.dummyContractAddress(),key,state.data,17);
  active=[];let error;
  try{contract.circuits.apply(context,supplied,scalar);}catch(e){error=String(e);}
  assert.ok(error,`${id}: input must refuse`);assert.equal(active.length,0);
  const after=Buffer.from(state.serialize()).toString('hex');assert.equal(before,after);
  assert.equal(context.currentPrivateState,17);
  refusals.push({id,error,before,after,privateBefore:17,privateAfter:context.currentPrivateState,queries:active,scope:'ledger8 TypeScript wrapper input validation'});active=undefined;
 }
 if(contract.circuits.raw){
  for(const id of ['q_minus_one','q']){
   const boundary=boundaries.values.find(v=>v.name===id),scalar=BigInt(boundary.decimal);
   const initial=contract.initialState({initialPrivateState:17,initialZswapLocalState:r.emptyZswapLocalState(key)});
   const state=initial.currentContractState;
   const before=Buffer.from(state.serialize()).toString('hex');
   const context=r.createCircuitContext(r.dummyContractAddress(),key,state.data,17);
   const beforeEffects=json(context.currentQueryContext.effects);
   active=[];let out,error;
   try{out=contract.circuits.raw(context,supplied,scalar);}catch(e){error=String(e);}
   const queryTrace=active;active=undefined;
   const row={id:`raw-${id}`,operation:'raw',scalar:boundary.hex_le,scalarDecimal:boundary.decimal,point:point(supplied,r),before,privateBefore:17,effectsBefore:beforeEffects,witnessInputs:[],queries:queryTrace.map(q=>q.gasCost),queryTrace};
   if(id==='q'){
    assert.ok(error,'generated raw q must refuse');assert.equal(queryTrace.length,0);
    Object.assign(row,{error,after:Buffer.from(state.serialize()).toString('hex'),privateAfter:context.currentPrivateState,effectsAfter:json(context.currentQueryContext.effects)});
    assert.equal(row.before,row.after);assert.equal(row.privateAfter,17);assert.deepEqual(row.effectsBefore,row.effectsAfter);
   }else{
    assert.ok(out,error);assert.equal(queryTrace.length,1);
    state.data=new r.ChargedState(out.context.currentQueryContext.state.state);
    const expected=reference?point(decodePoint(reference.rawCases.find(row=>row.id===`raw-${id}`).expected),r):point(p.multiply(supplied,scalar),originalRuntime),result=point(out.result,r);
    assert.deepEqual(result,expected);assert.equal(out.context.currentPrivateState,17);
    const totalGas=Object.fromEntries(['readTime','computeTime','bytesWritten','bytesDeleted'].map(k=>[k,row.queries.reduce((sum,q)=>sum+BigInt(q[k]),0n).toString()]));
    Object.assign(row,{expected,result,after:Buffer.from(state.serialize()).toString('hex'),privateAfter:out.context.currentPrivateState,effectsAfter:json(out.context.currentQueryContext.effects),publicTranscript:json(out.proofData.publicTranscript),privateTranscript:json(out.proofData.privateTranscriptOutputs),publicInput:json(out.proofData.input),publicOutput:json(out.proofData.output),totalGas});
   }
   rawCases.push(row);
  }
 }
} finally {r.QueryContext.prototype.query=query;}
const identityPaths=[fileURLToPath(import.meta.url),resolve(options.source),resolve(options.boundaries),wrapper,runtimePath];
if(reference) identityPaths.push(resolve(options.reference));
else identityPaths.push(resolve(options['original-source']),oracle,originalRuntimePath,resolve(dirname(originalRuntimePath),'built-ins.js'),resolve(options['oracle-packages']));
assert.deepEqual(runtimeInventory(),runtimeHashes,'runtime closure changed during capture');
writeFileSync(resolve(options.provenance),JSON.stringify({format:'compact-acc-jubjub-wrapper-provenance/v1',oracleMode:reference?'retained-original-reference':'live-original',command:process.argv,node:process.version,runtime:{path:realpathSync(runtimePath),...(originalRuntimePath?{originalPath:realpathSync(originalRuntimePath)}:{})},runtimeHashes,inputs:identityPaths.map(path=>({path,sha256:hash(path)}))},null,2)+'\n');
const result={format:'compact-acc-jubjub-wrapper-capture/v1',kind:'acc-jubjub-cell',oracle:'original Compact0.35/runtime0.20 pure cast circuits plus original runtime ecAdd; supplied point generator(7)',cases,refusals,rawCases};
if(reference) assert.deepEqual(result,reference,'fresh ledger8 TS differs from retained original-reference observations');
console.log(JSON.stringify(result,null,2));
