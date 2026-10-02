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

// Capture a Compact MerkleTreePath witness through ledger-8 TypeScript output.
import { createRequire } from 'node:module';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

if (process.argv.length !== 3) throw new Error('usage: node merkle_path_witness_capture.mjs <compiled-contract-dir>');
const contractIndex = resolve(process.argv[2], 'index.js');
const requireFromContract = createRequire(contractIndex);
const runtime = await import(pathToFileURL(requireFromContract.resolve('@midnight-ntwrk/compact-runtime')));
const { Contract } = await import(pathToFileURL(contractIndex));
const queryCosts = [];
const originalQuery = runtime.QueryContext.prototype.query;
runtime.QueryContext.prototype.query = function (...args) {
  const result = originalQuery.call(this, ...args);
  queryCosts.push({
    gasCost: result.gasCost,
    opTags: args[0].map((op) => typeof op === 'string' ? op : Object.keys(op)[0]),
  });
  return result;
};
const contract = new Contract({
  leaf_path: ({ ledger, privateState }) => [privateState, ledger.t.pathForLeaf(0n, 7n)],
  historic_path: ({ ledger, privateState }) => {
    const root = ledger.h.root();
    if (ledger.h.firstFree() !== 1n) throw new Error('historic first-free mismatch');
    if (!Array.from(ledger.h.history()).some((entry) => entry.field === root.field)) {
      throw new Error('historic root missing from history');
    }
    return [privateState, ledger.h.pathForLeaf(0n, 7n)];
  },
  merkle_checks: ({ ledger, privateState }) => {
    const root = ledger.t.root();
    const full = ledger.t.isFull();
    const known = ledger.t.checkRoot(root);
    return [privateState, !full && known];
  },
  historic_checks: ({ ledger, privateState }) => {
    const root = ledger.h.root();
    const prior = Array.from(ledger.h.history()).find((entry) => entry.field !== root.field);
    if (!prior) throw new Error('historic prior root missing');
    const full = ledger.h.isFull();
    const known = ledger.h.checkRoot(root);
    const knownPrior = ledger.h.checkRoot(prior);
    return [privateState, !full && known && knownPrior];
  },
});
const coinPublicKey = { bytes: new Uint8Array(32) };
const initial = contract.initialState({
  initialPrivateState: null,
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
context = contract.circuits.append(context, 7n).context;
const afterPlainInsert = stateHex();
function capture(name) {
  const queryStart = queryCosts.length;
  const output = contract.circuits[name](context);
  return { output, queries: queryCosts.slice(queryStart) };
}
const { output, queries } = capture('get_path');
const path = output.result;
const normalize = (value) => JSON.parse(JSON.stringify(value, (_, item) =>
  typeof item === 'bigint' ? item.toString() : item));
context = contract.circuits.append_h(context, 7n).context;
const afterHistoricInsert = stateHex();
const historic = capture('get_historic_path');
const plainReads = capture('check_witness_merkle');
const historicReads = capture('check_witness_history');
const readOutput = ({ output: result, queries: reads }) => ({
  result: result.result,
  reportedGas: normalize(result.gasCost),
  queries: normalize(reads),
  publicTranscript: normalize(result.proofData.publicTranscript),
  privateTranscriptOutputs: result.proofData.privateTranscriptOutputs.map(
    ({ value, alignment }) => ({
      valueAtoms: value.map(atom => Array.from(atom)),
      alignment,
    }),
  ),
});
process.stdout.write(JSON.stringify({
  afterPlainInsert,
  afterHistoricInsert,
  leaf: path.leaf.toString(),
  path: path.path.map(entry => ({
    sibling: entry.sibling.field.toString(),
    goesLeft: entry.goes_left,
  })),
  reportedGas: normalize(output.gasCost),
  queries: normalize(queries),
  publicTranscript: normalize(output.proofData.publicTranscript),
  privateTranscriptOutputs: output.proofData.privateTranscriptOutputs.map(
    ({ value, alignment }) => ({
      valueAtoms: value.map(atom => Array.from(atom)),
      alignment,
    }),
  ),
  historic: {
    leaf: historic.output.result.leaf.toString(),
    path: historic.output.result.path.map(entry => ({
      sibling: entry.sibling.field.toString(),
      goesLeft: entry.goes_left,
    })),
    reportedGas: normalize(historic.output.gasCost),
    queries: normalize(historic.queries),
    publicTranscript: normalize(historic.output.proofData.publicTranscript),
    privateTranscriptOutputs: readOutput(historic).privateTranscriptOutputs,
  },
  plainReads: readOutput(plainReads),
  historicReads: readOutput(historicReads),
}, null, 2) + '\n');
