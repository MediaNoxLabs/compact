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

// Compile a witness_ledger_set or witness_ledger_map fixture with --skip-zk,
// link its contract runtime to this branch's runtime, then run. The capture
// records each witness read query cost and the serialized state before/after
// the public write:
// node capture-witness-ledger-collection.mjs <contract/index.js> <set|map>.
import { pathToFileURL } from 'node:url';
import * as runtime from '../../../runtime/dist/index.js';

const queryCosts = [];
const originalQuery = runtime.QueryContext.prototype.query;
runtime.QueryContext.prototype.query = function (...args) {
  const result = originalQuery.call(this, ...args);
  queryCosts.push(Object.fromEntries(
    Object.entries(result.gasCost).map(([name, value]) => [name, value.toString()]),
  ));
  return result;
};

const [contractPath, kind] = process.argv.slice(2);
if (!contractPath || !['set', 'map'].includes(kind)) {
  throw new Error('expected contract/index.js and set or map');
}
const { Contract } = await import(pathToFileURL(contractPath).href);
const witnesses = kind === 'set'
  ? { contains_true: ({ ledger, privateState }) => {
    const set = ledger.seen;
    if (set.isEmpty() !== (set.size() === 0n)) throw new Error('Set size mismatch');
    return [privateState + 1, set.member(true)];
  } }
  : { has_true: ({ ledger, privateState }) => {
    const map = ledger.table;
    const present = map.member(true);
    if (map.isEmpty() !== (map.size() === 0n)) throw new Error('Map size mismatch');
    if (present && map.lookup(true) !== 42n) throw new Error('Map lookup mismatch');
    return [privateState + 1, present];
  } };
const contract = new Contract(witnesses);
const coinPublicKey = { bytes: new Uint8Array(32) };
const initial = contract.initialState({
  initialPrivateState: 7,
  initialZswapLocalState: runtime.emptyZswapLocalState(coinPublicKey),
});
const initialState = Buffer.from(initial.currentContractState.serialize()).toString('hex');
let context = runtime.createCircuitContext(
  runtime.dummyContractAddress(),
  coinPublicKey,
  initial.currentContractState.data,
  initial.currentPrivateState,
);

function output(result, costs) {
  return {
    result: result.result,
    privateState: result.context.currentPrivateState,
    privateTranscriptOutputs: result.proofData.privateTranscriptOutputs.map(
      ({ value, alignment }) => ({
        valueAtoms: value.map((atom) => Array.from(atom)),
        alignment,
      }),
    ),
    queryCosts: costs,
  };
}

const read = (ctx) => kind === 'set'
  ? contract.circuits.private_contains(ctx)
  : contract.circuits.private_has(ctx);
const write = (ctx) => kind === 'set'
  ? contract.circuits.add_true(ctx)
  : contract.circuits.put_true(ctx, 42n);
const beforeStart = queryCosts.length;
const before = read(context);
const beforeCosts = queryCosts.slice(beforeStart);
context = write(before.context).context;
initial.currentContractState.data = new runtime.ChargedState(
  context.currentQueryContext.state.state,
);
const afterState = Buffer.from(initial.currentContractState.serialize()).toString('hex');
const afterStart = queryCosts.length;
const after = read(context);
const afterCosts = queryCosts.slice(afterStart);
process.stdout.write(JSON.stringify({ initialState, afterState, before: output(before, beforeCosts), after: output(after, afterCosts) }, null, 2) + '\n');
