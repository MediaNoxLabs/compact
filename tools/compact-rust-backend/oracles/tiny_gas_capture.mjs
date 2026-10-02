// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//  http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

// Capture every circuit ledger query cost, since the generated TypeScript
// wrapper reports only the last query's gasCost for each invocation.
import { createRequire } from 'node:module';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

if (process.argv.length !== 3) {
  throw new Error('usage: node tiny_gas_capture.mjs <compiled-contract-dir>');
}
const contractIndex = resolve(process.argv[2], 'index.js');
const requireFromContract = createRequire(contractIndex);
const runtime = await import(pathToFileURL(requireFromContract.resolve('@midnight-ntwrk/compact-runtime')));
const { Contract } = await import(pathToFileURL(contractIndex));

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

const witnesses = {
  private$secret_key: (context) => [context.privateState, new Uint8Array(32).fill(7)],
};
const contract = new Contract(witnesses);
const coinPublicKey = { bytes: new Uint8Array(32) };
const initial = contract.initialState({
  initialPrivateState: null,
  initialZswapLocalState: runtime.emptyZswapLocalState(coinPublicKey),
}, 42n);
let context = runtime.createCircuitContext(
  runtime.dummyContractAddress(),
  coinPublicKey,
  initial.currentContractState.data,
  initial.currentPrivateState,
);

const steps = {};
for (const [name, invoke] of [
  ['clear', () => contract.circuits.clear(context)],
  ['set', () => contract.circuits.set(context, 99n)],
  ['get', () => contract.circuits.get(context)],
]) {
  const queryStart = queryCosts.length;
  const output = invoke();
  context = output.context;
  steps[name] = {
    reportedGas: output.gasCost,
    queries: queryCosts.slice(queryStart),
    publicTranscript: output.proofData.publicTranscript,
    privateOutputCount: output.proofData.privateTranscriptOutputs.length,
  };
}

function normalize(value) {
  if (typeof value === 'bigint') return value.toString();
  if (value instanceof Uint8Array) return { bytesHex: Buffer.from(value).toString('hex') };
  if (Array.isArray(value)) return value.map(normalize);
  if (value && typeof value === 'object') {
    return Object.fromEntries(Object.entries(value).map(([key, inner]) => [key, normalize(inner)]));
  }
  return value;
}

process.stdout.write(JSON.stringify(normalize(steps), null, 2) + '\n');
