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

// Compile wide_uint_oracle.compact with --skip-zk in a fresh target, link the
// generated contract to this branch's runtime, then pass contract/index.js.
import { pathToFileURL } from 'node:url';
import * as runtime from '../../../runtime/dist/index.js';

const queryCosts = [];
const originalQuery = runtime.QueryContext.prototype.query;
runtime.QueryContext.prototype.query = function (...args) {
  const result = originalQuery.call(this, ...args);
  queryCosts.push({
    gasCost: result.gasCost,
    opTags: args[0].map((op) => typeof op === 'string' ? op : Object.keys(op)[0]),
  });
  return result;
};

function normalize(value) {
  if (typeof value === 'bigint') return value.toString();
  if (value instanceof Uint8Array) return { bytesHex: Buffer.from(value).toString('hex') };
  if (Array.isArray(value)) return value.map(normalize);
  if (value && typeof value === 'object') {
    return Object.fromEntries(Object.entries(value).map(([key, inner]) => [key, normalize(inner)]));
  }
  return value;
}

function privateOutputs(output) {
  return output.proofData.privateTranscriptOutputs.map(({ value, alignment }) => ({
    valueAtoms: value.map((atom) => Array.from(atom)),
    alignment,
  }));
}

function capture(invoke) {
  const start = queryCosts.length;
  const output = invoke();
  return {
    output,
    observations: {
      reportedGas: normalize(output.gasCost),
      queries: normalize(queryCosts.slice(start)),
      publicTranscript: normalize(output.proofData.publicTranscript),
      privateOutputCount: output.proofData.privateTranscriptOutputs.length,
      privateTranscriptOutputs: privateOutputs(output),
    },
  };
}

const [contractPath] = process.argv.slice(2);
if (!contractPath) throw new Error('expected contract/index.js');
const { Contract, pureCircuits } = await import(pathToFileURL(contractPath).href);
const maximum = (1n << 248n) - 1n;
const contract = new Contract({
  nextWide: ({ privateState }) => [privateState + 1, maximum],
});
const coinPublicKey = { bytes: new Uint8Array(32) };
const initial = contract.initialState({
  initialPrivateState: 7,
  initialZswapLocalState: runtime.emptyZswapLocalState(coinPublicKey),
});
const initialHex = Buffer.from(initial.currentContractState.serialize()).toString('hex');
const context = runtime.createCircuitContext(
  runtime.dummyContractAddress(), coinPublicKey,
  initial.currentContractState.data, initial.currentPrivateState,
);
const { output: write, observations: writeWide } = capture(
  () => contract.circuits.writeWide(context),
);
initial.currentContractState.data = new runtime.ChargedState(
  write.context.currentQueryContext.state.state,
);
const afterWriteHex = Buffer.from(initial.currentContractState.serialize()).toString('hex');
const { output: read, observations: readWide } = capture(
  () => contract.circuits.readWide(write.context),
);
process.stdout.write(JSON.stringify({
  initialHex,
  afterWriteHex,
  privateState: write.context.currentPrivateState,
  read: String(read.result),
  maxWide: String(pureCircuits.maxWide()),
  privateTranscriptOutputs: privateOutputs(write),
  writeWide,
  readWide,
}, null, 2) + '\n');
