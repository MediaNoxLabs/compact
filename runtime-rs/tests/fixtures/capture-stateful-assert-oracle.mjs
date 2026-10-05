// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//  	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
import { createRequire } from 'node:module';
import { pathToFileURL } from 'node:url';
const [path] = process.argv.slice(2);
const require = createRequire(path);
const r = await import(pathToFileURL(require.resolve('@midnight-ntwrk/compact-runtime')));
const { Contract } = await import(pathToFileURL(path));
const events = [], queries = [];
const original = r.QueryContext.prototype.query;
r.QueryContext.prototype.query = function(...args) {
  const query = {ops:args[0]};queries.push(query);events.push({query:queries.length-1});
  try {const out=original.call(this,...args);query.gas=out.gasCost;return out;}
  catch(error){query.error=String(error);throw error;}
};
function run(name, selected, open, low, high, gates, zeroGas=false, gasBudget=null) {
  const trace=[];
  const c = new Contract({next_gate:(ctx,tag)=>{
    const event={tag:Number(tag),prior:[...ctx.privateState]};trace.push(event);events.push(event);
    return [[...ctx.privateState,Number(tag)],gates[Number(tag)-1]];
  }});
  const key={bytes:new Uint8Array(32)};
  const init=c.initialState({initialPrivateState:[],initialZswapLocalState:r.emptyZswapLocalState(key)},open,BigInt(low),BigInt(high));
  const before=Buffer.from(init.currentContractState.serialize()).toString('hex');
  const ctx=r.createCircuitContext(r.decodeContractAddress(key.bytes),key,init.currentContractState.data,[]);
  if(zeroGas) ctx.gasLimit=r.emptyRunningCost();
  if(gasBudget) ctx.gasLimit=gasBudget;
  queries.length=0;events.length=0;
  const row={name,selected,open,low,high,gates,zeroGas,gasBudget,before};
  try {
    const out=c.circuits[name](ctx,selected);
    const state=r.ContractState.deserialize(init.currentContractState.serialize());
    state.data=new r.ChargedState(out.context.currentQueryContext.state.state);
    Object.assign(row,{result:out.result,after:Buffer.from(state.serialize()).toString('hex'),
      effects:out.context.currentQueryContext.effects,privateState:out.context.currentPrivateState,
      privateOutputs:out.proofData.privateTranscriptOutputs,output:out.proofData.output,gas:out.gasCost, publicTranscript:out.proofData.publicTranscript});
    const replay = r.createCircuitContext(r.decodeContractAddress(key.bytes),key,init.currentContractState.data,[]);
    row.replayGas = original.call(replay.currentQueryContext, queries.flatMap(q=>q.ops), replay.costModel).gasCost;
  } catch(error){row.error=String(error);}
  return {...row,trace,queries:[...queries],events:[...events]};
}
const all=[true,true,true];
const rows=[
 run('checked',true,true,3,7,all),run('checked',false,true,3,7,all),
 run('checked',true,true,3,7,[false,true,true]),run('checked',true,false,3,7,all),
 run('checked',true,true,3,7,[true,false,true]),run('checked',true,true,9,7,all),
 run('checked',true,true,3,7,[true,true,false]),
 run('checked',false,true,3,7,all,true),run('checked',true,true,3,7,all,true),
 run('unit_result',true,true,3,7,all),run('unit_result',false,true,3,7,all),
 run('unit_result',true,false,3,7,all),
];
// Both runtimes apply this limit per query. A read fits; the costlier lessThan query fails.
for (const count of [1]) {
  const budget=r.emptyRunningCost();
  for (const query of rows[0].queries.slice(0,count))
    for (const key of Object.keys(budget)) budget[key]+=query.gas[key];
  rows.push(run('checked',true,true,3,7,all,false,budget));
}
process.stdout.write(JSON.stringify(rows,(_,x)=>x instanceof Map?Object.fromEntries(x)
 : x instanceof Uint8Array?Array.from(x):typeof x==='bigint'?String(x):x,2)+'\n');
