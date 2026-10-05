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

// Compile the original asset_registry_oracle.compact for TypeScript with
// --skip-zk, link this checkout's runtime, and pass the generated directory.
import { createRequire } from 'node:module';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

const [contractDirectory] = process.argv.slice(2);
if (!contractDirectory) throw new Error('expected generated contract directory');
const index = resolve(contractDirectory, 'index.js');
const requireFromContract = createRequire(index);
const runtime = await import(pathToFileURL(requireFromContract.resolve('@midnight-ntwrk/compact-runtime')));
const { Contract } = await import(pathToFileURL(index));
const coinPublicKey = { bytes: new Uint8Array(32) };

function normalize(value) {
  if (typeof value === 'bigint') return value.toString();
  if (value instanceof Uint8Array) return Array.from(value);
  if (Array.isArray(value)) return value.map(normalize);
  if (value && typeof value === 'object') {
    return Object.fromEntries(Object.entries(value).map(([key, inner]) => [key, normalize(inner)]));
  }
  return value;
}
function shape(op) {
  if (typeof op === 'string') return { kind: op };
  if (op.idx) return { kind: 'idx', cached: op.idx.cached, pushPath: op.idx.pushPath, pathLength: op.idx.path.length };
  if (op.push) return { kind: 'push', storage: op.push.storage };
  if (op.ins) return { kind: 'ins', cached: op.ins.cached, n: op.ins.n };
  if (op.rem) return { kind: 'rem', cached: op.rem.cached };
  if (op.dup) return { kind: 'dup', n: op.dup.n };
  if (op.addi) return { kind: 'addi', immediate: op.addi.immediate };
  if (op.popeq) return { kind: 'popeq', cached: op.popeq.cached };
  return { kind: Object.keys(op)[0] };
}
const queries = [];
let witnessRead = false;
const originalQuery = runtime.QueryContext.prototype.query;
runtime.QueryContext.prototype.query = function (...args) {
  const result = originalQuery.call(this, ...args);
  queries.push({ gasCost: normalize(result.gasCost), opTags: args[0].map((op) => typeof op === 'string' ? op : Object.keys(op)[0]), program: normalize(args[0]), raw: args[0], witnessRead });
  return result;
};
const indexType = new runtime.CompactTypeUnsignedInteger(255n, 1);
function seedCell(context, index, value) {
  const program = [
    { idx: { cached: false, pushPath: true, path: [{ tag: 'value', value: {
      value: indexType.toValue(1n), alignment: indexType.alignment(),
    } }] } },
    { push: { storage: false, value: runtime.StateValue.newCell({
      value: indexType.toValue(BigInt(index)), alignment: indexType.alignment(),
    }).encode() } },
    { push: { storage: true, value: runtime.StateValue.newCell({
      value: runtime.CompactTypeBoolean.toValue(value), alignment: runtime.CompactTypeBoolean.alignment(),
    }).encode() } },
    { ins: { cached: false, n: 1 } },
    { ins: { cached: true, n: 1 } },
  ];
  context.currentQueryContext = context.currentQueryContext.query(program, context.costModel).context;
}
function makeEnv(mode = 'success', seedRecord = true) {
  const witnessCalls = [];
  const contract = new Contract({
    localOperatorKey: ({ privateState }) => {
      witnessCalls.push('localOperatorKey');
      return [privateState + 1, runtime.hashToCurve(runtime.CompactTypeField, 1n)];
    },
    localAuditorKey: ({ privateState }) => {
      witnessCalls.push('localAuditorKey');
      return [privateState + 1, runtime.hashToCurve(runtime.CompactTypeField, 2n)];
    },
    currentTimestamp: ({ ledger, privateState }) => {
      witnessCalls.push('currentTimestamp');
      witnessRead = true;
      void ledger.recordCount;
      void ledger.records;
      void ledger.watchList;
      witnessRead = false;
      return [privateState + 1, 1_700_000_000n];
    },
  });
  const state = contract.initialState({
    initialPrivateState: 7,
    initialZswapLocalState: runtime.emptyZswapLocalState(coinPublicKey),
  });
  witnessCalls.length = 0;
  const context = () => runtime.createCircuitContext(
    runtime.dummyContractAddress(), coinPublicKey,
    state.currentContractState.data, state.currentPrivateState,
  );
  if (seedRecord) {
    const record = { code: new Uint8Array(32), note: '検査資料 🔒', provenance: { facility: new Uint8Array(32).fill(4), registeredAt: 91n }, kind: 1, quantity: 5n };
    const seededRecord = contract.circuits.setRecord(context(), 'record-α-1', record, 1);
    state.currentContractState.data = new runtime.ChargedState(seededRecord.context.currentQueryContext.state.state);
    state.currentPrivateState = seededRecord.context.currentPrivateState;
    witnessCalls.length = 0;
  }
  if (mode !== 'success') {
    const seeded = context();
    seedCell(seeded, mode === 'closed' ? 6 : 7, mode === 'frozen');
    state.currentContractState.data = new runtime.ChargedState(seeded.currentQueryContext.state.state);
  }
  const hex = () => Buffer.from(state.currentContractState.serialize()).toString('hex');
  function call(mutation) {
    const initialStateHex = hex();
    const priorState = state.currentContractState.data;
    const priorPrivate = state.currentPrivateState;
    const start = queries.length;
    witnessCalls.length = 0;
    const output = contract.circuits.setWatch(context(), 'record-α-1', mutation);
    state.currentContractState.data = new runtime.ChargedState(output.context.currentQueryContext.state.state);
    state.currentPrivateState = output.context.currentPrivateState;
    const own = queries.slice(start);
    const program = own.filter((entry) => !entry.witnessRead).flatMap((entry) => entry.raw);
    const replayContext = runtime.createCircuitContext(
      runtime.dummyContractAddress(), coinPublicKey, priorState, priorPrivate,
    );
    const replay = replayContext.currentQueryContext.query(program, replayContext.costModel);
    return {
      result: normalize(output.result), initialStateHex, stateHex: hex(),
      privateState: state.currentPrivateState,
      privateTranscriptOutputs: output.proofData.privateTranscriptOutputs.map(({ value, alignment }) => ({
        valueAtoms: value.map((atom) => Array.from(atom)), alignment,
      })),
      publicTranscriptShape: output.proofData.publicTranscript.map(shape),
      vmProgram: normalize(output.proofData.publicTranscript),
      replayGas: normalize(replay.gasCost),
      queries: own.map(({ gasCost, opTags }) => ({ gasCost, opTags })),
      witnessCalls: [...witnessCalls],
    };
  }
  function rejected(mutation) {
    const initialStateHex = hex();
    const before = queries.length;
    witnessCalls.length = 0;
    try {
      contract.circuits.setWatch(context(), 'record-α-1', mutation);
      return null;
    } catch (error) {
      return { error: String(error.message), initialStateHex, stateHex: hex(),
        privateState: state.currentPrivateState,
        queries: queries.slice(before).map(({ gasCost, opTags }) => ({ gasCost, opTags })),
        witnessCalls: [...witnessCalls] };
    }
  }
  return { call, rejected };
}
const env = makeEnv();
const add = env.call(1);
const duplicateAdd = env.rejected(1);
const drop = env.call(2);
const absentDrop = makeEnv().rejected(2);
const unknownRecord = makeEnv('success', false).rejected(1);
const invalidMutation = makeEnv().rejected(0);
const closed = makeEnv('closed').rejected(1);
const frozen = makeEnv('frozen').rejected(1);
process.stdout.write(JSON.stringify({ add, drop, duplicateAdd, absentDrop, unknownRecord, invalidMutation, closed, frozen }, null, 2) + '\n');
