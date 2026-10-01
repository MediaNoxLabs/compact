// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

import { createRequire } from 'node:module';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

if (process.argv.length !== 3) throw new Error('usage: node vector_key_capture.mjs <compiled-contract-dir>');
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

const afterInit = Buffer.from(initial.currentContractState.serialize()).toString('hex');
context = contract.circuits.setInsert(context).context;
const afterSetInsert = snapshot();
context = contract.circuits.setMember(context).context;
const afterSetMember = snapshot();
const setPresent = ledger(context.currentQueryContext.state).present;
context = contract.circuits.mapInsert(context).context;
const afterMapInsert = snapshot();
context = contract.circuits.mapMember(context).context;
const afterMapMember = snapshot();
const mapPresent = ledger(context.currentQueryContext.state).present;
context = contract.circuits.mapLookup(context).context;
const afterMapLookup = snapshot();
const stored = String(ledger(context.currentQueryContext.state).stored);
context = contract.circuits.setRemove(context).context;
const afterSetRemove = snapshot();
context = contract.circuits.setMember(context).context;
const afterSetMissing = snapshot();
const setMissing = ledger(context.currentQueryContext.state).present;
context = contract.circuits.mapRemove(context).context;
const afterMapRemove = snapshot();
context = contract.circuits.mapMember(context).context;
const afterMapMissing = snapshot();
const mapMissing = ledger(context.currentQueryContext.state).present;
context = contract.circuits.mapInsertDefault(context).context;
const afterMapInsertDefault = snapshot();
context = contract.circuits.mapLookup(context).context;
const afterMapDefaultLookup = snapshot();
const defaultStored = String(ledger(context.currentQueryContext.state).stored);
process.stdout.write(JSON.stringify({
  afterInit, afterSetInsert, afterSetMember, setPresent,
  afterMapInsert, afterMapMember, mapPresent, afterMapLookup, stored,
  afterSetRemove, afterSetMissing, setMissing,
  afterMapRemove, afterMapMissing, mapMissing,
  afterMapInsertDefault, afterMapDefaultLookup, defaultStored,
}, null, 2) + '\n');
