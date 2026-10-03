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

// Compile chunked_cell.compact with --target ts --skip-zk, link
// contract/node_modules/@midnight-ntwrk/compact-runtime to this runtime,
// then pass the generated contract/index.js as the argument.
import { pathToFileURL } from 'node:url';
import * as runtime from '../../../runtime/dist/index.js';

const [contractPath] = process.argv.slice(2);
if (!contractPath) throw new Error('expected contract/index.js');
const { Contract } = await import(pathToFileURL(contractPath).href);
const contract = new Contract({});
const coinPublicKey = { bytes: new Uint8Array(32) };
const initial = contract.initialState({
  initialPrivateState: null,
  initialZswapLocalState: runtime.emptyZswapLocalState(coinPublicKey),
});
const initialStateHex = Buffer.from(initial.currentContractState.serialize()).toString('hex');
const queries = [];
const originalQuery = runtime.QueryContext.prototype.query;
runtime.QueryContext.prototype.query = function (...args) {
  const result = originalQuery.call(this, ...args);
  queries.push({
    gasCost: Object.fromEntries(
      Object.entries(result.gasCost).map(([key, value]) => [key, value.toString()]),
    ),
    opTags: args[0].map((operation) =>
      typeof operation === 'string' ? operation : Object.keys(operation)[0]),
  });
  return result;
};

function shape(operation) {
  if (typeof operation === 'string') return { kind: operation };
  if (operation.idx) {
    const { cached, pushPath, path } = operation.idx;
    return { kind: 'idx', cached, pushPath, pathLength: path.length };
  }
  if (operation.push) return { kind: 'push', storage: operation.push.storage };
  if (operation.ins) return { kind: 'ins', cached: operation.ins.cached, n: operation.ins.n };
  if (operation.rem) return { kind: 'rem', cached: operation.rem.cached };
  if (operation.dup) return { kind: 'dup', n: operation.dup.n };
  if (operation.popeq) {
    return {
      kind: 'popeq', cached: operation.popeq.cached,
      resultAtoms: operation.popeq.result.value.map((atom) => Array.from(atom)),
    };
  }
  throw new Error(`unexpected operation: ${Object.keys(operation)}`);
}

function capture(name, ...inputs) {
  const start = queries.length;
  const context = runtime.createCircuitContext(
    runtime.dummyContractAddress(), coinPublicKey,
    initial.currentContractState.data, initial.currentPrivateState,
  );
  const output = contract.circuits[name](context, ...inputs);
  const proofData = output.proofData;
  initial.currentContractState.data = new runtime.ChargedState(
    output.context.currentQueryContext.state.state,
  );
  const afterStateHex = Buffer.from(initial.currentContractState.serialize()).toString('hex');
  initial.currentContractState.data = new runtime.ChargedState(
    context.currentQueryContext.state.state,
  );
  return {
    result: String(output.result),
    valueAtoms: proofData.input.value.map((atom) => Array.from(atom)),
    alignment: proofData.input.alignment,
    publicTranscriptShape: proofData.publicTranscript.map(shape),
    queries: queries.slice(start),
    reportedGas: Object.fromEntries(
      Object.entries(output.gasCost).map(([key, value]) => [key, value.toString()]),
    ),
    afterStateHex,
  };
}

process.stdout.write(JSON.stringify({
  initialStateHex,
  activePath: [1, 14],
  amountPath: [1, 13],
  setActive: capture('set_active', false),
  getActive: capture('get_active'),
  assertActive: capture('assert_active', true),
  setAmount: capture('set_amount', 11n),
  getAmount: capture('get_amount'),
  addAmount: capture('add_amount', 7n),
  activeEquals: capture('active_equals', true),
  plusAmount: capture('plus_amount', 7n),
  subtractAmount: capture('subtract_amount', 2n),
  multiplyAmount: capture('multiply_amount', 7n),
}, null, 2) + '\n');
