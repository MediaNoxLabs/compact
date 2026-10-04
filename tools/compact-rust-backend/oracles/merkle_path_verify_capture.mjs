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

// Capture a witness Merkle path folded into ledger.checkRoot with ledger-8 TypeScript.
import { createRequire } from 'node:module';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
import { normalizeQueryProgram } from './normalize_query_program.mjs';

if (process.argv.length !== 3) throw new Error('usage: node merkle_path_verify_capture.mjs <compiled-contract-dir>');
const contractIndex = resolve(process.argv[2], 'index.js');
const requireFromContract = createRequire(contractIndex);
const runtime = await import(pathToFileURL(requireFromContract.resolve('@midnight-ntwrk/compact-runtime')));
const { Contract } = await import(pathToFileURL(contractIndex));
const queryCosts = [];
const originalQuery = runtime.QueryContext.prototype.query;
runtime.QueryContext.prototype.query = function (...args) {
  const result = originalQuery.call(this, ...args);
  queryCosts.push({
    gasCost: result.gasCost,
    program: normalizeQueryProgram(args[0]),
  });
  return result;
};
const contract = new Contract({
  leaf_path: ({ ledger, privateState }) => [privateState, ledger.t.pathForLeaf(0n, 7n)],
});
const coinPublicKey = { bytes: new Uint8Array(32) };
const initial = contract.initialState({
  initialPrivateState: null,
  initialZswapLocalState: runtime.emptyZswapLocalState(coinPublicKey),
});
let context = runtime.createCircuitContext(
  runtime.dummyContractAddress(), coinPublicKey,
  initial.currentContractState.data, initial.currentPrivateState,
);
function stateHex() {
  const state = new runtime.ContractState();
  state.data = new runtime.ChargedState(context.currentQueryContext.state.state);
  for (const key of initial.currentContractState.operations()) {
    state.setOperation(key, initial.currentContractState.operation(key));
  }
  state.maintenanceAuthority = initial.currentContractState.maintenanceAuthority;
  state.balance = initial.currentContractState.balance;
  return Buffer.from(state.serialize()).toString('hex');
}
function verify() {
  const start = queryCosts.length;
  const output = contract.circuits.verify(context);
  context = output.context;
  return {
    result: output.result,
    reportedGas: output.gasCost,
    queries: queryCosts.slice(start),
    state: stateHex(),
    privateTranscriptOutputs: output.proofData.privateTranscriptOutputs.map(
      ({ value, alignment }) => ({
        valueAtoms: value.map(atom => Array.from(atom)),
        alignment,
      }),
    ),
  };
}
context = contract.circuits.append(context, 7n).context;
const afterAppend = verify();
context = contract.circuits.replace(context, 8n).context;
const afterReplace = verify();
function normalize(value) {
  if (typeof value === 'bigint') return value.toString();
  if (value instanceof Uint8Array) return { bytesHex: Buffer.from(value).toString('hex') };
  if (Array.isArray(value)) return value.map(normalize);
  if (value && typeof value === 'object') {
    return Object.fromEntries(Object.entries(value).map(([key, inner]) => [key, normalize(inner)]));
  }
  return value;
}
process.stdout.write(JSON.stringify(normalize({ afterAppend, afterReplace }), null, 2) + '\n');
