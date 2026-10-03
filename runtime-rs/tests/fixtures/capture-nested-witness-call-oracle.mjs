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

// Compile nested_witness_call_oracle.compact with --skip-zk into a fresh target,
// link the generated contract to this branch's runtime, then pass contract/index.js.
import { pathToFileURL } from 'node:url';
import * as runtime from '../../../runtime/dist/index.js';

const [contractPath] = process.argv.slice(2);
if (!contractPath) throw new Error('expected contract/index.js');
const { Contract } = await import(pathToFileURL(contractPath).href);
const queryCosts = [];
const originalQuery = runtime.QueryContext.prototype.query;
runtime.QueryContext.prototype.query = function (...args) {
  const result = originalQuery.call(this, ...args);
  queryCosts.push({
    gasCost: result.gasCost,
    opTags: args[0].map((op) => Object.keys(op)[0]),
  });
  return result;
};
const contract = new Contract({
  secret: ({ privateState }) => [privateState + 1, 7n],
  ordered: ({ privateState }) => [privateState + 1, BigInt(privateState)],
});
const coinPublicKey = { bytes: new Uint8Array(32) };
const initial = contract.initialState({
  initialPrivateState: 7,
  initialZswapLocalState: runtime.emptyZswapLocalState(coinPublicKey),
});
const initialHex = Buffer.from(initial.currentContractState.serialize()).toString('hex');
const context = runtime.createCircuitContext(
  runtime.dummyContractAddress(), coinPublicKey,
  initial.currentContractState.data, initial.currentPrivateState,
);
const outer = contract.circuits.outer(context);
initial.currentContractState.data = new runtime.ChargedState(
  outer.context.currentQueryContext.state.state,
);
const afterOuterHex = Buffer.from(initial.currentContractState.serialize()).toString('hex');
const outerValue = contract.circuits.outerValue(outer.context);
initial.currentContractState.data = new runtime.ChargedState(
  outerValue.context.currentQueryContext.state.state,
);
const afterOuterValueHex = Buffer.from(initial.currentContractState.serialize()).toString('hex');
const outerValue2QueryStart = queryCosts.length;
const outerValue2 = contract.circuits.outerValue2(outerValue.context);
initial.currentContractState.data = new runtime.ChargedState(
  outerValue2.context.currentQueryContext.state.state,
);
const afterOuterValue2Hex = Buffer.from(initial.currentContractState.serialize()).toString('hex');
const outerValue2Queries = queryCosts.slice(outerValue2QueryStart);
const outerValueExprQueryStart = queryCosts.length;
const outerValueExpr = contract.circuits.outerValueExpr(outerValue2.context);
initial.currentContractState.data = new runtime.ChargedState(
  outerValueExpr.context.currentQueryContext.state.state,
);
function normalize(value) {
  if (typeof value === 'bigint') return value.toString();
  if (value instanceof Uint8Array) return { bytesHex: Buffer.from(value).toString('hex') };
  if (Array.isArray(value)) return value.map(normalize);
  if (value && typeof value === 'object') {
    return Object.fromEntries(Object.entries(value).map(([key, inner]) => [key, normalize(inner)]));
  }
  return value;
}
process.stdout.write(JSON.stringify({
  initialHex,
  afterOuterHex,
  afterOuterValueHex,
  afterOuterValue2Hex,
  afterOuterValueExprHex: Buffer.from(initial.currentContractState.serialize()).toString('hex'),
  privateState: outer.context.currentPrivateState,
  afterOuterValuePrivateState: outerValue.context.currentPrivateState,
  afterOuterValue2PrivateState: outerValue2.context.currentPrivateState,
  afterOuterValueExprPrivateState: outerValueExpr.context.currentPrivateState,
  privateTranscriptOutputs: outer.proofData.privateTranscriptOutputs.map(
    ({ value, alignment }) => ({
      valueAtoms: value.map((atom) => Array.from(atom)),
      alignment,
    }),
  ),
  outerValueTranscript: outerValue.proofData.privateTranscriptOutputs.map(
    ({ value, alignment }) => ({
      valueAtoms: value.map((atom) => Array.from(atom)),
      alignment,
    }),
  ),
  outerValue2Transcript: outerValue2.proofData.privateTranscriptOutputs.map(
    ({ value, alignment }) => ({
      valueAtoms: value.map((atom) => Array.from(atom)),
      alignment,
    }),
  ),
  outerValueExprTranscript: outerValueExpr.proofData.privateTranscriptOutputs.map(
    ({ value, alignment }) => ({
      valueAtoms: value.map((atom) => Array.from(atom)),
      alignment,
    }),
  ),
  outerValue2Gas: normalize(outerValue2.gasCost),
  outerValue2Queries: normalize(outerValue2Queries),
  outerValue2PublicTranscript: normalize(outerValue2.proofData.publicTranscript),
  outerValueExprGas: normalize(outerValueExpr.gasCost),
  outerValueExprQueries: normalize(queryCosts.slice(outerValueExprQueryStart)),
  outerValueExprPublicTranscript: normalize(outerValueExpr.proofData.publicTranscript),
}, null, 2) + '\n');
