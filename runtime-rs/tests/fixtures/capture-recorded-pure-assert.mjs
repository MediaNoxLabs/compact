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

// Compile pure_call_action.compact and assert_parity_oracle.compact for TS,
// then pass their contract/index.js paths in that order.
import { pathToFileURL } from 'node:url';
import * as runtime from '../../../runtime/dist/index.js';

const [pureCallPath, assertPath] = process.argv.slice(2);
if (!pureCallPath || !assertPath) throw new Error('expected two contract/index.js paths');
const [{ Contract: PureCall }, { Contract: Assert }] = await Promise.all([
  import(pathToFileURL(pureCallPath).href),
  import(pathToFileURL(assertPath).href),
]);
const coinPublicKey = { bytes: new Uint8Array(32) };
const hex = (state) => Buffer.from(state.serialize()).toString('hex');
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
function fresh(contract) {
  const initial = contract.initialState({
    initialPrivateState: null,
    initialZswapLocalState: runtime.emptyZswapLocalState(coinPublicKey),
  });
  const context = runtime.createCircuitContext(
    runtime.dummyContractAddress(), coinPublicKey,
    initial.currentContractState.data, initial.currentPrivateState,
  );
  return { initial, context };
}
function stateAfter(initial, queryState) {
  const state = new runtime.ContractState();
  state.data = new runtime.ChargedState(queryState);
  for (const entry of initial.currentContractState.operations()) {
    state.setOperation(entry, initial.currentContractState.operation(entry));
  }
  state.maintenanceAuthority = initial.currentContractState.maintenanceAuthority;
  state.balance = initial.currentContractState.balance;
  return hex(state);
}
function capture(contract, name, args) {
  const { initial, context } = fresh(contract);
  queries = [];
  const initialStateHex = hex(initial.currentContractState);
  try {
    const output = contract.circuits[name](context, ...args);
    const circuitQueries = queries;
    queries = [];
    return {
      success: true,
      initialStateHex,
      afterStateHex: stateAfter(initial, output.context.currentQueryContext.state.state),
      result: String(output.result),
      reportedGas: Object.fromEntries(
        Object.entries(output.gasCost).map(([key, value]) => [key, value.toString()]),
      ),
      publicTranscriptShape: output.proofData.publicTranscript.map(shape),
      privateTranscriptCount: output.proofData.privateTranscriptOutputs.length,
      queries: circuitQueries,
    };
  } catch (error) {
    return {
      success: false,
      initialStateHex,
      afterStateHex: stateAfter(initial, context.currentQueryContext.state.state),
      error: error.message,
      compactError: error.constructor.name === 'CompactError',
      queryCount: queries.length,
    };
  }
}
const pureCall = new PureCall({});
const assert = new Assert({});
process.stdout.write(JSON.stringify({
  saveSuccess: capture(pureCall, 'save', [7n]),
  saveFailure: capture(pureCall, 'save', [0n]),
  triggerOk: capture(assert, 'trigger_ok', []),
  triggerFail: capture(assert, 'trigger_fail', []),
}, null, 2) + '\n');
