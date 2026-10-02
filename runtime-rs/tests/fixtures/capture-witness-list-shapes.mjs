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

// Compile witness_list_shapes.compact with ledger-8 TypeScript, link this
// branch's runtime package, then pass the generated contract/index.js path.
import { pathToFileURL } from 'node:url';
import * as runtime from '../../../runtime/dist/index.js';

const [contractPath] = process.argv.slice(2);
if (!contractPath) throw new Error('expected contract/index.js');
const { Contract, Choice } = await import(pathToFileURL(contractPath).href);
const normalize = (value) => JSON.parse(JSON.stringify(value, (_, item) =>
  typeof item === 'bigint' ? item.toString() : item));
const normalizeVm = (value) => JSON.parse(JSON.stringify(value, (_, item) => {
  if (typeof item === 'bigint') return item.toString();
  if (item instanceof Uint8Array) return { bytesHex: Buffer.from(item).toString('hex') };
  return item;
}));
const queries = [];
const originalQuery = runtime.QueryContext.prototype.query;
runtime.QueryContext.prototype.query = function (...args) {
  const result = originalQuery.call(this, ...args);
  queries.push({
    gasCost: normalize(result.gasCost),
    opTags: args[0].map((op) => typeof op === 'string' ? op : Object.keys(op)[0]),
  });
  return result;
};
const observed = [];
function inspect(name, list) {
  const head = list.head();
  const empty = list.isEmpty();
  const length = list.length();
  if (empty !== (length === 0n) || empty !== !head.is_some) {
    throw new Error(`${name} List shape mismatch`);
  }
  observed.push({ name, head: normalize(head), empty, length: length.toString() });
  return head.is_some;
}
const contract = new Contract({
  inspect_flags: ({ ledger, privateState }) => [privateState + 1, inspect('flags', ledger.flags)],
  inspect_counts: ({ ledger, privateState }) => [privateState + 1, inspect('counts', ledger.counts)],
  inspect_tags: ({ ledger, privateState }) => [privateState + 1, inspect('tags', ledger.tags)],
  inspect_choices: ({ ledger, privateState }) => [privateState + 1, inspect('choices', ledger.choices)],
  inspect_packets: ({ ledger, privateState }) => [privateState + 1, inspect('packets', ledger.packets)],
});
const coinPublicKey = { bytes: new Uint8Array(32) };
const initial = contract.initialState({
  initialPrivateState: 0,
  initialZswapLocalState: runtime.emptyZswapLocalState(coinPublicKey),
});
let context = runtime.createCircuitContext(
  runtime.dummyContractAddress(), coinPublicKey,
  initial.currentContractState.data, initial.currentPrivateState,
);
function stateHex() {
  initial.currentContractState.data = new runtime.ChargedState(context.currentQueryContext.state.state);
  return Buffer.from(initial.currentContractState.serialize()).toString('hex');
}
function read(name) {
  const q = queries.length;
  const o = observed.length;
  const output = contract.circuits[`read_${name}`](context);
  context = output.context;
  return {
    result: output.result,
    privateState: output.context.currentPrivateState,
    privateTranscriptOutputs: output.proofData.privateTranscriptOutputs.map(
      ({ value, alignment }) => ({
        valueAtoms: value.map(atom => Array.from(atom)), alignment,
      }),
    ),
    publicTranscript: normalizeVm(output.proofData.publicTranscript),
    reportedGas: normalize(output.gasCost),
    queries: queries.slice(q),
    observation: observed.slice(o)[0],
  };
}
function readPacketHead() {
  const q = queries.length;
  const output = contract.circuits.first_packet(context);
  context = output.context;
  return {
    result: normalize(output.result),
    privateState: output.context.currentPrivateState,
    publicTranscript: normalizeVm(output.proofData.publicTranscript),
    privateTranscriptOutputs: normalize(output.proofData.privateTranscriptOutputs),
    reportedGas: normalize(output.gasCost),
    queries: queries.slice(q),
  };
}
const names = ['flags', 'counts', 'tags', 'choices', 'packets'];
const initialState = stateHex();
const before = Object.fromEntries(names.map(name => [name, read(name)]));
const emptyPacketHead = readPacketHead();
context = contract.circuits.push_flag(context, true).context;
context = contract.circuits.push_count(context, 42n).context;
context = contract.circuits.push_tag(context, new Uint8Array([1, 2, 3])).context;
context = contract.circuits.push_choice(context, Choice.no).context;
context = contract.circuits.push_packet(context, { tag: new Uint8Array([4, 5, 6]), count: 7n }).context;
const populatedState = stateHex();
const after = Object.fromEntries(names.map(name => [name, read(name)]));
const populatedPacketHead = readPacketHead();
process.stdout.write(JSON.stringify({ initialState, populatedState, before, after, emptyPacketHead, populatedPacketHead }, null, 2) + '\n');
