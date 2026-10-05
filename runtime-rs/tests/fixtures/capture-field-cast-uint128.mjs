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

// Compile field_cast_uint128.compact with --skip-zk, link generated contract
// to this branch's runtime, then pass contract/index.js as argument.
import { createRequire } from 'node:module';
import { pathToFileURL } from 'node:url';

const [contractPath] = process.argv.slice(2);
if (!contractPath) throw new Error('expected contract/index.js');
const requireFromContract = createRequire(contractPath);
const runtime = await import(pathToFileURL(requireFromContract.resolve('@midnight-ntwrk/compact-runtime')));
const { Contract, pureCircuits } = await import(pathToFileURL(contractPath).href);
const queries = [];
const originalQuery = runtime.QueryContext.prototype.query;
runtime.QueryContext.prototype.query = function (...args) {
  const output = originalQuery.call(this, ...args);
  queries.push({
    gasCost: output.gasCost,
    opTags: args[0].map((operation) => Object.keys(operation)[0]),
  });
  return output;
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
const contract = new Contract({});
const coinPublicKey = { bytes: new Uint8Array(32) };
const initial = contract.initialState({
  initialPrivateState: null,
  initialZswapLocalState: runtime.emptyZswapLocalState(coinPublicKey),
});
const input = (1n << 80n) + 7n;
const context = runtime.createCircuitContext(
  runtime.dummyContractAddress(), coinPublicKey,
  initial.currentContractState.data, initial.currentPrivateState,
);
const queryStart = queries.length;
const saved = contract.circuits.save(context, input);
initial.currentContractState.data = new runtime.ChargedState(saved.context.currentQueryContext.state.state);
process.stdout.write(JSON.stringify(normalize({
  input: input.toString(),
  pure: pureCircuits.as_field(input).toString(),
  returned: saved.result.toString(),
  afterHex: Buffer.from(initial.currentContractState.serialize()).toString('hex'),
  reportedGas: saved.gasCost,
  queries: queries.slice(queryStart),
  publicTranscript: saved.proofData.publicTranscript,
  privateOutputCount: saved.proofData.privateTranscriptOutputs.length,
}), null, 2) + '\n');
