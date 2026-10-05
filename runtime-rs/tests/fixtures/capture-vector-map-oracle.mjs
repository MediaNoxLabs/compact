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

// Compile a map oracle with --skip-zk into a fresh directory, link the
// generated contract to this branch's compact runtime, then pass its index.js
// path and ledger field name.
import { pathToFileURL } from 'node:url';
import { createRequire } from 'node:module';
import { normalizeQueryProgram } from '../../../tools/compact-rust-backend/oracles/normalize_query_program.mjs';

const [contractPath, field] = process.argv.slice(2);
if (!contractPath || !field) throw new Error('expected contract/index.js and field');
const runtime = await import(pathToFileURL(createRequire(contractPath).resolve('@midnight-ntwrk/compact-runtime')));
const { Contract, ledger } = await import(pathToFileURL(contractPath).href);
const contract = new Contract({});
const coinPublicKey = { bytes: new Uint8Array(32) };
const initial = contract.initialState({
  initialPrivateState: null,
  initialZswapLocalState: runtime.emptyZswapLocalState(coinPublicKey),
});
const stateHex = () => Buffer.from(initial.currentContractState.serialize()).toString('hex');
const values = () => Array.from(ledger(initial.currentContractState.data)[field], String);
const afterInit = { stateHex: stateHex(), values: values() };
const context = runtime.createCircuitContext(
  runtime.dummyContractAddress(), coinPublicKey,
  initial.currentContractState.data, initial.currentPrivateState,
);
const queries = [];
const originalQuery = runtime.QueryContext.prototype.query;
runtime.QueryContext.prototype.query = function (...args) {
  const output = originalQuery.call(this, ...args);
  queries.push({ gasCost: output.gasCost, program: normalizeQueryProgram(args[0]) });
  return output;
};
const result = contract.circuits.ping(context);
const ping = { result: result.result, privateOutputs: result.proofData.privateTranscriptOutputs.length,
  gasCost: result.gasCost, queries: queries.slice() };

initial.currentContractState.data = new runtime.ChargedState(
  result.context.currentQueryContext.state.state,
);
const afterPing = { stateHex: stateHex(), values: values() };
process.stdout.write(JSON.stringify({ afterInit, afterPing, ping }, (_, value) => typeof value === 'bigint' ? value.toString() : value, 2) + '\n');
