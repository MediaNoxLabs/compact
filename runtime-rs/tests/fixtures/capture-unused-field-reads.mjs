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


// Compile the original PM-19252 source with --target ts --skip-zk and link
// its runtime dependency. Arguments: generated index.js, seven|eight-a|eight-b.
import { createRequire } from 'node:module';
import { pathToFileURL } from 'node:url';
import { normalizeQueryProgram } from '../../../tools/compact-rust-backend/oracles/normalize_query_program.mjs';
const [contractPath, kind] = process.argv.slice(2);
if (!contractPath || !['seven', 'eight-a', 'eight-b'].includes(kind)) throw new Error('expected contract and case');
const runtime = await import(pathToFileURL(createRequire(contractPath).resolve('@midnight-ntwrk/compact-runtime')));
const { Contract } = await import(pathToFileURL(contractPath));
const contract = new Contract({});
const key = { bytes: new Uint8Array(32) };
const queries = [];
const originalQuery = runtime.QueryContext.prototype.query;
runtime.QueryContext.prototype.query = function (...args) {
  const result = originalQuery.call(this, ...args);
  queries.push({ program: normalizeQueryProgram(args[0]), gasCost: result.gasCost });
  return result;
};
const cases = [];
for (const seed of (kind === 'seven' ? [0n] : [0n, 21n])) {
  const initial = contract.initialState({initialPrivateState: null, initialZswapLocalState: runtime.emptyZswapLocalState(key)});
  let context = runtime.createCircuitContext(runtime.dummyContractAddress(), key, initial.currentContractState.data, null);
  if (seed !== 0n) context = contract.circuits[kind === 'eight-a' ? 'test' : 'test1'](context, seed).context;
  initial.currentContractState.data = new runtime.ChargedState(context.currentQueryContext.state.state);
  const beforeStateHex = Buffer.from(initial.currentContractState.serialize()).toString('hex');
  const start = queries.length;
  const output = kind === 'seven' ? contract.circuits.test(context, 9n) : contract.circuits[kind === 'eight-a' ? 'test1' : 'test'](context);
  initial.currentContractState.data = new runtime.ChargedState(output.context.currentQueryContext.state.state);
  cases.push({seed, beforeStateHex, afterStateHex: Buffer.from(initial.currentContractState.serialize()).toString('hex'),
    result: output.result, gasCost: output.gasCost, queries: queries.slice(start),
    publicProgram: normalizeQueryProgram(output.proofData.publicTranscript),
    privateTranscriptCount: output.proofData.privateTranscriptOutputs.length});
}
process.stdout.write(JSON.stringify({kind,cases}, (_,value) => typeof value === 'bigint' ? value.toString() : value, 2)+'\n');
