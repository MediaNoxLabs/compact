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

// Capture ledger-8 TypeScript state for merkle_tree_oracle.compact.
// Pass a compiled contract directory with @midnight-ntwrk/compact-runtime linked.
import { createRequire } from 'node:module';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
import { normalizeQueryProgram } from './normalize_query_program.mjs';

if (process.argv.length !== 3) throw new Error('usage: node merkle_tree_capture.mjs <compiled-contract-dir>');
const contractIndex = resolve(process.argv[2], 'index.js');
const requireFromContract = createRequire(contractIndex);
const runtime = await import(pathToFileURL(requireFromContract.resolve('@midnight-ntwrk/compact-runtime')));
const { Contract, ledger } = await import(pathToFileURL(contractIndex));
const queryCosts = [];
const originalQuery = runtime.QueryContext.prototype.query;
runtime.QueryContext.prototype.query = function (...args) {
  const result = originalQuery.call(this, ...args);
  queryCosts.push({
    gasCost: result.gasCost,
    opTags: args[0].map((op) => typeof op === 'string' ? op : Object.keys(op)[0]),
    program: normalizeQueryProgram(args[0]),
  });
  return result;
};
const nativeQueries = {};
function capture(name, invoke) {
  const start = queryCosts.length;
  const output = invoke();
  nativeQueries[name] = {
    result: output.result,
    privateOutputs: output.proofData.privateTranscriptOutputs.length,
    reportedGas: output.gasCost,
    queries: queryCosts.slice(start),
  };
  return output;
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
function full() {
  const out = contract.circuits.full(context);
  context = out.context;
  return out.result;
}
function known(root) {
  const out = contract.circuits.known(context, root);
  context = out.context;
  return out.result;
}
function currentRoot() {
  return ledger(new runtime.ChargedState(context.currentQueryContext.state.state)).t.root();
}
function currentTree() {
  return ledger(new runtime.ChargedState(context.currentQueryContext.state.state)).t;
}
function pathData(path) {
  if (!path) return null;
  return {
    leaf: path.leaf.toString(),
    path: path.path.map(entry => ({
      sibling: entry.sibling.field.toString(),
      goesLeft: entry.goes_left,
    })),
  };
}
const afterInit = Buffer.from(initial.currentContractState.serialize()).toString('hex');
const rootAtInit = ledger(initial.currentContractState.data).t.root();
const fullAtInitOutput = capture('fullAtInit', () => contract.circuits.full(context));
context = fullAtInitOutput.context;
const fullAtInit = fullAtInitOutput.result;
const knownAtInitOutput = capture('knownAtInit', () => contract.circuits.known(context, rootAtInit));
context = knownAtInitOutput.context;
const knownAtInit = knownAtInitOutput.result;
context = capture('append7', () => contract.circuits.append(context, 7n)).context;
const afterAppend7 = snapshot();
const pathFor7At0 = pathData(currentTree().pathForLeaf(0n, 7n));
const wrongPathFor8At0 = pathData(currentTree().pathForLeaf(0n, 8n));
const foundPathFor7 = pathData(currentTree().findPathForLeaf(7n));
const missingPathFor8 = pathData(currentTree().findPathForLeaf(8n));
const knownInitialAfterAppend = known(rootAtInit);
context = capture('place9At3', () => contract.circuits.place(context, 9n, 3n)).context;
const afterPlace9At3 = snapshot();
const pathFor9At3 = pathData(currentTree().pathForLeaf(3n, 9n));
context = contract.circuits.append(context, 11n).context;
const afterAppend11 = snapshot();
context = capture('place13At1', () => contract.circuits.place(context, 13n, 1n)).context;
const afterPlace13At1 = snapshot();
context = capture('placeDefaultAt6', () => contract.circuits.place_default(context, 6n)).context;
const afterDefaultAt6 = snapshot();
const fullBeforeCapacity = full();
context = capture('appendHash', () => contract.circuits.append_hash(context, new Uint8Array(32).fill(1))).context;
const afterAppendHash = snapshot();
const fullAtCapacity = full();
context = contract.circuits.place_hash(context, new Uint8Array(32).fill(2), 1n).context;
const afterReplaceHashAt1 = snapshot();
const fullAfterReplacement = full();
const rootBeforeTreeReset = currentRoot();
const knownCurrent = known(rootBeforeTreeReset);
const knownInitialBeforeReset = known(rootAtInit);
context = contract.circuits.reset_tree(context).context;
const afterResetTree = snapshot();
const fullAfterTreeReset = full();
const knownOldAfterTreeReset = known(rootBeforeTreeReset);
const knownBlankAfterTreeReset = known(rootAtInit);
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
  afterInit, fullAtInit, knownAtInit, afterAppend7, pathFor7At0, wrongPathFor8At0, foundPathFor7,
  missingPathFor8, knownInitialAfterAppend,
  afterPlace9At3, pathFor9At3, afterAppend11, afterPlace13At1, afterDefaultAt6,
  fullBeforeCapacity, afterAppendHash, fullAtCapacity, afterReplaceHashAt1,
  fullAfterReplacement, knownCurrent, knownInitialBeforeReset, afterResetTree,
  fullAfterTreeReset, knownOldAfterTreeReset, knownBlankAfterTreeReset, nativeQueries,
}), null, 2) + '\n');
