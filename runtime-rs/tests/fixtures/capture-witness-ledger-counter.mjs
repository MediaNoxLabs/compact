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

// Compile examples/rust_backend/witness_ledger_counter.compact with --skip-zk,
// link the generated contract runtime to this branch's runtime, then run:
// node capture-witness-ledger-counter.mjs <contract/index.js>.
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
  read_round: ({ ledger, privateState }) => [privateState + 1, ledger.round],
});
const coinPublicKey = { bytes: new Uint8Array(32) };
const initial = contract.initialState({
  initialPrivateState: 7,
  initialZswapLocalState: runtime.emptyZswapLocalState(coinPublicKey),
});
let context = runtime.createCircuitContext(
  runtime.dummyContractAddress(),
  coinPublicKey,
  initial.currentContractState.data,
  initial.currentPrivateState,
);

function normalize(value) {
  if (typeof value === 'bigint') return value.toString();
  if (value instanceof Uint8Array) return { bytesHex: Buffer.from(value).toString('hex') };
  if (Array.isArray(value)) return value.map(normalize);
  if (value && typeof value === 'object') {
    return Object.fromEntries(Object.entries(value).map(([key, inner]) => [key, normalize(inner)]));
  }
  return value;
}

function output(result, queries) {
  return {
    result: result.result.toString(),
    privateState: result.context.currentPrivateState,
    reportedGas: normalize(result.gasCost),
    queries: normalize(queries),
    publicTranscript: normalize(result.proofData.publicTranscript),
    privateTranscriptOutputs: result.proofData.privateTranscriptOutputs.map(
      ({ value, alignment }) => ({
        valueAtoms: value.map((atom) => Array.from(atom)),
        alignment,
      }),
    ),
  };
}

let queryStart = queryCosts.length;
const before = contract.circuits.private_round(context);
const beforeQueries = queryCosts.slice(queryStart);
context = contract.circuits.increment_round(before.context).context;
queryStart = queryCosts.length;
const after = contract.circuits.private_round(context);
const afterQueries = queryCosts.slice(queryStart);
process.stdout.write(JSON.stringify({
  before: output(before, beforeQueries),
  after: output(after, afterQueries),
}, null, 2) + '\n');
