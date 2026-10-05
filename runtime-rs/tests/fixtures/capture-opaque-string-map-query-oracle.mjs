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

// Compile opaque_string_map_query_oracle.compact with --skip-zk into a fresh
// target, link its package to this branch's runtime, then pass contract/index.js.
import { pathToFileURL } from 'node:url';
import * as runtime from '../../../runtime/dist/index.js';

const [contractPath] = process.argv.slice(2);
if (!contractPath) throw new Error('expected contract/index.js');
const { Contract } = await import(pathToFileURL(contractPath).href);
const contract = new Contract({});
const coinPublicKey = { bytes: new Uint8Array(32) };
const initial = contract.initialState({
  initialPrivateState: 7,
  initialZswapLocalState: runtime.emptyZswapLocalState(coinPublicKey),
});
const initialHex = Buffer.from(initial.currentContractState.serialize()).toString('hex');
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
  const beforeStateHex = Buffer.from(initial.currentContractState.serialize()).toString('hex');
  const context = runtime.createCircuitContext(
    runtime.dummyContractAddress(), coinPublicKey,
    initial.currentContractState.data, initial.currentPrivateState,
  );
  try {
    const output = contract.circuits[name](context, ...inputs);
    initial.currentContractState.data = new runtime.ChargedState(
      output.context.currentQueryContext.state.state,
    );
    return {
      result: typeof output.result === 'bigint' ? output.result.toString() : output.result,
      afterStateHex: Buffer.from(initial.currentContractState.serialize()).toString('hex'),
      reportedGas: Object.fromEntries(
        Object.entries(output.gasCost).map(([key, value]) => [key, value.toString()]),
      ),
      queries: queries.slice(start),
      publicTranscriptShape: output.proofData.publicTranscript.map(shape),
      privateTranscriptCount: output.proofData.privateTranscriptOutputs.length,
      privateState: output.context.currentPrivateState,
    };
  } catch (error) {
    return {
      error: error.message,
      afterStateHex: beforeStateHex,
      queries: queries.slice(start),
      privateState: initial.currentPrivateState,
    };
  }
}

const before = capture('ensure', 'asset-1');
const put = capture('put', 'asset-1', 42n);
const ensured = capture('ensure', 'asset-1');
const other = capture('ensure', 'other');
process.stdout.write(JSON.stringify({
  initialHex,
  afterPutHex: put.afterStateHex,
  beforeError: before.error,
  result: ensured.result,
  calls: { before, put, ensure: ensured, other },
}, null, 2) + '\n');
