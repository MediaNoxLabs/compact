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

// Compile assert_witness.compact with --skip-zk, link generated contract to
// this branch's runtime, then pass contract/index.js as the argument.
import { pathToFileURL } from 'node:url';
import * as runtime from '../../../runtime/dist/index.js';

const [contractPath] = process.argv.slice(2);
if (!contractPath) throw new Error('expected contract/index.js');
const { Contract } = await import(pathToFileURL(contractPath).href);
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
let observed = [];
const contract = new Contract({
  echo: ({ privateState }, value) => {
    observed.push({ privateState, value });
    return [privateState + 1, value];
  },
});
const coinPublicKey = { bytes: new Uint8Array(32) };
function initialWithContext() {
  const initial = contract.initialState({
    initialPrivateState: 7,
    initialZswapLocalState: runtime.emptyZswapLocalState(coinPublicKey),
  });
  return {
    initial,
    context: runtime.createCircuitContext(
      runtime.dummyContractAddress(), coinPublicKey,
      initial.currentContractState.data, initial.currentPrivateState,
    ),
  };
}
const initialContext = () => initialWithContext().context;
const stateHex = (initial) => Buffer.from(initial.currentContractState.serialize()).toString('hex');
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
const privateOutputs = (result) => result.proofData.privateTranscriptOutputs.map(
  ({ value, alignment }) => ({ valueAtoms: value.map((atom) => Array.from(atom)), alignment }),
);
function run(first, second) {
  observed = [];
  try {
    const result = contract.circuits.checked_value(initialContext(), first, second, 42n);
    return {
      result: result.result.toString(),
      privateState: result.context.currentPrivateState,
      privateTranscriptOutputs: privateOutputs(result),
      observed,
    };
  } catch (error) {
    return { error: error.message, observed };
  }
}
function write(flag) {
  observed = [];
  const { initial, context } = initialWithContext();
  const initialStateHex = stateHex(initial);
  const queryStart = queries.length;
  try {
    const written = contract.circuits.checked_write(context, flag, 42n);
    const writeQueries = queries.slice(queryStart);
    initial.currentContractState.data = new runtime.ChargedState(
      written.context.currentQueryContext.state.state,
    );
    const afterCall = stateHex(initial);
    const read = contract.circuits.read_cell(written.context);
    return {
      value: read.result.toString(),
      initialStateHex, afterCall,
      privateState: written.context.currentPrivateState,
      privateTranscriptOutputs: privateOutputs(written),
      gasCost: Object.fromEntries(
        Object.entries(written.gasCost).map(([key, value]) => [key, value.toString()]),
      ),
      publicTranscriptShape: written.proofData.publicTranscript.map(shape),
      queries: writeQueries,
      observed,
    };
  } catch (error) {
    return {
      error: error.message,
      initialStateHex,
      queryCount: queries.length - queryStart,
      observed,
    };
  }
}
process.stdout.write(JSON.stringify({
  pass: run(true, true),
  firstFails: run(false, false),
  secondFails: run(true, false),
  writePass: write(true),
  writeFails: write(false),
}, null, 2) + '\n');
