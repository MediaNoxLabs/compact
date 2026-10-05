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

// Independent original planned / funded-transfer primitive ordering captures.
import { createRequire } from 'node:module';
import { pathToFileURL } from 'node:url';
const [path, name] = process.argv.slice(2);
const require = createRequire(path);
const r = await import(pathToFileURL(require.resolve('@midnight-ntwrk/compact-runtime')));
const { Contract } = await import(pathToFileURL(path));
const bytes = n => Uint8Array.from([n, ...Array(31).fill(0)]);
const key = {bytes: bytes(0)};
const address = r.decodeContractAddress(bytes(9));
const gas = value => Object.fromEntries(Object.entries(value).map(([k,v]) => [k,String(v)]));
const queries = [];
const original = r.QueryContext.prototype.query;
r.QueryContext.prototype.query = function (...args) {
  const out = original.call(this, ...args);
  queries.push({ops: args[0], gas: gas(out.gasCost)});
  return out;
};
function run(left, value, start, privateState) {
  const c = new Contract({next_value: (ctx, tag) => [
    [...ctx.privateState, Number(tag)], tag * 10n + BigInt(ctx.privateState.length),
  ]});
  const init = c.initialState({initialPrivateState: privateState, initialZswapLocalState: r.emptyZswapLocalState(key)});
  const context = r.createCircuitContext(address, key, init.currentContractState.data, privateState);
  context.currentZswapLocalState.currentIndex = start;
  const coin = {nonce: bytes(7), color: bytes(8), value};
  const recipient = {is_left: left, left: {bytes: bytes(left ? 5 : 0)}, right: {bytes: bytes(left ? 0 : 9)}};
  const input = {...coin, nonce: bytes(6), mt_index: 3n};
  const args = name === 'planned' ? [coin,recipient] : [input,coin,recipient,bytes(10),bytes(11)];
  const before = Buffer.from(init.currentContractState.serialize()).toString('hex');
  queries.length = 0;
  const out = c.circuits[name](context,...args);
  const captured = [...queries];
  const state = r.ContractState.deserialize(init.currentContractState.serialize());
  state.data = new r.ChargedState(out.context.currentQueryContext.state.state);
  const replayContext = r.createCircuitContext(address,key,init.currentContractState.data,privateState);
  const replay = original.call(replayContext.currentQueryContext,captured.flatMap(query=>query.ops),replayContext.costModel);
  return {name,left,value,start,initialPrivateState:privateState,args,before,after:Buffer.from(state.serialize()).toString('hex'),result:out.result,output:out.proofData.output,input:out.proofData.input,privateState:out.context.currentPrivateState,privateOutputs:out.proofData.privateTranscriptOutputs,publicTranscript:out.proofData.publicTranscript,queries:captured,replayGas:gas(replay.gasCost),plan:out.context.currentZswapLocalState,effects:out.context.currentQueryContext.effects};
}
const rows = name === 'planned'
  ? [run(true,0n,0n,[]),run(true,42n,7n,[]),run(false,42n,7n,[]),run(false,0n,11n,[99])]
  : [run(true,42n,7n,[]),run(false,42n,11n,[99])];
process.stdout.write(JSON.stringify(rows,(_,x) => x instanceof Map ? Object.fromEntries(x) : x instanceof Uint8Array ? Array.from(x) : typeof x === 'bigint' ? String(x) : x,2)+'\n');
