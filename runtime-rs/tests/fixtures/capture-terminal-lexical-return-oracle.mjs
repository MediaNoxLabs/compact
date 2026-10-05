// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Compile terminal_lexical_return_oracle.compact with --target ts --skip-zk.
// Pass contract/index.js linked to the matching Compact runtime node_modules.
import { createRequire } from 'node:module';
import { pathToFileURL } from 'node:url';
const [path] = process.argv.slice(2);
const require = createRequire(path);
const r = await import(pathToFileURL(require.resolve('@midnight-ntwrk/compact-runtime')));
const { Contract } = await import(pathToFileURL(path));
const queries = [], events = [];
const original = r.QueryContext.prototype.query;
r.QueryContext.prototype.query = function(...args) {
  const q = {ops:args[0]}; queries.push(q); events.push({query:queries.length-1});
  try {const out=original.call(this,...args); q.gas=out.gasCost; return out;}
  catch(error){q.error=String(error);throw error;}
};
function run(name, seed, delta=2, reject=false, zeroGas=false) {
  const trace=[];
  const c=new Contract({next_value:(ctx,value)=>{
    const event={seed:value.toString(),prior:[...ctx.privateState]};
    trace.push(event); events.push(event);
    return [[...ctx.privateState,value.toString()],BigInt(delta)];
  }});
  const key={bytes:new Uint8Array(32)};
  const init=c.initialState({initialPrivateState:[],initialZswapLocalState:r.emptyZswapLocalState(key)});
  // Seed through the real source's echo method; capture only the next call.
  for(let i=0;i<seed;i++) {
    const ctx=r.createCircuitContext(r.decodeContractAddress(key.bytes),key,init.currentContractState.data,[]);
    const out=c.circuits.echo(ctx,0n);
    init.currentContractState.data=new r.ChargedState(out.context.currentQueryContext.state.state);
  }
  const before=Buffer.from(init.currentContractState.serialize()).toString('hex');
  const ctx=r.createCircuitContext(r.decodeContractAddress(key.bytes),key,init.currentContractState.data,[]);
  if(zeroGas)ctx.gasLimit=r.emptyRunningCost();
  queries.length=0;events.length=0;
  const row={name,seed,delta,reject,zeroGas,before};
  try {
    const args=name==='echo'?[91n]:name==='observed'?[reject]:[];
    const out=c.circuits[name](ctx,...args);
    const state=r.ContractState.deserialize(init.currentContractState.serialize());
    state.data=new r.ChargedState(out.context.currentQueryContext.state.state);
    Object.assign(row,{result:out.result,after:Buffer.from(state.serialize()).toString('hex'),
      effects:out.context.currentQueryContext.effects,privateState:out.context.currentPrivateState,
      privateOutputs:out.proofData.privateTranscriptOutputs,output:out.proofData.output,
      gas:out.gasCost,publicTranscript:out.proofData.publicTranscript});
  }catch(error){row.error=String(error);}
  return {...row,trace,queries:[...queries],events:[...events]};
}
const rows=[];
for(const seed of [0,4])for(const name of ['two','three','echo','nested','observed'])rows.push(run(name,seed));
rows.push(run('observed',4,0),run('observed',4,2,true),run('observed',4,2,false,true));
process.stdout.write(JSON.stringify(rows,(_,x)=>x instanceof Map?Object.fromEntries(x)
 :x instanceof Uint8Array?Array.from(x):typeof x==='bigint'?String(x):x,2)+'\n');
