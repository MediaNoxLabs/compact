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

// Compile ternary_cond_oracle.compact for TS, then pass contract/index.js. Capture both conditional Field vector branches.
import { pathToFileURL } from 'node:url';
import * as runtime from '../../../runtime/dist/index.js';

const [contractPath] = process.argv.slice(2);
if (!contractPath) throw new Error('expected contract/index.js');
const { Contract } = await import(pathToFileURL(contractPath).href);
const contract = new Contract({ echoField: (context, value) => [context.privateState, value] });
const coinPublicKey = { bytes: new Uint8Array(32) };
const hex = (state) => Buffer.from(state.serialize()).toString('hex');
function initialState(seedFlag) {
  const initial = contract.initialState({
    initialPrivateState: null,
    initialZswapLocalState: runtime.emptyZswapLocalState(coinPublicKey),
  }, true, true, 111n);
  if (seedFlag) {
    const context = runtime.createCircuitContext(
      runtime.dummyContractAddress(), coinPublicKey,
      initial.currentContractState.data, initial.currentPrivateState,
    );
    const partialProofData = {
      input: { value: [], alignment: [] }, output: undefined,
      publicTranscript: [], privateTranscriptOutputs: [],
    };
    const index = new runtime.CompactTypeUnsignedInteger(255n, 1);
    runtime.queryLedgerState(context, partialProofData, [
      { push: { storage: false, value: runtime.StateValue.newCell({
        value: index.toValue(0n), alignment: index.alignment(),
      }).encode() } },
      { push: { storage: true, value: runtime.StateValue.newCell({
        value: runtime.CompactTypeBoolean.toValue(true),
        alignment: runtime.CompactTypeBoolean.alignment(),
      }).encode() } },
      { ins: { cached: false, n: 1 } },
    ]);
    initial.currentContractState.data = new runtime.ChargedState(context.currentQueryContext.state.state);
  }
  return initial;
}
let queries = [];
const originalQuery = runtime.QueryContext.prototype.query;
runtime.QueryContext.prototype.query = function (...args) {
  const output = originalQuery.call(this, ...args);
  queries.push({ gasCost: Object.fromEntries(
    Object.entries(output.gasCost).map(([key, value]) => [key, value.toString()]),
  ) });
  return output;
};
function shape(operation) {
  if (typeof operation === 'string') return { kind: operation };
  if (operation.idx) {
    const { cached, pushPath, path } = operation.idx;
    return { kind: 'idx', cached, pushPath, pathLength: path.length };
  }
  if (operation.push) return { kind: 'push', storage: operation.push.storage };
  if (operation.ins) return { kind: 'ins', cached: operation.ins.cached, n: operation.ins.n };
  if (operation.rem) return { kind: 'rem', cached: operation.rem.cached };
  if (operation.dup) return { kind: 'dup', n: operation.dup.n };
  if (operation.popeq) return {
    kind: 'popeq', cached: operation.popeq.cached,
    resultAtoms: operation.popeq.result.value.map((atom) => Array.from(atom)),
  };
  if (operation.branch) return { kind: 'branch', skip: operation.branch.skip };
  if (operation.swap) return { kind: 'swap', n: operation.swap.n };
  if (operation.concat) return { kind: 'concat', cached: operation.concat.cached, n: operation.concat.n };
  if (operation.jmp) return { kind: 'jmp', skip: operation.jmp.skip };
  if (operation.addi) return { kind: 'addi', immediate: operation.addi.immediate };
  throw new Error(`unexpected operation: ${Object.keys(operation)}`);
}
function afterState(initial, output) {
  const original = initial.currentContractState;
  const state = new runtime.ContractState();
  state.data = new runtime.ChargedState(output.context.currentQueryContext.state.state);
  for (const entry of original.operations()) state.setOperation(entry, original.operation(entry));
  state.maintenanceAuthority = original.maintenanceAuthority;
  state.balance = original.balance;
  return state;
}
function capture(name, args, seedFlag = false) {
  queries = [];
  const initial = initialState(seedFlag);
  queries = [];
  const initialStateHex = hex(initial.currentContractState);
  const context = runtime.createCircuitContext(
    runtime.dummyContractAddress(), coinPublicKey,
    initial.currentContractState.data, initial.currentPrivateState,
  );
  const output = contract.circuits[name](context, ...args);
  return {
    initialStateHex,
    afterStateHex: hex(afterState(initial, output)),
    result: String(output.result),
    reportedGas: Object.fromEntries(
      Object.entries(output.gasCost).map(([key, value]) => [key, value.toString()]),
    ),
    publicTranscriptShape: output.proofData.publicTranscript.map(shape),
    privateTranscriptCount: output.proofData.privateTranscriptOutputs.length,
    queries,
  };
}
process.stdout.write(JSON.stringify({
  streamFalse: capture('streamVectorElement', []),
  streamTrue: capture('streamVectorElement', [], true),
}, null, 2) + '\n');
