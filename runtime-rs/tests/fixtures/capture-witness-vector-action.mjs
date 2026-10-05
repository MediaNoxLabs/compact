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

// Compile witness_vector_action.compact with --skip-zk, link generated
// contract to this branch's runtime, then pass contract/index.js as argument.
import { pathToFileURL } from 'node:url';
import * as runtime from '../../../runtime/dist/index.js';

const [contractPath] = process.argv.slice(2);
if (!contractPath) throw new Error('expected contract/index.js');
const { Contract } = await import(pathToFileURL(contractPath).href);
let calls = 0;
const contract = new Contract({
  sumWitness: ({ privateState }, values) => {
    calls++;
    if (values.length !== 2 || values[0] !== 0n || values[1] !== 1n) {
      throw new Error('unexpected witness vector');
    }
    return [privateState + 1, values[0] + values[1] + BigInt(privateState)];
  },
});
const coinPublicKey = { bytes: new Uint8Array(32) };
const queries = [];
const originalQuery = runtime.QueryContext.prototype.query;
runtime.QueryContext.prototype.query = function (...args) {
  const result = originalQuery.call(this, ...args);
  queries.push({
    gasCost: Object.fromEntries(
      Object.entries(result.gasCost).map(([key, value]) => [key, value.toString()]),
    ),
  });
  return result;
};

function operationShape(operation) {
  if (typeof operation === 'string') return { kind: operation };
  if (operation.idx) {
    const { cached, pushPath, path } = operation.idx;
    return { kind: 'idx', cached, pushPath, pathLength: path.length };
  }
  if (operation.push) return { kind: 'push', storage: operation.push.storage };
  if (operation.addi) return { kind: 'addi', immediate: operation.addi.immediate };
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

function scenario(name) {
  calls = 0;
  const initial = contract.initialState({
    initialPrivateState: 7,
    initialZswapLocalState: runtime.emptyZswapLocalState(coinPublicKey),
  });
  const context = runtime.createCircuitContext(
    runtime.dummyContractAddress(), coinPublicKey,
    initial.currentContractState.data, initial.currentPrivateState,
  );
  const queryStart = queries.length;
  const result = contract.circuits[name](context);
  initial.currentContractState.data = new runtime.ChargedState(
    result.context.currentQueryContext.state.state,
  );
  return {
    calls,
    privateState: result.context.currentPrivateState,
    stateHex: Buffer.from(initial.currentContractState.serialize()).toString('hex'),
    privateTranscriptOutputs: result.proofData.privateTranscriptOutputs.map(
      ({ value, alignment }) => ({
        valueAtoms: value.map((atom) => Array.from(atom)),
        alignment,
      }),
    ),
    trace: {
      publicTranscriptShape: result.proofData.publicTranscript.map(operationShape),
      privateTranscriptCount: result.proofData.privateTranscriptOutputs.length,
      queries: queries.slice(queryStart),
      reportedGas: Object.fromEntries(
        Object.entries(result.gasCost).map(([key, value]) => [key, value.toString()]),
      ),
    },
  };
}

process.stdout.write(JSON.stringify({
  keepResult: scenario('keepResult'),
  discardResult: scenario('discardResult'),
  reuseResult: scenario('reuseResult'),
}, null, 2) + '\n');
