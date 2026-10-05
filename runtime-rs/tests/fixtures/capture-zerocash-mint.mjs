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


// Compile original Zerocash source to TS, link its runtime, pass contract/index.js.
import { createRequire } from 'node:module';
import { pathToFileURL } from 'node:url';
import { normalizeQueryProgram } from '../../../tools/compact-rust-backend/oracles/normalize_query_program.mjs';
const [contractPath] = process.argv.slice(2);
const runtime = await import(pathToFileURL(createRequire(contractPath).resolve('@midnight-ntwrk/compact-runtime')));
const { Contract } = await import(pathToFileURL(contractPath));
const coin = {nonce: {bytes: new Uint8Array(32).fill(3)}, opening: {bytes: new Uint8Array(32).fill(4)}};
const pk = {bytes: Uint8Array.from(Buffer.from('72cd6e8422c407fb6d098690f1130b7ded7ec2f7f5e1d30bd9d521f015363793','hex'))};
const calls = [];
const contract = new Contract({
  'context$new_coin_info': ({privateState}) => { calls.push('new_coin'); return [privateState,coin]; },
  'private$zk_public_key': ({privateState}) => { calls.push('public_key'); return [privateState,pk]; },
  'private$add_coin': ({privateState}, value) => {
    if (JSON.stringify(value) !== JSON.stringify(coin)) throw new Error('wrong coin witness argument');
    calls.push('add_coin'); return [privateState,[]];
  },
  'private$zk_secret_key': () => { throw new Error('unexpected secret key witness'); },
  'private$remove_coin': () => { throw new Error('unexpected remove witness'); },
  'context$path_of': () => { throw new Error('unexpected path witness'); },
  'context$encrypt': () => { throw new Error('unexpected encrypt witness'); },
});
const key = {bytes: new Uint8Array(32)};
const initial = contract.initialState({initialPrivateState:null,initialZswapLocalState:runtime.emptyZswapLocalState(key)});
const beforeStateHex = Buffer.from(initial.currentContractState.serialize()).toString('hex');
const queries = [];
const originalQuery = runtime.QueryContext.prototype.query;
runtime.QueryContext.prototype.query = function (...args) {
 const result=originalQuery.call(this,...args);queries.push({program:normalizeQueryProgram(args[0]),gasCost:result.gasCost});return result;
};
const context = runtime.createCircuitContext(runtime.dummyContractAddress(),key,initial.currentContractState.data,null);
const output=contract.circuits.zerocash_mint(context);
initial.currentContractState.data=new runtime.ChargedState(output.context.currentQueryContext.state.state);
const capture={beforeStateHex,afterStateHex:Buffer.from(initial.currentContractState.serialize()).toString('hex'),calls,result:output.result,gasCost:output.gasCost,queries,
 publicProgram:normalizeQueryProgram(output.proofData.publicTranscript),privateOutputs:normalizeQueryProgram(output.proofData.privateTranscriptOutputs)};
process.stdout.write(JSON.stringify(capture,(_,v)=>typeof v==='bigint'?v.toString():v,2)+'\n');
