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

// Compile vector_tuple_cell.compact with --skip-zk, link generated contract to
// this branch's runtime, then pass contract/index.js as the argument.
import { pathToFileURL } from 'node:url';
import * as runtime from '../../../runtime/dist/index.js';

const [contractPath] = process.argv.slice(2);
if (!contractPath) throw new Error('expected contract/index.js');
const { Contract } = await import(pathToFileURL(contractPath).href);
const contract = new Contract({});
const coinPublicKey = { bytes: new Uint8Array(32) };
const initial = contract.initialState({
  initialPrivateState: null,
  initialZswapLocalState: runtime.emptyZswapLocalState(coinPublicKey),
});
let context = runtime.createCircuitContext(
  runtime.dummyContractAddress(), coinPublicKey,
  initial.currentContractState.data, initial.currentPrivateState,
);
function stateHex() {
  initial.currentContractState.data = new runtime.ChargedState(context.currentQueryContext.state.state);
  return Buffer.from(initial.currentContractState.serialize()).toString('hex');
}
const initialHex = stateHex();
const beforeVector = contract.circuits.read_values(context);
context = beforeVector.context;
const beforePair = contract.circuits.read_pair(context);
context = beforePair.context;
context = contract.circuits.set_values(context, [3n, 5n, 8n]).context;
const afterVectorHex = stateHex();
const afterVector = contract.circuits.read_values(context);
context = afterVector.context;
context = contract.circuits.set_pair(context, [42n, true]).context;
const afterPairHex = stateHex();
const afterPair = contract.circuits.read_pair(context);
process.stdout.write(JSON.stringify({
  initialHex, afterVectorHex, afterPairHex,
  beforeVector: beforeVector.result.map(String),
  beforePair: [String(beforePair.result[0]), beforePair.result[1]],
  afterVector: afterVector.result.map(String),
  afterPair: [String(afterPair.result[0]), afterPair.result[1]],
}, null, 2) + '\n');
