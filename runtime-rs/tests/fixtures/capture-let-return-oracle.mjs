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

// Compile the exact let_return_oracle.compact source with --target ts
// --skip-zk, link its contract/node_modules to the matching runtime,
// then pass the generated contract and runtime index.js paths.
import { pathToFileURL } from 'node:url';

const [contractPath, runtimePath] = process.argv.slice(2);
if (!contractPath || !runtimePath) throw new Error('expected contract and runtime paths');
const runtime = await import(pathToFileURL(runtimePath).href);
const { Contract } = await import(pathToFileURL(contractPath).href);
const contract = new Contract({});
const coinPublicKey = { bytes: new Uint8Array(32) };
const stateHex = (state) => Buffer.from(state.serialize()).toString('hex');
const gas = (cost) => Object.fromEntries(
  Object.entries(cost).map(([key, value]) => [key, value.toString()]),
);
const queries = [];
const originalQuery = runtime.QueryContext.prototype.query;
runtime.QueryContext.prototype.query = function (...args) {
  const result = originalQuery.call(this, ...args);
  queries.push({
    gasCost: gas(result.gasCost),
    opTags: args[0].map((operation) =>
      typeof operation === 'string' ? operation : Object.keys(operation)[0]),
  });
  return result;
};
function shape(operation) {
  if (typeof operation === 'string') return { kind: operation };
  if (operation.idx) return {
    kind: 'idx', cached: operation.idx.cached,
    pushPath: operation.idx.pushPath, pathLength: operation.idx.path.length,
  };
  if (operation.push) return { kind: 'push', storage: operation.push.storage };
  if (operation.ins) return { kind: 'ins', cached: operation.ins.cached, n: operation.ins.n };
  if (operation.dup) return { kind: 'dup', n: operation.dup.n };
  if (operation.rem) return { kind: 'rem', cached: operation.rem.cached };
  if (operation.popeq) return {
    kind: 'popeq', cached: operation.popeq.cached,
    resultAtoms: operation.popeq.result.value.map((atom) => Array.from(atom)),
  };
  throw new Error(`unexpected operation: ${Object.keys(operation)}`);
}

const initial = contract.initialState({
  initialPrivateState: null,
  initialZswapLocalState: runtime.emptyZswapLocalState(coinPublicKey),
});
const initialStateHex = stateHex(initial.currentContractState);
function call(next) {
  const queryStart = queries.length;
  const context = runtime.createCircuitContext(
    runtime.dummyContractAddress(), coinPublicKey,
    initial.currentContractState.data, initial.currentPrivateState,
  );
  const output = contract.circuits.replace(context, next);
  initial.currentContractState.data = new runtime.ChargedState(
    output.context.currentQueryContext.state.state,
  );
  return {
    next: next.toString(), result: output.result.toString(),
    afterStateHex: stateHex(initial.currentContractState),
    reportedGas: gas(output.gasCost),
    queries: queries.slice(queryStart),
    publicTranscriptShape: output.proofData.publicTranscript.map(shape),
    privateTranscriptCount: output.proofData.privateTranscriptOutputs.length,
  };
}
process.stdout.write(JSON.stringify({
  initialStateHex,
  first: call(9n),
  second: call(13n),
}, null, 2) + '\n');
