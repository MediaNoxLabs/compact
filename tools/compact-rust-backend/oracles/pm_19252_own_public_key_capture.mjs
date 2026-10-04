// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//   http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

// Capture the original PM-19252 source with the pinned TypeScript runtime.
import { createRequire } from 'node:module';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

if (process.argv.length !== 3) {
  throw new Error('usage: node pm_19252_own_public_key_capture.mjs <compiled-contract-dir>');
}
const contractIndex = resolve(process.argv[2], 'index.js');
const requireFromContract = createRequire(contractIndex);
const runtime = await import(pathToFileURL(requireFromContract.resolve('@midnight-ntwrk/compact-runtime')));
const { Contract } = await import(pathToFileURL(contractIndex));
const contract = new Contract({});
const coinPublicKey = { bytes: Uint8Array.from({ length: 32 }, (_, i) => i + 1) };
const initial = contract.initialState({
  initialPrivateState: null,
  initialZswapLocalState: runtime.emptyZswapLocalState(coinPublicKey),
});
const before = Buffer.from(initial.currentContractState.serialize()).toString('hex');
const context = runtime.createCircuitContext(
  runtime.dummyContractAddress(), coinPublicKey,
  initial.currentContractState.data, initial.currentPrivateState,
);
const output = contract.circuits.test1(context);
initial.currentContractState.data = new runtime.ChargedState(output.context.currentQueryContext.state.state);
const after = Buffer.from(initial.currentContractState.serialize()).toString('hex');
const value = {
  source: 'examples/bugs/pm-19252/example_ten.compact',
  exportedCircuit: 'test1',
  proofRequired: Object.hasOwn(contract.provableCircuits, 'test1'),
  coinPublicKey: Array.from(coinPublicKey.bytes),
  result: output.result,
  gasCost: Object.fromEntries(Object.entries(output.gasCost).map(([key, cost]) => [key, cost.toString()])),
  publicTranscript: output.proofData.publicTranscript,
  privateTranscriptOutputs: output.proofData.privateTranscriptOutputs.map(({ value, alignment }) => ({
    valueAtoms: value.map(atom => Array.from(atom)),
    alignment,
  })),
  stateBefore: before,
  stateAfter: after,
};
process.stdout.write(`${JSON.stringify(value, null, 2)}\n`);
