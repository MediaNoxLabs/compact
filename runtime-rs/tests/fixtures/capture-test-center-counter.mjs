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

// Compile the original test-center/test-contracts/counter.compact with --target ts
// --skip-zk, link its contract/node_modules to the matching Nix runtime package,
// then run: node capture-test-center-counter.mjs <contract/index.js> <runtime/dist/index.js>.
import { pathToFileURL } from 'node:url';

const [contractPath, runtimePath] = process.argv.slice(2);
if (!contractPath || !runtimePath) throw new Error('expected contract and runtime index.js paths');
const runtime = await import(pathToFileURL(runtimePath).href);
const { Contract } = await import(pathToFileURL(contractPath).href);
const queries = [];
const originalQuery = runtime.QueryContext.prototype.query;
runtime.QueryContext.prototype.query = function (...args) {
  const result = originalQuery.call(this, ...args);
  queries.push({
    gasCost: Object.fromEntries(
      Object.entries(result.gasCost).map(([key, value]) => [key, value.toString()]),
    ),
    opTags: args[0].map((operation) => Object.keys(operation)[0]),
  });
  return result;
};

const observed = [];
const contract = new Contract({
  private_increment: ({ ledger, privateState }) => {
    observed.push({ ledgerRound: ledger.round.toString(), privateState });
    return [privateState + 1, []];
  },
});
const coinPublicKey = { bytes: new Uint8Array(32) };
const initial = contract.initialState({
  initialPrivateState: 7,
  initialZswapLocalState: runtime.emptyZswapLocalState(coinPublicKey),
});
const initialStateHex = Buffer.from(initial.currentContractState.serialize()).toString('hex');
const context = runtime.createCircuitContext(
  runtime.dummyContractAddress(), coinPublicKey,
  initial.currentContractState.data, initial.currentPrivateState,
);
const queryStart = queries.length;
const call = contract.circuits.increment(context);
initial.currentContractState.data = new runtime.ChargedState(
  call.context.currentQueryContext.state.state,
);
const afterCallStateHex = Buffer.from(initial.currentContractState.serialize()).toString('hex');
const normalizedGas = Object.fromEntries(
  Object.entries(call.gasCost).map(([key, value]) => [key, value.toString()]),
);
process.stdout.write(JSON.stringify({
  source: 'test-center/test-contracts/counter.compact',
  initialStateHex,
  afterCallStateHex,
  result: call.result,
  privateState: call.context.currentPrivateState,
  observed,
  gasCost: normalizedGas,
  queries: queries.slice(queryStart),
  publicTranscriptOpTags: call.proofData.publicTranscript.map(
    (operation) => Object.keys(operation)[0],
  ),
  privateTranscriptOutputs: call.proofData.privateTranscriptOutputs.map(
    ({ value, alignment }) => ({
      valueAtoms: value.map((atom) => Array.from(atom)),
      alignment,
    }),
  ),
}, null, 2) + '\n');
