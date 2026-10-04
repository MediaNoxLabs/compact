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

// Capture exact codegen-rust uints_fixture with ledger-8 TypeScript.
import { createRequire } from 'node:module';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

if (process.argv.length !== 3) throw new Error('usage: node uints_capture.mjs <compiled-contract-dir>');
const contractIndex = resolve(process.argv[2], 'index.js');
const requireFromContract = createRequire(contractIndex);
const runtime = await import(pathToFileURL(requireFromContract.resolve('@midnight-ntwrk/compact-runtime')));
const { Contract, ledger } = await import(pathToFileURL(contractIndex));
const contract = new Contract({});
const coinPublicKey = { bytes: new Uint8Array(32) };
const initial = contract.initialState({
  initialPrivateState: null,
  initialZswapLocalState: runtime.emptyZswapLocalState(coinPublicKey),
});
let context = runtime.createCircuitContext(
  runtime.dummyContractAddress(), coinPublicKey,
  initial.currentContractState.data, initial.currentPrivateState,
);
function snapshot() {
  const state = new runtime.ContractState();
  state.data = new runtime.ChargedState(context.currentQueryContext.state.state);
  for (const key of initial.currentContractState.operations()) {
    state.setOperation(key, initial.currentContractState.operation(key));
  }
  state.maintenanceAuthority = initial.currentContractState.maintenanceAuthority;
  state.balance = initial.currentContractState.balance;
  return Buffer.from(state.serialize()).toString('hex');
}
function gas(result) {
  return Object.fromEntries(
    Object.entries(result.gasCost).map(([name, value]) => [name, value.toString()]),
  );
}
function operation(operation) {
  if (typeof operation === 'string') return { kind: operation };
  if (operation.push) return { kind: 'push', storage: operation.push.storage };
  if (operation.ins) return { kind: 'ins', cached: operation.ins.cached, n: operation.ins.n };
  if (operation.idx) {
    const { cached, pushPath, path } = operation.idx;
    return { kind: 'idx', cached, pushPath, pathLength: path.length };
  }
  if (operation.popeq) {
    return {
      kind: 'popeq', cached: operation.popeq.cached,
      resultAtoms: operation.popeq.result.value.map((atom) => Array.from(atom)),
    };
  }
  throw new Error(`unexpected operation: ${Object.keys(operation)}`);
}
const afterInit = Buffer.from(initial.currentContractState.serialize()).toString('hex');
const set255 = contract.circuits.set_byte(context, 255n);
context = set255.context;
const afterSet255 = snapshot();
const byteAfterSet255 = ledger(context.currentQueryContext.state).byte_field.toString();
const set0 = contract.circuits.set_byte(context, 0n);
context = set0.context;
const afterSet0 = snapshot();
process.stdout.write(JSON.stringify({
  afterInit, afterSet255, byteAfterSet255, afterSet0,
  set255Gas: gas(set255),
  set255Transcript: set255.proofData.publicTranscript.map(operation),
  set255PrivateTranscriptCount: set255.proofData.privateTranscriptOutputs.length,
  set0Gas: gas(set0),
  set0Transcript: set0.proofData.publicTranscript.map(operation),
  set0PrivateTranscriptCount: set0.proofData.privateTranscriptOutputs.length,
}, null, 2) + '\n');
