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

// Compile mixed_width_operand_oracle.compact with --skip-zk, link generated
// contract to this branch's runtime, then pass contract/index.js as argument.
import { pathToFileURL } from 'node:url';
import * as runtime from '../../../runtime/dist/index.js';

const [contractPath] = process.argv.slice(2);
if (!contractPath) throw new Error('expected contract/index.js');
const { Contract, pureCircuits } = await import(pathToFileURL(contractPath).href);
const queries = [];
const originalQuery = runtime.QueryContext.prototype.query;
runtime.QueryContext.prototype.query = function (...args) {
  const result = originalQuery.call(this, ...args);
  queries.push({ gasCost: normalize(result.gasCost) });
  return result;
};
function normalize(value) {
  if (typeof value === 'bigint') return value.toString();
  if (value instanceof Uint8Array) return { bytesHex: Buffer.from(value).toString('hex') };
  if (Array.isArray(value)) return value.map(normalize);
  if (value && typeof value === 'object') {
    return Object.fromEntries(Object.entries(value).map(([key, inner]) => [key, normalize(inner)]));
  }
  return value;
}
const coinPublicKey = { bytes: new Uint8Array(32) };
const contract = new Contract({});
const initial = contract.initialState({
  initialPrivateState: null,
  initialZswapLocalState: runtime.emptyZswapLocalState(coinPublicKey),
}, 20n, 4n);
const initialHex = Buffer.from(initial.currentContractState.serialize()).toString('hex');
const check = (run) => {
  try {
    return { ok: true, value: run()?.toString() ?? null };
  } catch (error) {
    return { ok: false, message: String(error.message) };
  }
};
const comparisons = Object.fromEntries(
  ['LE', 'LT', 'GT', 'GE', 'EQ', 'NE'].map((operator) => [
    operator,
    [
      check(() => pureCircuits[`assertProduct${operator}`](1073741823n, 4294967292n)),
      check(() => pureCircuits[`assertProduct${operator}`](1073741824n, 4294967295n)),
    ],
  ]),
);
let context = runtime.createCircuitContext(
  runtime.dummyContractAddress(), coinPublicKey,
  initial.currentContractState.data, initial.currentPrivateState,
);
context = contract.circuits.recordPinned(context, 4n, 20n).context;
context = contract.circuits.recordMatching(context, 7n, 7n).context;
initial.currentContractState.data = new runtime.ChargedState(
  context.currentQueryContext.state.state,
);
function captureCall(name, args) {
  const fresh = contract.initialState({
    initialPrivateState: null,
    initialZswapLocalState: runtime.emptyZswapLocalState(coinPublicKey),
  }, 20n, 4n);
  const freshContext = runtime.createCircuitContext(
    runtime.dummyContractAddress(), coinPublicKey,
    fresh.currentContractState.data, fresh.currentPrivateState,
  );
  const initialStateHex = Buffer.from(fresh.currentContractState.serialize()).toString('hex');
  const queryStart = queries.length;
  try {
    const output = contract.circuits[name](freshContext, ...args);
    fresh.currentContractState.data = new runtime.ChargedState(
      output.context.currentQueryContext.state.state,
    );
    return {
      ok: true,
      result: normalize(output.result),
      initialStateHex,
      afterStateHex: Buffer.from(fresh.currentContractState.serialize()).toString('hex'),
      queries: queries.slice(queryStart),
      reportedGas: normalize(output.gasCost),
      publicTranscript: normalize(output.proofData.publicTranscript),
      privateTranscriptOutputs: normalize(output.proofData.privateTranscriptOutputs),
    };
  } catch (error) {
    return {
      ok: false,
      message: String(error.message),
      initialStateHex,
      afterStateHex: Buffer.from(fresh.currentContractState.serialize()).toString('hex'),
      queryCount: queries.length - queryStart,
    };
  }
}
const recordedEvidence = {
  recordPinnedSuccess: captureCall('recordPinned', [4n, 20n]),
  recordMatchingSuccess: captureCall('recordMatching', [7n, 7n]),
  recordPinnedFailure: captureCall('recordPinned', [1073741824n, 4294967295n]),
  recordMatchingFailure: captureCall('recordMatching', [7n, 8n]),
};
process.stdout.write(JSON.stringify({
  initialHex,
  stateAfterActionsHex: Buffer.from(initial.currentContractState.serialize()).toString('hex'),
  constructorUnderflow: check(() => contract.initialState({
    initialPrivateState: null,
    initialZswapLocalState: runtime.emptyZswapLocalState(coinPublicKey),
  }, 1n, 1n)),
  comparisons,
  sumMixed: check(() => pureCircuits.sumMixed(4294967295n, 255n)),
  productMixed: check(() => pureCircuits.productMixed(4294967295n, 255n)),
  guardedDiff: [
    check(() => pureCircuits.guardedDiff(20n, 4n)),
    check(() => pureCircuits.guardedDiff(1n, 1n)),
  ],
  recordPinnedFailure: check(() => contract.circuits.recordPinned(
    context, 1073741824n, 4294967295n,
  )),
  recordMatchingFailure: check(() => contract.circuits.recordMatching(context, 7n, 8n)),
  recordedEvidence,
}, null, 2) + '\n');
