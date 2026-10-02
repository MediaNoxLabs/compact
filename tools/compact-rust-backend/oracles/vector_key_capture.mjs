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
import { normalizeQueryProgram } from './normalize_query_program.mjs';

if (process.argv.length !== 3) throw new Error('usage: node vector_key_capture.mjs <compiled-contract-dir>');
const contractIndex = resolve(process.argv[2], 'index.js');
const requireFromContract = createRequire(contractIndex);
const runtime = await import(pathToFileURL(requireFromContract.resolve('@midnight-ntwrk/compact-runtime')));
const { Contract, ledger } = await import(pathToFileURL(contractIndex));
const queryCosts = [];
const originalQuery = runtime.QueryContext.prototype.query;
runtime.QueryContext.prototype.query = function (...args) {
  const result = originalQuery.call(this, ...args);
  const program = normalizeQueryProgram(args[0]);
  // The TypeScript query program leaves `popeq.result` undefined. The VM's
  // independent read event carries the observed value required by ledger-8's
  // Rust verification-mode Op; pair them in program order for exact comparison.
  const reads = normalizeQueryProgram(result.events)
    .filter((event) => event.tag === 'read')
    .map((event) => event.content);
  let readIndex = 0;
  for (const operation of program) {
    if (operation.popeq && operation.popeq.result == null) {
      operation.popeq.result = reads[readIndex++];
    }
  }
  if (readIndex !== reads.length) throw new Error('unmatched VM read event');
  queryCosts.push({
    gasCost: result.gasCost,
    opTags: args[0].map((op) => typeof op === 'string' ? op : Object.keys(op)[0]),
    program,
  });
  return result;
};
const nativeQueries = {};
function call(label, circuit) {
  const start = queryCosts.length;
  const output = contract.circuits[circuit](context);
  nativeQueries[label] = { reportedGas: output.gasCost, queries: queryCosts.slice(start) };
  context = output.context;
}
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
call('setInsert', 'setInsert');
const afterSetInsert = snapshot();
call('setMemberPresent', 'setMember');
const afterSetMember = snapshot();
const setPresent = ledger(context.currentQueryContext.state).present;
call('mapInsert', 'mapInsert');
const afterMapInsert = snapshot();
call('mapMemberPresent', 'mapMember');
const afterMapMember = snapshot();
const mapPresent = ledger(context.currentQueryContext.state).present;
call('mapLookupPresent', 'mapLookup');
const afterMapLookup = snapshot();
const stored = String(ledger(context.currentQueryContext.state).stored);
call('setRemove', 'setRemove');
const afterSetRemove = snapshot();
call('setMemberMissing', 'setMember');
const afterSetMissing = snapshot();
const setMissing = ledger(context.currentQueryContext.state).present;
call('mapRemove', 'mapRemove');
const afterMapRemove = snapshot();
call('mapMemberMissing', 'mapMember');
const afterMapMissing = snapshot();
const mapMissing = ledger(context.currentQueryContext.state).present;
call('mapInsertDefault', 'mapInsertDefault');
const afterMapInsertDefault = snapshot();
call('mapLookupDefault', 'mapLookup');
const afterMapDefaultLookup = snapshot();
const defaultStored = String(ledger(context.currentQueryContext.state).stored);
function normalize(value) {
  if (typeof value === 'bigint') return value.toString();
  if (value instanceof Uint8Array) return { bytesHex: Buffer.from(value).toString('hex') };
  if (Array.isArray(value)) return value.map(normalize);
  if (value && typeof value === 'object') {
    return Object.fromEntries(Object.entries(value).map(([key, inner]) => [key, normalize(inner)]));
  }
  return value;
}
process.stdout.write(JSON.stringify(normalize({
  afterInit, afterSetInsert, afterSetMember, setPresent,
  afterMapInsert, afterMapMember, mapPresent, afterMapLookup, stored,
  afterSetRemove, afterSetMissing, setMissing,
  afterMapRemove, afterMapMissing, mapMissing,
  afterMapInsertDefault, afterMapDefaultLookup, defaultStored, nativeQueries,
}), null, 2) + '\n');
