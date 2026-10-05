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
// Independent original-source TS token query, including exact metered public VM.
import { createRequire } from 'node:module';
import { pathToFileURL } from 'node:url';
const [path] = process.argv.slice(2);
const require = createRequire(path);
const r = await import(pathToFileURL(require.resolve('@midnight-ntwrk/compact-runtime')));
const { Contract } = await import(pathToFileURL(path));
const bytes = n => Uint8Array.from([n, ...Array(31).fill(0)]);
const key = { bytes: bytes(0) };
const gas = v => Object.fromEntries(Object.entries(v).map(([k,x]) => [k,String(x)]));
const queries = [];
const original = r.QueryContext.prototype.query;
r.QueryContext.prototype.query = function(...args) {
 const out = original.call(this,...args);
 queries.push({ops: args[0],gas: gas(out.gasCost)});
 return out;
};
function run(address) {
 const unexpected = () => { throw new Error('token query must not call a witness'); };
 const c = new Contract(Object.fromEntries(['local_secret_key','local_state','local_advance_state','local_record_vote','local_vote_cast','local_path_of_cm'].map(n=>[n,unexpected])));
 const init = c.initialState({initialPrivateState:[77],initialZswapLocalState:r.emptyZswapLocalState(key)}, bytes(4), {seed_dust:10n,buy_in_dust:3n});
 const addr = r.decodeContractAddress(bytes(address));
 const before = Buffer.from(init.currentContractState.serialize()).toString('hex');
 const ctx = r.createCircuitContext(addr,key,init.currentContractState.data,[77]);
 queries.length = 0;
 const out = c.circuits.dao_voting_token(ctx);
 const state = r.ContractState.deserialize(init.currentContractState.serialize());
 state.data = new r.ChargedState(out.context.currentQueryContext.state.state);
 const replay = r.createCircuitContext(addr,key,init.currentContractState.data,[77]);
 const replayGas = gas(original.call(replay.currentQueryContext,queries.flatMap(q=>q.ops),replay.costModel).gasCost);
 return {address,before,after:Buffer.from(state.serialize()).toString('hex'),result:out.result,output:out.proofData.output,privateState:out.context.currentPrivateState,privateOutputs:out.proofData.privateTranscriptOutputs,effects:out.context.currentQueryContext.effects,gas:gas(out.gasCost),queries:[...queries],publicTranscript:out.proofData.publicTranscript,replayGas};
}
process.stdout.write(JSON.stringify([9,10,255].map(run),(_,x)=>x instanceof Map?Object.fromEntries(x):x instanceof Uint8Array?Array.from(x):typeof x==='bigint'?String(x):x,2)+'\n');
