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

// Compile opaque_string_map_query_oracle.compact with --skip-zk into a fresh
// target, link its package to this branch's runtime, then pass contract/index.js.
import { pathToFileURL } from 'node:url';
import * as runtime from '../../../runtime/dist/index.js';

const [contractPath] = process.argv.slice(2);
if (!contractPath) throw new Error('expected contract/index.js');
const { Contract } = await import(pathToFileURL(contractPath).href);
const contract = new Contract({});
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
let beforeError;
try { contract.circuits.ensure(context, 'asset-1'); }
catch (error) { beforeError = error.message; }
const put = contract.circuits.put(context, 'asset-1', 42n);
initial.currentContractState.data = new runtime.ChargedState(
  put.context.currentQueryContext.state.state,
);
const ensured = contract.circuits.ensure(put.context, 'asset-1');
process.stdout.write(JSON.stringify({
  initialHex,
  afterPutHex: Buffer.from(initial.currentContractState.serialize()).toString('hex'),
  beforeError,
  result: String(ensured.result),
}, null, 2) + '\n');
