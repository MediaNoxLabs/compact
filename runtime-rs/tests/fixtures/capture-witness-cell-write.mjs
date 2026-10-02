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

// Compile witness_cell_write with --skip-zk, link its generated contract to
// this branch's runtime, then pass contract/index.js as the only argument.
import { pathToFileURL } from 'node:url';
import * as runtime from '../../../runtime/dist/index.js';

const [contractPath] = process.argv.slice(2);
if (!contractPath) throw new Error('expected contract/index.js');
const { Contract } = await import(pathToFileURL(contractPath).href);
const queryCosts = [];
const originalQuery = runtime.QueryContext.prototype.query;
runtime.QueryContext.prototype.query = function (...args) {
  const result = originalQuery.call(this, ...args);
  queryCosts.push({
    gasCost: result.gasCost,
    opTags: args[0].map((op) => Object.keys(op)[0]),
  });
  return result;
};
const contract = new Contract({
  secret: ({ ledger, privateState }, seed) => {
    if (seed !== 2n) throw new Error('unexpected witness argument');
    const expectedCell = privateState === 7 ? 0n : 9n;
    if (ledger.cell !== expectedCell) throw new Error('witness saw stale Cell state');
    return [privateState + 1, seed + BigInt(privateState)];
  },
});
const coinPublicKey = { bytes: new Uint8Array(32) };

function initialContext() {
  const initial = contract.initialState({
    initialPrivateState: 7,
    initialZswapLocalState: runtime.emptyZswapLocalState(coinPublicKey),
  });
  return {
    initial,
    context: runtime.createCircuitContext(
      runtime.dummyContractAddress(),
      coinPublicKey,
      initial.currentContractState.data,
      initial.currentPrivateState,
    ),
  };
}

const stateHex = (initial) => Buffer.from(initial.currentContractState.serialize()).toString('hex');

function normalize(value) {
  if (typeof value === 'bigint') return value.toString();
  if (value instanceof Uint8Array) return { bytesHex: Buffer.from(value).toString('hex') };
  if (Array.isArray(value)) return value.map(normalize);
  if (value && typeof value === 'object') {
    return Object.fromEntries(Object.entries(value).map(([key, inner]) => [key, normalize(inner)]));
  }
  return value;
}

function run(name) {
  const { initial, context } = initialContext();
  const afterInit = stateHex(initial);
  const queryStart = queryCosts.length;
  const write = contract.circuits[name](context, 2n);
  const queries = queryCosts.slice(queryStart);
  initial.currentContractState.data = new runtime.ChargedState(
    write.context.currentQueryContext.state.state,
  );
  const afterCall = stateHex(initial);
  const read = contract.circuits.read_cell(write.context);
  return {
    afterInit,
    afterCall,
    cell: read.result.toString(),
    privateState: write.context.currentPrivateState,
    reportedGas: normalize(write.gasCost),
    queries: normalize(queries),
    publicTranscript: normalize(write.proofData.publicTranscript),
    privateTranscriptOutputs: write.proofData.privateTranscriptOutputs.map(
      ({ value, alignment }) => ({
        valueAtoms: value.map((atom) => Array.from(atom)),
        alignment,
      }),
    ),
  };
}

process.stdout.write(JSON.stringify({
  single: run('write_secret'),
  twice: run('write_twice'),
  nested: run('write_nested_twice'),
}, null, 2) + '\n');
