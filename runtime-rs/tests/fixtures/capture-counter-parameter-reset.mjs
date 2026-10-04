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

// Compile counter_parameter.compact with --skip-zk, link generated
// contract/node_modules/@midnight-ntwrk/compact-runtime to runtime,
// then pass contract/index.js as the argument.
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
let context = runtime.createCircuitContext(
  runtime.dummyContractAddress(),
  coinPublicKey,
  initial.currentContractState.data,
  initial.currentPrivateState,
);
function stateHex() {
  initial.currentContractState.data = new runtime.ChargedState(
    context.currentQueryContext.state.state,
  );
  return Buffer.from(initial.currentContractState.serialize()).toString('hex');
}
function shape(operation) {
  if (typeof operation === 'string') return { kind: operation };
  if (operation.idx) {
    const { cached, pushPath, path } = operation.idx;
    return { kind: 'idx', cached, pushPath, pathLength: path.length };
  }
  if (operation.push) return { kind: 'push', storage: operation.push.storage };
  if (operation.ins) return { kind: 'ins', cached: operation.ins.cached, n: operation.ins.n };
  if (operation.popeq) {
    return {
      kind: 'popeq', cached: operation.popeq.cached,
      resultAtoms: operation.popeq.result.value.map((atom) => Array.from(atom)),
    };
  }
  throw new Error(`unexpected operation: ${Object.keys(operation)}`);
}
function gas(result) {
  return Object.fromEntries(
    Object.entries(result.gasCost).map(([name, value]) => [name, value.toString()]),
  );
}
const before = stateHex();
const increment = contract.circuits.increment_by(context, 7n);
context = increment.context;
const afterIncrement = stateHex();
const reset = contract.circuits.reset_round(context);
context = reset.context;
process.stdout.write(JSON.stringify({
  before,
  afterIncrement,
  afterReset: stateHex(),
  resetGas: gas(reset),
  resetTranscript: reset.proofData.publicTranscript.map(shape),
  resetPrivateTranscriptCount: reset.proofData.privateTranscriptOutputs.length,
}, null, 2) + '\n');
