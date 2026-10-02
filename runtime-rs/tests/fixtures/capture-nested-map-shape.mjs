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

// Compile nested_map_shape.compact with the TypeScript target into a fresh
// directory, link its contract/node_modules to this branch's runtime, and pass
// contract/index.js. The source is also compiled by the Rust fixture gate.
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
const stateHex = () => Buffer.from(initial.currentContractState.serialize()).toString('hex');
const afterInit = stateHex();
const context = runtime.createCircuitContext(
  runtime.dummyContractAddress(), coinPublicKey,
  initial.currentContractState.data, initial.currentPrivateState,
);
const result = contract.circuits.check_nested_empty(context);
initial.currentContractState.data = new runtime.ChargedState(
  result.context.currentQueryContext.state.state,
);
const afterCall = stateHex();
const gas = Object.fromEntries(
  Object.entries(result.gasCost).map(([key, value]) => [key, value.toString()]),
);
process.stdout.write(JSON.stringify({
  afterInit,
  afterCall,
  result: result.result,
  gas,
  privateOutputCount: result.proofData.privateTranscriptOutputs.length,
}, null, 2) + '\n');
