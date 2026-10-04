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

// Compile ternary_cond_oracle.compact with --target ts --skip-zk and link
// contract/node_modules/@midnight-ntwrk/compact-runtime to this runtime.
// Then pass contract/index.js as the argument. The flag seed uses the same
// typed root Cell write VM program that the generated constructor emits.
import { pathToFileURL } from 'node:url';
import * as runtime from '../../../runtime/dist/index.js';

const [contractPath] = process.argv.slice(2);
if (!contractPath) throw new Error('expected contract/index.js');
const { Contract } = await import(pathToFileURL(contractPath).href);
const contract = new Contract({ echoField: (ctx, value) => [ctx.privateState, value] });
const coinPublicKey = { bytes: new Uint8Array(32) };
const queries = [];
const originalQuery = runtime.QueryContext.prototype.query;
runtime.QueryContext.prototype.query = function (...args) {
  const result = originalQuery.call(this, ...args);
  queries.push({
    gasCost: Object.fromEntries(
      Object.entries(result.gasCost).map(([name, value]) => [name, value.toString()]),
    ),
    opTags: args[0].map((operation) =>
      typeof operation === 'string' ? operation : Object.keys(operation)[0]),
  });
  return result;
};

function shape(operation) {
  if (typeof operation === 'string') return { kind: operation };
  if (operation.idx) {
    const { cached, pushPath, path } = operation.idx;
    return { kind: 'idx', cached, pushPath, pathLength: path.length };
  }
  if (operation.push) return { kind: 'push', storage: operation.push.storage };
  if (operation.ins) return { kind: 'ins', cached: operation.ins.cached, n: operation.ins.n };
  if (operation.dup) return { kind: 'dup', n: operation.dup.n };
  if (operation.addi) return { kind: 'addi', immediate: operation.addi.immediate };
  if (operation.popeq) {
    return {
      kind: 'popeq', cached: operation.popeq.cached,
      resultAtoms: operation.popeq.result.value.map((atom) => Array.from(atom)),
    };
  }
  throw new Error(`unexpected operation: ${Object.keys(operation)}`);
}

function capture(name, args, flag) {
  const initial = contract.initialState({
    initialPrivateState: null,
    initialZswapLocalState: runtime.emptyZswapLocalState(coinPublicKey),
  }, true, true, 111n);
  const context = runtime.createCircuitContext(
    runtime.dummyContractAddress(), coinPublicKey,
    initial.currentContractState.data, initial.currentPrivateState,
  );
  if (flag) {
    const indexType = new runtime.CompactTypeUnsignedInteger(255n, 1);
    const boolType = runtime.CompactTypeBoolean;
    const write = [
      { push: { storage: false,
        value: runtime.StateValue.newCell({ value: indexType.toValue(0n), alignment: indexType.alignment() }).encode() } },
      { push: { storage: true,
        value: runtime.StateValue.newCell({ value: boolType.toValue(true), alignment: boolType.alignment() }).encode() } },
      { ins: { cached: false, n: 1 } },
    ];
    context.currentQueryContext = context.currentQueryContext.query(write, context.costModel).context;
  }
  const start = queries.length;
  const output = contract.circuits[name](context, ...args);
  initial.currentContractState.data = new runtime.ChargedState(
    output.context.currentQueryContext.state.state,
  );
  return {
    stateHex: Buffer.from(initial.currentContractState.serialize()).toString('hex'),
    publicTranscriptShape: output.proofData.publicTranscript.map(shape),
    privateTranscriptCount: output.proofData.privateTranscriptOutputs.length,
    privateStateNull: output.context.currentPrivateState === null,
    queries: queries.slice(start),
    reportedGas: Object.fromEntries(
      Object.entries(output.gasCost).map(([key, value]) => [key, value.toString()]),
    ),
  };
}

process.stdout.write(JSON.stringify({
  walkerWriteFalse: capture('walkerWrite', [false, 777n], false),
  walkerWriteTrue: capture('walkerWrite', [true, 777n], false),
  streamWriteFalse: capture('streamWrite', [false, 777n], false),
  streamWriteTrue: capture('streamWrite', [true, 777n], false),
  walkerInlineWriteFalse: capture('walkerInlineWrite', [false], false),
  walkerInlineWriteTrue: capture('walkerInlineWrite', [true], false),
  streamConstAnnotatedFalse: capture('streamConstAnnotated', [], false),
  streamConstAnnotatedTrue: capture('streamConstAnnotated', [], true),
  streamIncrementFalse: capture('streamIncrement', [], false),
  streamIncrementTrue: capture('streamIncrement', [], true),
}, null, 2) + '\n');
