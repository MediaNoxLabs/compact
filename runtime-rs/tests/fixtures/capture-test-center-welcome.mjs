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

// Compile the exact test-center/test-contracts/welcome.compact with --target ts
// --skip-zk, link to the matching compact-runtime, then pass both index.js paths.
import { pathToFileURL } from 'node:url';

const [contractPath, runtimePath] = process.argv.slice(2);
if (!contractPath || !runtimePath) throw new Error('expected contract and runtime paths');
const runtime = await import(pathToFileURL(runtimePath).href);
const { Contract, ledger } = await import(pathToFileURL(contractPath).href);

const queries = [];
const originalQuery = runtime.QueryContext.prototype.query;
runtime.QueryContext.prototype.query = function (...args) {
  const result = originalQuery.call(this, ...args);
  queries.push({
    opTags: args[0].map((operation) => Object.keys(operation)[0]),
    gasCost: Object.fromEntries(
      Object.entries(result.gasCost).map(([key, value]) => [key, value.toString()]),
    ),
  });
  return result;
};

const witnessCalls = [];
const contract = new Contract({
  local_sk: ({ privateState }) => {
    witnessCalls.push(privateState);
    return [privateState, { is_some: true, value: new Uint8Array(32) }];
  },
  set_local_id: ({ privateState }) => [privateState, []],
});
const coinPublicKey = { bytes: new Uint8Array(32) };
const cases = [];
for (const present of [false, true]) {
  queries.length = 0;
  witnessCalls.length = 0;
  const participants = Array.from({ length: 5000 }, () => ({
    is_some: false, value: '',
  }));
  if (present) participants[7] = { is_some: true, value: 'alice' };
  const initial = contract.initialState({
    initialPrivateState: 7,
    initialZswapLocalState: runtime.emptyZswapLocalState(coinPublicKey),
  }, participants);
  const constructorQueries = queries.map((query) => structuredClone(query));
  const view = ledger(initial.currentContractState.data);
  cases.push({
    present,
    stateHex: Buffer.from(initial.currentContractState.serialize()).toString('hex'),
    privateState: initial.currentPrivateState,
    witnessCalls: [...witnessCalls],
    eligibleAlice: view.eligible_participants.member('alice'),
    eligibleSize: view.eligible_participants.size().toString(),
    organizerSize: view.organizer_pks.size().toString(),
    constructorQueries,
  });
}
process.stdout.write(JSON.stringify({
  source: 'test-center/test-contracts/welcome.compact',
  cases,
}, null, 2) + '\n');
