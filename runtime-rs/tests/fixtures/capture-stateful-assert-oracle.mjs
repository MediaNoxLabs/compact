// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
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
function run(name, selected, open, low, high, gates, zeroGas=false) {
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
  queries.length=0;events.length=0;
  const row={name,selected,open,low,high,gates,zeroGas,before};
  try {
    const out=c.circuits[name](ctx,selected);
    const state=r.ContractState.deserialize(init.currentContractState.serialize());
    state.data=new r.ChargedState(out.context.currentQueryContext.state.state);
    Object.assign(row,{result:out.result,after:Buffer.from(state.serialize()).toString('hex'),
      effects:out.context.currentQueryContext.effects,privateState:out.context.currentPrivateState,
      privateOutputs:out.proofData.privateTranscriptOutputs,output:out.proofData.output,gas:out.gasCost});
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
process.stdout.write(JSON.stringify(rows,(_,x)=>x instanceof Map?Object.fromEntries(x)
 : x instanceof Uint8Array?Array.from(x):typeof x==='bigint'?String(x):x,2)+'\n');
