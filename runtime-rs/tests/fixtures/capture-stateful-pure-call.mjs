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

// Compile stateful_pure_call.compact with --skip-zk, link generated contract
// to this branch's runtime, then pass contract/index.js as the argument.
import { pathToFileURL } from 'node:url';
import * as runtime from '../../../runtime/dist/index.js';

const [contractPath] = process.argv.slice(2);
if (!contractPath) throw new Error('expected contract/index.js');
const { Contract, pureCircuits } = await import(pathToFileURL(contractPath).href);
const contract = new Contract({});
const coinPublicKey = { bytes: new Uint8Array(32) };
const queries = [];
const originalQuery = runtime.QueryContext.prototype.query;
runtime.QueryContext.prototype.query = function (...args) {
  const result = originalQuery.call(this, ...args);
  queries.push({ gasCost: Object.fromEntries(
    Object.entries(result.gasCost).map(([name, value]) => [name, value.toString()]),
  ) });
  return result;
};
function shape(operation) {
  if (typeof operation === 'string') return { kind: operation };
  if (operation.push) return { kind: 'push', storage: operation.push.storage };
  if (operation.ins) return { kind: 'ins', cached: operation.ins.cached, n: operation.ins.n };
  throw new Error(`unexpected operation: ${Object.keys(operation)}`);
}
const initial = contract.initialState({
  initialPrivateState: null,
  initialZswapLocalState: runtime.emptyZswapLocalState(coinPublicKey),
});
let context = runtime.createCircuitContext(
  runtime.dummyContractAddress(), coinPublicKey,
  initial.currentContractState.data, initial.currentPrivateState,
);
const queryStart = queries.length;
const saved = contract.circuits.save(context, 7n);
const saveQueryEnd = queries.length;
context = saved.context;
const read = contract.circuits.read_stored(context);
initial.currentContractState.data = new runtime.ChargedState(read.context.currentQueryContext.state.state);
process.stdout.write(JSON.stringify({
  pure: pureCircuits.square(7n).toString(),
  returned: saved.result.toString(),
  stored: read.result.toString(),
  stateHex: Buffer.from(initial.currentContractState.serialize()).toString('hex'),
  saveTrace: {
    publicTranscriptShape: saved.proofData.publicTranscript.map(shape),
    privateTranscriptCount: saved.proofData.privateTranscriptOutputs.length,
    privateState: saved.context.currentPrivateState,
    queries: queries.slice(queryStart, saveQueryEnd),
    reportedGas: Object.fromEntries(
      Object.entries(saved.gasCost).map(([key, value]) => [key, value.toString()]),
    ),
  },
}, null, 2) + '\n');
