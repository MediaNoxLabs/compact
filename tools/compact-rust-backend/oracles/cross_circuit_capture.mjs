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

// Capture exact codegen-rust cross_circuit_fixture with ledger-8 TypeScript.
import { createRequire } from 'node:module';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

if (process.argv.length !== 3) throw new Error('usage: node cross_circuit_capture.mjs <compiled-contract-dir>');
const contractIndex = resolve(process.argv[2], 'index.js');
const requireFromContract = createRequire(contractIndex);
const runtime = await import(pathToFileURL(requireFromContract.resolve('@midnight-ntwrk/compact-runtime')));
const { Contract, ledger } = await import(pathToFileURL(contractIndex));
function gas(cost) {
  return Object.fromEntries(
    Object.entries(cost).map(([name, value]) => [name, value.toString()]),
  );
}
function operation(operation) {
  if (typeof operation === 'string') return { kind: operation };
  if (operation.push) return { kind: 'push', storage: operation.push.storage };
  if (operation.ins) return { kind: 'ins', cached: operation.ins.cached, n: operation.ins.n };
  if (operation.idx) {
    const { cached, pushPath, path } = operation.idx;
    return { kind: 'idx', cached, pushPath, pathLength: path.length };
  }
  if (operation.popeq) {
    return {
      kind: 'popeq', cached: operation.popeq.cached,
      resultAtoms: operation.popeq.result.value.map((atom) => Array.from(atom)),
    };
  }
  throw new Error(`unexpected operation: ${Object.keys(operation)}`);
}
const contract = new Contract({});
const coinPublicKey = { bytes: new Uint8Array(32) };
const initial = contract.initialState({
  initialPrivateState: null,
  initialZswapLocalState: runtime.emptyZswapLocalState(coinPublicKey),
});
let context = runtime.createCircuitContext(
  runtime.dummyContractAddress(), coinPublicKey,
  initial.currentContractState.data, initial.currentPrivateState,
);
function snapshot() {
  const state = new runtime.ContractState();
  state.data = new runtime.ChargedState(context.currentQueryContext.state.state);
  for (const key of initial.currentContractState.operations()) {
    state.setOperation(key, initial.currentContractState.operation(key));
  }
  state.maintenanceAuthority = initial.currentContractState.maintenanceAuthority;
  state.balance = initial.currentContractState.balance;
  return Buffer.from(state.serialize()).toString('hex');
}
function sumGas(queries) {
  return Object.fromEntries(
    ['readTime', 'computeTime', 'bytesWritten', 'bytesDeleted'].map((name) => [
      name,
      queries.reduce((total, query) => total + BigInt(query.gas[name]), 0n).toString(),
    ]),
  );
}
function invoke(call) {
  const firstQuery = queryCosts.length;
  const result = call();
  context = result.context;
  const queries = queryCosts.slice(firstQuery);
  return {
    reportedGas: gas(result.gasCost),
    totalGas: sumGas(queries),
    queries,
    transcript: result.proofData.publicTranscript.map(operation),
    privateTranscriptCount: result.proofData.privateTranscriptOutputs.length,
  };
}
const afterInit = Buffer.from(initial.currentContractState.serialize()).toString('hex');
const initialValue = ledger(initial.currentContractState.data).n.toString();
const queryCosts = [];
const originalQuery = runtime.QueryContext.prototype.query;
runtime.QueryContext.prototype.query = function (...args) {
  const result = originalQuery.call(this, ...args);
  queryCosts.push({ gas: gas(result.gasCost), transcript: args[0].map(operation) });
  return result;
};
const set7 = invoke(() => contract.circuits.reset_and_set(context, 7n));
const afterSet7 = snapshot();
const set13 = invoke(() => contract.circuits.reset_and_set(context, 13n));
const afterSet13 = snapshot();
const reset = invoke(() => contract.circuits.reset(context));
const afterReset = snapshot();
process.stdout.write(JSON.stringify({
  afterInit, initialValue, afterSet7, afterSet13, afterReset,
  set7, set13, reset,
}, null, 2) + '\n');
