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

// Independent provisional TypeScript two-output intent/Kernel/qualified Cell execution.
// Canonical offer indices are a separate Rust ledger-bound mode; this captures raw source order.
import {createRequire} from 'node:module';
import {readFileSync} from 'node:fs';
import {createHash} from 'node:crypto';
import {pathToFileURL} from 'node:url';
const [path]=process.argv.slice(2);const require=createRequire(path);
const r=await import(pathToFileURL(require.resolve('@midnight-ntwrk/compact-runtime')));
const {Contract}=await import(pathToFileURL(path));
const generatedRuntimeVersion=readFileSync(path,'utf8').match(/checkRuntimeVersion\('([^']+)'\)/)?.[1];
const runtimePackageVersion=JSON.parse(readFileSync(require.resolve('@midnight-ntwrk/compact-runtime/package.json'),'utf8')).version;
if(generatedRuntimeVersion!==runtimePackageVersion)throw new Error('runtime version mismatch');
const runtimeTypesSha256=createHash('sha256').update(readFileSync(new URL('./compact-types.js',pathToFileURL(require.resolve('@midnight-ntwrk/compact-runtime'))))).digest('hex');
const provenance={source:'examples/rust_backend/canonical_output_order_oracle.compact',generatedRuntimeVersion,runtimePackageVersion,runtimeTypesSha256,allocation:'provisional source order; canonical offer execution tested separately'};
const key={bytes:new Uint8Array(32)},address=r.dummyContractAddress();
const coin={nonce:Uint8Array.from([3,...Array(31).fill(0)]),color:Uint8Array.from([2,...Array(31).fill(0)]),value:17n};
const recipient=left=>({is_left:left,left:{bytes:Uint8Array.from([left?7:0,...Array(31).fill(0)])},right:key});
const change={...coin,nonce:Uint8Array.from([4,...Array(31).fill(0)]),value:25n};
const input={...coin,value:42n,nonce:Uint8Array.from([1,...Array(31).fill(0)]),mt_index:3n};
const gas=x=>Object.fromEntries(Object.entries(x).map(([k,v])=>[k,String(v)]));
const queries=[];const original=r.QueryContext.prototype.query;
r.QueryContext.prototype.query=function(...args){const out=original.call(this,...args);queries.push({ops:args[0],gas:gas(out.gasCost)});return out;};
function run(name,left,selected=true,start=7n){
 const c=new Contract({next_coin:ctx=>[[...ctx.privateState,'coin'],coin],next_recipient:ctx=>[[...ctx.privateState,'recipient'],recipient(left)]});
 const initial=c.initialState({initialPrivateState:[],initialZswapLocalState:r.emptyZswapLocalState(key)});
 const ctx=r.createCircuitContext(address,key,initial.currentContractState.data,[]);ctx.currentZswapLocalState.currentIndex=start;
 const before=Buffer.from(initial.currentContractState.serialize()).toString('hex');queries.length=0;
 const args=[input,coin,change,recipient(left),recipient(false),Uint8Array.from([5,...Array(31).fill(0)]),Uint8Array.from([6,...Array(31).fill(0)]),Uint8Array.from([8,...Array(31).fill(0)])];
 const out=c.circuits[name](ctx,...args);const state=r.ContractState.deserialize(initial.currentContractState.serialize());state.data=new r.ChargedState(out.context.currentQueryContext.state.state);
 const result={name,left,selected,start:String(start),before,after:Buffer.from(state.serialize()).toString('hex'),plan:out.context.currentZswapLocalState,privateState:out.context.currentPrivateState,privateOutputs:out.proofData.privateTranscriptOutputs,queries:[...queries],effects:out.context.currentQueryContext.effects,gas:gas(out.gasCost)};
 if(name==='distribute')result.qualifiedIndex=String(c.circuits.read_coin(out.context).result.mt_index);
 return result;
}
const rows=[run('distribute',true,true,7n),run('distribute',false,true,7n)];
process.stdout.write(JSON.stringify({provenance,rows},(_,x)=>x instanceof Uint8Array?Array.from(x):typeof x==='bigint'?String(x):x,2)+'\n');
