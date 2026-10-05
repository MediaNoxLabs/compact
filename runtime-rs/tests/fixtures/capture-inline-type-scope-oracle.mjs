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

// Compile inline_type_scope_oracle.compact with --skip-zk, link generated
// contract to this branch's runtime, then pass contract/index.js as argument.
import { pathToFileURL } from 'node:url';
import { createRequire } from 'node:module';
import { normalizeQueryProgram } from '../../../tools/compact-rust-backend/oracles/normalize_query_program.mjs';

const [contractPath] = process.argv.slice(2);
if (!contractPath) throw new Error('expected contract/index.js');
const runtime = await import(pathToFileURL(createRequire(contractPath).resolve('@midnight-ntwrk/compact-runtime')));
const { Contract } = await import(pathToFileURL(contractPath).href);
const contract = new Contract({});
const coinPublicKey = { bytes: new Uint8Array(32) };
const initialState = () => contract.initialState({
  initialPrivateState: null,
  initialZswapLocalState: runtime.emptyZswapLocalState(coinPublicKey),
});
const initial = initialState();
const initialHex = Buffer.from(initial.currentContractState.serialize()).toString('hex');
const fieldVector = (length) => new runtime.CompactTypeVector(length, runtime.CompactTypeField);

const queries = [];
const rawQueries = [];
const nativeQueries = {};
const originalQuery = runtime.QueryContext.prototype.query;
runtime.QueryContext.prototype.query = function (...args) {
  const output = originalQuery.call(this, ...args);
  queries.push({ gasCost: output.gasCost, program: normalizeQueryProgram(args[0]) });
  rawQueries.push(args[0]);
  return output;
};
function scenario(name, hash, invoke) {
  const state = initialState();
  let context = runtime.createCircuitContext(
    runtime.dummyContractAddress(), coinPublicKey,
    state.currentContractState.data, state.currentPrivateState,
  );
  context = contract.circuits.setHash(context, hash).context;
  const preCheckState = new runtime.ChargedState(context.currentQueryContext.state.state);
  const start = queries.length;
  const result = invoke(context);
  nativeQueries[name] = { result: result.result, gasCost: result.gasCost,
    privateOutputs: result.proofData.privateTranscriptOutputs.length, queries: queries.slice(start) };
  const replayContext = runtime.createCircuitContext(
    runtime.dummyContractAddress(), coinPublicKey, preCheckState, state.currentPrivateState,
  );
  const replayProgram = rawQueries.slice(start).flat();
  const replay = replayContext.currentQueryContext.query(replayProgram, replayContext.costModel);
  nativeQueries[name].replayGas = replay.gasCost;
  context = result.context;
  state.currentContractState.data = new runtime.ChargedState(
    context.currentQueryContext.state.state,
  );
  return Buffer.from(state.currentContractState.serialize()).toString('hex');
}

const scalarHash = runtime.persistentHash(runtime.CompactTypeField, 5n);
const scalarStateHex = scenario('scalar', scalarHash, (context) =>
  contract.circuits.checkScalarScope(context, [9n, 10n], 5n));
const aggStateHex = scenario('aggregate',
  runtime.persistentHash(fieldVector(4), [1n, 2n, 3n, 4n]),
  (context) => contract.circuits.checkAggScope(context, [7n, 8n], [1n, 2n, 3n, 4n]),
);
const noCollisionStateHex = scenario('noCollision',
  runtime.persistentHash(fieldVector(2), [3n, 4n]),
  (context) => contract.circuits.checkNoCollisionScope(context, [3n, 4n]),
);
let scalarMismatch;
try {
  scenario('scalar', scalarHash, (context) => contract.circuits.checkScalarScope(context, [9n, 10n], 6n));
  throw new Error('scalar mismatch unexpectedly passed');
} catch (error) {
  scalarMismatch = String(error.message);
}
process.stdout.write(JSON.stringify({
  initialHex,
  scalarStateHex,
  aggStateHex,
  noCollisionStateHex,
  scalarMismatch, nativeQueries,
}, (_, value) => typeof value === 'bigint' ? value.toString() : value, 2) + '\n');
