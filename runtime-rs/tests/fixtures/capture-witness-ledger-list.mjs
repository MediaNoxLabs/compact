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

// Compile witness_ledger_list with --skip-zk, link its contract runtime to this
// branch's runtime, then run: node capture-witness-ledger-list.mjs <contract/index.js>.
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

const [contractPath] = process.argv.slice(2);
if (!contractPath) throw new Error('expected contract/index.js');
const { Contract } = await import(pathToFileURL(contractPath).href);
const witnesses = {
  first_is_42: ({ ledger, privateState }) => {
    const list = ledger.items;
    const head = list.head();
    const empty = list.isEmpty();
    if (empty !== (list.length() === 0n)) throw new Error('List length mismatch');
    if (empty !== !head.is_some) throw new Error('List head mismatch');
    return [privateState + 1, head.is_some && head.value === 42n];
  },
};
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

function read(ctx) {
  const start = queryCosts.length;
  const result = contract.circuits.private_first_is_42(ctx);
  return { context: result.context, output: output(result, queryCosts.slice(start)) };
}

function stateHex(ctx) {
  initial.currentContractState.data = new runtime.ChargedState(
    ctx.currentQueryContext.state.state,
  );
  return Buffer.from(initial.currentContractState.serialize()).toString('hex');
}

const before = read(context);
context = contract.circuits.prepend(before.context, 42n).context;
const afterState = stateHex(context);
const after = read(context);
context = contract.circuits.prepend(after.context, 7n).context;
const coveredState = stateHex(context);
const covered = read(context);
context = contract.circuits.drop_first(covered.context).context;
const restoredState = stateHex(context);
const restored = read(context);
process.stdout.write(JSON.stringify({
  initialState, afterState, coveredState, restoredState,
  before: before.output, after: after.output,
  covered: covered.output, restored: restored.output,
}, null, 2) + '\n');
