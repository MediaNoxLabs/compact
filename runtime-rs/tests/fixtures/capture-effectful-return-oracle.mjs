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

// Compile effectful_return_oracle.compact independently for TS, link the
// contract's @midnight-ntwrk/compact-runtime import, then pass both paths.
import { pathToFileURL } from 'node:url';

const [contractPath, runtimePath] = process.argv.slice(2);
if (!contractPath || !runtimePath) throw new Error('expected contract and runtime paths');
const runtime = await import(pathToFileURL(runtimePath).href);
const { Contract } = await import(pathToFileURL(contractPath).href);
const contract = new Contract({
  mark: ({ privateState }) => [privateState + 1, 77n],
});
const coinPublicKey = { bytes: new Uint8Array(32) };
const stateHex = (state) => Buffer.from(state.serialize()).toString('hex');
const gas = (cost) => Object.fromEntries(
  Object.entries(cost).map(([key, value]) => [key, value.toString()]),
);
const queries = [];
const programs = [];
const originalQuery = runtime.QueryContext.prototype.query;
runtime.QueryContext.prototype.query = function (...args) {
  const result = originalQuery.call(this, ...args);
  programs.push(args[0]);
  queries.push({
    gasCost: gas(result.gasCost),
    opTags: args[0].map((operation) =>
      typeof operation === 'string' ? operation : Object.keys(operation)[0]),
  });
  return result;
};

const initial = contract.initialState({
  initialPrivateState: 0,
  initialZswapLocalState: runtime.emptyZswapLocalState(coinPublicKey),
});
const initialStateHex = stateHex(initial.currentContractState);
function call(next) {
  const queryStart = queries.length;
  const programStart = programs.length;
  const context = runtime.createCircuitContext(
    runtime.dummyContractAddress(), coinPublicKey,
    initial.currentContractState.data, initial.currentPrivateState,
  );
  const output = contract.circuits.choose(context, next);
  const replayContext = runtime.createCircuitContext(
    runtime.dummyContractAddress(), coinPublicKey,
    initial.currentContractState.data, initial.currentPrivateState,
  );
  const replay = originalQuery.call(replayContext.currentQueryContext,
    programs.slice(programStart).flat(), replayContext.costModel);
  initial.currentPrivateState = output.context.currentPrivateState;
  initial.currentContractState.data = new runtime.ChargedState(
    output.context.currentQueryContext.state.state,
  );
  return {
    next: next.toString(),
    result: output.result.toString(),
    privateState: output.context.currentPrivateState,
    afterStateHex: stateHex(initial.currentContractState),
    effects: output.context.currentQueryContext.effects,
    reportedGas: gas(output.gasCost),
    replayGas: gas(replay.gasCost),
    publicTranscript: output.proofData.publicTranscript,
    privateTranscriptOutputs: output.proofData.privateTranscriptOutputs,
    output: output.proofData.output,
    queries: queries.slice(queryStart),
    privateTranscriptCount: output.proofData.privateTranscriptOutputs.length,
  };
}
process.stdout.write(JSON.stringify({
  initialStateHex,
  chosen: call(9n),
  unchosen: call(8n),
}, (_, value) => value instanceof Uint8Array ? Array.from(value) : typeof value === 'bigint' ? value.toString() : value, 2) + '\n');
