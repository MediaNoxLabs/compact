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

// Compile the original election_oracle.compact with --skip-zk and pass contract/index.js.
import { pathToFileURL } from 'node:url';
import { createRequire } from 'node:module';
import { normalizeQueryProgram } from '../../../tools/compact-rust-backend/oracles/normalize_query_program.mjs';
const [contractPath] = process.argv.slice(2);
const runtime = await import(pathToFileURL(createRequire(contractPath).resolve('@midnight-ntwrk/compact-runtime')));
const { Contract } = await import(pathToFileURL(contractPath));
const secret = new Uint8Array(32).fill(7);
const prefix = new Uint8Array(32); prefix.set(new TextEncoder().encode('lares:election:pk:'));
const authority = runtime.persistentHash(new runtime.CompactTypeVector(2, new runtime.CompactTypeBytes(32)), [prefix, secret]);
const coinPublicKey = { bytes: new Uint8Array(32) };
const contract = new Contract({
  private$secret_key: ({ privateState }) => [privateState + 1, secret],
  private$state: () => [0, 0], private$state$advance: () => [0, []],
  private$vote$record: () => [0, []], private$vote: () => [0, 0],
  context$eligible_voters$path_of: () => { throw new Error('unused'); },
  context$committed_votes$path_of: () => { throw new Error('unused'); },
});
const initial = (key = authority) => contract.initialState({ initialPrivateState: 10,
  initialZswapLocalState: runtime.emptyZswapLocalState(coinPublicKey) }, key);
const context = (state) => runtime.createCircuitContext(runtime.dummyContractAddress(), coinPublicKey,
  state.currentContractState.data, state.currentPrivateState);
const hex = (state, ctx) => { state.currentContractState.data = new runtime.ChargedState(ctx.currentQueryContext.state.state);
  return Buffer.from(state.currentContractState.serialize()).toString('hex'); };
let queries = [];
let raw = [];
const original = runtime.QueryContext.prototype.query;
runtime.QueryContext.prototype.query = function (...args) {
  const output = original.call(this, ...args);
  queries.push({ program: normalizeQueryProgram(args[0]), gasCost: output.gasCost });
  raw.push(args[0]); return output;
};
const scenarios = [];
for (const topic of ['', 'hello', '議題 🗳️']) {
  const state = initial(); const before = state.currentContractState.data;
  queries = []; raw = [];
  const result = contract.circuits.set_topic(context(state), topic);
  const captured = queries.slice(); const replayOps = raw.flat();
  const replayContext = runtime.createCircuitContext(runtime.dummyContractAddress(), coinPublicKey, before, 10);
  const replay = replayContext.currentQueryContext.query(replayOps, replayContext.costModel);
  scenarios.push({ topic, stateHex: hex(state, result.context), privateState: result.context.currentPrivateState,
    privateOutputs: result.proofData.privateTranscriptOutputs.length,
    privateTranscriptOutputs: result.proofData.privateTranscriptOutputs,
    gasCost: result.gasCost, queries: captured, replayGas: replay.gasCost });
}
function rejected(invoke) { try { invoke(); throw new Error('unexpected success'); } catch (error) { return error.message; } }
const wrongAuthority = rejected(() => contract.circuits.set_topic(context(initial(new Uint8Array(32))), 'denied'));
const state = initial();
const configured = contract.circuits.set_topic(context(state), 'ready');
const advanced = contract.circuits.advance(configured.context);
const wrongPhase = rejected(() => contract.circuits.set_topic(advanced.context, 'denied'));
process.stdout.write(JSON.stringify({ authority: Array.from(authority), scenarios, wrongAuthority, wrongPhase },
  (_key, value) => typeof value === 'bigint' ? value.toString() : value instanceof Uint8Array ? Array.from(value) : value, 2) + '\n');
