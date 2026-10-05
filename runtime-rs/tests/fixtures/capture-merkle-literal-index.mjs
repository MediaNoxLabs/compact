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

// Compile merkle_path_verify.compact for TypeScript with --skip-zk, link the
// generated contract package to this repository runtime, then pass its directory.
import { createRequire } from 'node:module';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

const [contractDirectory] = process.argv.slice(2);
if (!contractDirectory) throw new Error('expected generated contract directory');
const index = resolve(contractDirectory, 'index.js');
const requireFromContract = createRequire(index);
const runtime = await import(pathToFileURL(requireFromContract.resolve('@midnight-ntwrk/compact-runtime')));
const { Contract } = await import(pathToFileURL(index));

function normalize(value) {
  if (typeof value === 'bigint') return value.toString();
  if (value instanceof Uint8Array) return Array.from(value);
  if (Array.isArray(value)) return value.map(normalize);
  if (value && typeof value === 'object') {
    return Object.fromEntries(Object.entries(value).map(([key, inner]) => [key, normalize(inner)]));
  }
  return value;
}

const queries = [];
const originalQuery = runtime.QueryContext.prototype.query;
runtime.QueryContext.prototype.query = function (...args) {
  const result = originalQuery.call(this, ...args);
  queries.push({ gasCost: normalize(result.gasCost), program: normalize(args[0]) });
  return result;
};

const contract = new Contract({
  leaf_path: () => { throw new Error('leaf_path is not used by append or replace'); },
});
const coinPublicKey = { bytes: new Uint8Array(32) };
const state = contract.initialState({
  initialPrivateState: null,
  initialZswapLocalState: runtime.emptyZswapLocalState(coinPublicKey),
});
const hex = () => Buffer.from(state.currentContractState.serialize()).toString('hex');
function call(name, ...args) {
  const start = queries.length;
  const context = runtime.createCircuitContext(
    runtime.dummyContractAddress(), coinPublicKey,
    state.currentContractState.data, state.currentPrivateState,
  );
  const output = contract.circuits[name](context, ...args);
  state.currentContractState.data = new runtime.ChargedState(
    output.context.currentQueryContext.state.state,
  );
  state.currentPrivateState = output.context.currentPrivateState;
  return {
    result: normalize(output.result),
    stateHex: hex(),
    reportedGas: normalize(output.gasCost),
    queries: queries.slice(start),
    publicTranscript: normalize(output.proofData.publicTranscript),
    privateTranscriptOutputs: normalize(output.proofData.privateTranscriptOutputs),
    privateState: normalize(state.currentPrivateState),
  };
}

const initialHex = hex();
const append = call('append', 7n);
const replace = call('replace', 8n);
process.stdout.write(JSON.stringify({ initialHex, append, replace }, null, 2) + '\n');
