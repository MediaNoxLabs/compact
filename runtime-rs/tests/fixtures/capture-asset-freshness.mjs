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

// Compile asset_registry_oracle.compact with --target ts --skip-zk, link its
// contract/node_modules/@midnight-ntwrk/compact-runtime to this runtime, and
// pass the generated contract directory.
import { createRequire } from 'node:module';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

const [contractDirectory] = process.argv.slice(2);
if (!contractDirectory) throw new Error('expected generated contract directory');
const index = resolve(contractDirectory, 'index.js');
const requireFromContract = createRequire(index);
const runtime = await import(pathToFileURL(requireFromContract.resolve('@midnight-ntwrk/compact-runtime')));
const { Contract } = await import(pathToFileURL(index));

const queries = [];
const originalQuery = runtime.QueryContext.prototype.query;
runtime.QueryContext.prototype.query = function (...args) {
  const result = originalQuery.call(this, ...args);
  queries.push({
    gasCost: Object.fromEntries(Object.entries(result.gasCost).map(([name, value]) => [name, value.toString()])),
    opTags: args[0].map((op) => typeof op === 'string' ? op : Object.keys(op)[0]),
  });
  return result;
};

function shape(op) {
  if (typeof op === 'string') return { kind: op };
  if (op.idx) return { kind: 'idx', cached: op.idx.cached, pushPath: op.idx.pushPath, pathLength: op.idx.path.length };
  if (op.push) return { kind: 'push', storage: op.push.storage };
  if (op.ins) return { kind: 'ins', cached: op.ins.cached, n: op.ins.n };
  if (op.dup) return { kind: 'dup', n: op.dup.n };
  if (op.addi) return { kind: 'addi', immediate: op.addi.immediate };
  if (op.popeq) return { kind: 'popeq', cached: op.popeq.cached };
  return { kind: Object.keys(op)[0] };
}

function capture(name, policy, registeredAt, currentTime, note) {
  const witnessCalls = [];
  const point1 = runtime.hashToCurve(runtime.CompactTypeField, 1n);
  const point2 = runtime.hashToCurve(runtime.CompactTypeField, 2n);
  const contract = new Contract({
    localOperatorKey: ({ privateState }) => {
      witnessCalls.push('localOperatorKey');
      return [privateState + 1, point1];
    },
    localAuditorKey: ({ privateState }) => {
      witnessCalls.push('localAuditorKey');
      return [privateState + 1, point2];
    },
    currentTimestamp: ({ ledger, privateState }) => {
      witnessCalls.push('currentTimestamp');
      void ledger.recordCount;
      void ledger.records;
      void ledger.watchList;
      return [privateState + 1, 1_700_000_000n];
    },
  });
  const coinPublicKey = { bytes: new Uint8Array(32) };
  const initial = contract.initialState({
    initialPrivateState: 7,
    initialZswapLocalState: runtime.emptyZswapLocalState(coinPublicKey),
  });
  witnessCalls.length = 0;
  const context = runtime.createCircuitContext(
    runtime.dummyContractAddress(), coinPublicKey,
    initial.currentContractState.data, initial.currentPrivateState,
  );
  const initialStateHex = Buffer.from(initial.currentContractState.serialize()).toString('hex');
  const before = queries.length;
  const record = {
    code: new Uint8Array(32).fill(3), note,
    provenance: { facility: new Uint8Array(32).fill(4), registeredAt },
    kind: 1, quantity: 5n,
  };
  try {
    const output = contract.circuits.acceptIfFresh(context, policy, record, currentTime);
    initial.currentContractState.data = new runtime.ChargedState(output.context.currentQueryContext.state.state);
    return {
      name, initialStateHex,
      stateHex: Buffer.from(initial.currentContractState.serialize()).toString('hex'),
      privateState: output.context.currentPrivateState,
      result: output.result,
      privateTranscriptOutputs: output.proofData.privateTranscriptOutputs.map(({ value, alignment }) => ({
        valueAtoms: value.map((atom) => Array.from(atom)), alignment,
      })),
      publicTranscriptShape: output.proofData.publicTranscript.map(shape),
      queries: queries.slice(before), witnessCalls,
    };
  } catch (error) {
    return {
      name, initialStateHex, stateHex: initialStateHex,
      privateState: context.currentPrivateState,
      error: error.message, queries: queries.slice(before), witnessCalls,
    };
  }
}

const cases = {
  fresh: capture('fresh', { enforceMaxAge: true, maxAge: 50n }, 100n, 120n, 'valid note'),
  unchecked_age: capture('unchecked_age', { enforceMaxAge: false, maxAge: 0n }, 100n, 500n, 'valid note'),
  future: capture('future', { enforceMaxAge: true, maxAge: 50n }, 130n, 120n, 'valid note'),
  expired: capture('expired', { enforceMaxAge: true, maxAge: 10n }, 100n, 120n, 'valid note'),
};
process.stdout.write(JSON.stringify(cases, null, 2) + '\n');
