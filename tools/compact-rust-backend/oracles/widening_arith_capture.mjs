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

// Capture exact codegen-rust widening_arith_fixture with ledger-8 TypeScript.
import { createRequire } from 'node:module';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

if (process.argv.length !== 3) throw new Error('usage: node widening_arith_capture.mjs <compiled-contract-dir>');
const contractIndex = resolve(process.argv[2], 'index.js');
const requireFromContract = createRequire(contractIndex);
const runtime = await import(pathToFileURL(requireFromContract.resolve('@midnight-ntwrk/compact-runtime')));
const { Contract, pureCircuits, ledger } = await import(pathToFileURL(contractIndex));
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
const afterInit = Buffer.from(initial.currentContractState.serialize()).toString('hex');
const sumBytes = pureCircuits.sumBytes(255n, 255n).toString();
const ageThresholdDays = pureCircuits.ageThresholdDays(255n).toString();
const productBytes = pureCircuits.productBytes(255n, 255n).toString();
const areaOf = pureCircuits.areaOf(65535n, 65535n).toString();
context = contract.circuits.recordArea(context, 65535n, 65535n).context;
const afterRecordMax = snapshot();
const countAfterRecordMax = ledger(context.currentQueryContext.state).lastArea.toString();
context = contract.circuits.recordArea(context, 5n, 7n).context;
const afterRecordSmall = snapshot();
const countAfterRecordSmall = ledger(context.currentQueryContext.state).lastArea.toString();
process.stdout.write(JSON.stringify({
  afterInit, sumBytes, ageThresholdDays, productBytes, areaOf,
  afterRecordMax, countAfterRecordMax, afterRecordSmall, countAfterRecordSmall,
}, null, 2) + '\n');
