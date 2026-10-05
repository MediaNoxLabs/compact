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

// Compile counter_less_than_oracle.compact with the TypeScript target.
import { createRequire } from 'node:module';
import { pathToFileURL } from 'node:url';
const [contractPath] = process.argv.slice(2);
const require = createRequire(contractPath);
const runtime = await import(pathToFileURL(require.resolve('@midnight-ntwrk/compact-runtime')));
const { Contract } = await import(pathToFileURL(contractPath));
const contract = new Contract({});
const key = { bytes: new Uint8Array(32) };
const initial = contract.initialState({initialPrivateState: null, initialZswapLocalState: runtime.emptyZswapLocalState(key)});
const bytesHex = (x) => Buffer.from(x).toString('hex');
const cost = (gas) => Object.fromEntries(Object.entries(gas).map(([key,value])=>[key,value.toString()]));
const queries = [];
const originalQuery = runtime.QueryContext.prototype.query;
runtime.QueryContext.prototype.query = function(...args) {
  const result = originalQuery.call(this, ...args);
  queries.push({gasCost:cost(result.gasCost), opTags:args[0].map(op=>typeof op==='string'?op:Object.keys(op)[0])});
  return result;
};
function shape(op) {
  if (typeof op === 'string') return {kind:op};
  if (op.idx) return {kind:'idx',cached:op.idx.cached,pushPath:op.idx.pushPath,pathLength:op.idx.path.length};
  if (op.dup) return {kind:'dup',n:op.dup.n};
  if (op.push) return {kind:'push',storage:op.push.storage,value:op.push.value};
  if (op.popeq) return {kind:'popeq',cached:op.popeq.cached,resultAtoms:op.popeq.result.value.map(x=>Array.from(x))};
  throw new Error('unexpected operation '+Object.keys(op));
}
const rows=[];
for (const [name, args] of [['compare',[0n]],['compare',[3n]],['compare',[4n]],['compare',[(1n<<64n)-1n]],['nested',[]],['short_circuit',[false]],['short_circuit',[true]],['checked',[4n]],['checked',[3n]]]) {
  const deployed=runtime.ContractState.deserialize(initial.currentContractState.serialize());
  const context=runtime.createCircuitContext(runtime.dummyContractAddress(),key,deployed.data,null);
  queries.length=0;
  try {
    const output=contract.circuits[name](context,...args);
    deployed.data=new runtime.ChargedState(output.context.currentQueryContext.state.state);
    rows.push({name,args:args.map(x=>typeof x==='bigint'?x.toString():x),result:Array.isArray(output.result)?null:output.result,
      queries:[...queries],reportedGas:cost(output.gasCost),privateCount:output.proofData.privateTranscriptOutputs.length,
      shape:output.proofData.publicTranscript.map(shape),afterStateHex:bytesHex(deployed.serialize())});
  } catch(error) {
    rows.push({name,args:args.map(String),error:error.message,queries:[...queries]});
  }
}
process.stdout.write(JSON.stringify({initialStateHex:bytesHex(initial.currentContractState.serialize()),rows},(_,v)=>v instanceof Uint8Array?Array.from(v):typeof v==='bigint'?v.toString():v,2)+'\n');
