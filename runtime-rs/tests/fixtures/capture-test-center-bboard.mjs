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

// Capture the unmodified test-center/test-contracts/bboard.compact using the
// matching ledger-8 TypeScript runtime and a rehydrated deployment per call.
import { createRequire } from 'node:module';
import { pathToFileURL } from 'node:url';

const [contractPath] = process.argv.slice(2);
if (!contractPath) throw new Error('expected contract/index.js');
const requireFromContract = createRequire(contractPath);
const runtime = await import(pathToFileURL(requireFromContract.resolve('@midnight-ntwrk/compact-runtime')));
const { Contract, pureCircuits } = await import(pathToFileURL(contractPath).href);

const cost = (gas) => Object.fromEntries(Object.entries(gas).map(([key, value]) => [key, value.toString()]));
const queries = [];
const originalQuery = runtime.QueryContext.prototype.query;
runtime.QueryContext.prototype.query = function (...args) {
  const result = originalQuery.call(this, ...args);
  queries.push({
    opTags: args[0].map((operation) => typeof operation === 'string' ? operation : Object.keys(operation)[0]),
    gasCost: cost(result.gasCost),
  });
  return result;
};
const stateHex = (state) => Buffer.from(state.serialize()).toString('hex');
function updatedState(previous, queryState) {
  const state = new runtime.ContractState();
  state.data = new runtime.ChargedState(queryState);
  for (const entry of previous.operations()) state.setOperation(entry, previous.operation(entry));
  state.maintenanceAuthority = previous.maintenanceAuthority;
  state.balance = previous.balance;
  return state;
}
function shape(operation) {
  if (typeof operation === 'string') return { kind: operation };
  const [kind, details] = Object.entries(operation)[0];
  if (kind === 'idx') return { kind, cached: details.cached, pushPath: details.pushPath, pathLength: details.path.length };
  if (kind === 'push') return { kind, storage: details.storage };
  if (kind === 'ins') return { kind, cached: details.cached, n: details.n };
  if (kind === 'popeq') return { kind, cached: details.cached, resultAtoms: details.result.value.map((atom) => Array.from(atom)) };
  if (kind === 'dup' || kind === 'swap') return { kind, n: details.n };
  return { kind };
}
const secretKey = new Uint8Array(32).fill(7);
const witnessCalls = [];
const contract = new Contract({
  local_secret_key: ({ privateState }) => {
    witnessCalls.push(privateState);
    return [privateState, secretKey];
  },
});
const coinPublicKey = { bytes: new Uint8Array(32) };
const initial = contract.initialState({
  initialPrivateState: 5,
  initialZswapLocalState: runtime.emptyZswapLocalState(coinPublicKey),
});
const deployed = runtime.ContractState.deserialize(initial.currentContractState.serialize());
const initialStateHex = stateHex(deployed);
function context(state) {
  return runtime.createCircuitContext(runtime.dummyContractAddress(), coinPublicKey, state.data, 5);
}
function capture(name, state, arg) {
  const queryStart = queries.length;
  const witnessStart = witnessCalls.length;
  const beforeStateHex = stateHex(state);
  try {
    const result = name === 'post' ? contract.circuits.post(context(state), arg) : contract.circuits.take_down(context(state));
    const after = updatedState(state, result.context.currentQueryContext.state.state);
    return {
      success: true,
      result: result.result,
      beforeStateHex,
      afterStateHex: stateHex(after),
      reportedGas: cost(result.gasCost),
      queries: queries.slice(queryStart),
      publicTranscriptShape: result.proofData.publicTranscript.map(shape),
      privateTranscriptCount: result.proofData.privateTranscriptOutputs.length,
      witnessCalls: witnessCalls.slice(witnessStart),
      state: after,
    };
  } catch (error) {
    return {
      success: false,
      error: error.message,
      beforeStateHex,
      afterStateHex: stateHex(state),
      queries: queries.slice(queryStart),
      witnessCalls: witnessCalls.slice(witnessStart),
    };
  }
}
const emptyTakeDown = capture('take_down', deployed);
const message = '🌙 Midnight — 你好, Привіт';
const post = capture('post', deployed, message);
const takeDown = capture('take_down', post.state);
delete post.state;
delete takeDown.state;
const instanceBytes = new Uint8Array(32);
instanceBytes[0] = 1;
process.stdout.write(JSON.stringify({
  source: 'test-center/test-contracts/bboard.compact',
  message,
  initialStateHex,
  emptyTakeDown,
  post,
  takeDown,
  publicKeyHex: Buffer.from(pureCircuits.public_key(secretKey, instanceBytes)).toString('hex'),
}, null, 2) + '\n');
