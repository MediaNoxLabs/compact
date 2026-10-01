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

// Compile nested_map_oracle.compact with --skip-zk into a fresh target and
// link the generated contract to this branch's compact runtime.
import { pathToFileURL } from 'node:url';
import * as runtime from '../../../runtime/dist/index.js';

const [contractPath] = process.argv.slice(2);
if (!contractPath) throw new Error('expected contract/index.js');
const { Contract, ledger } = await import(pathToFileURL(contractPath).href);
const contract = new Contract({});
const coinPublicKey = { bytes: new Uint8Array(32) };
const initial = contract.initialState({
  initialPrivateState: null,
  initialZswapLocalState: runtime.emptyZswapLocalState(coinPublicKey),
});
const stateHex = () => Buffer.from(initial.currentContractState.serialize()).toString('hex');
const afterInit = {
  stateHex: stateHex(),
  flag: ledger(initial.currentContractState.data).flag,
};
const context = runtime.createCircuitContext(
  runtime.dummyContractAddress(), coinPublicKey,
  initial.currentContractState.data, initial.currentPrivateState,
);
const result = contract.circuits.ping(context);
initial.currentContractState.data = new runtime.ChargedState(
  result.context.currentQueryContext.state.state,
);
const afterPing = {
  stateHex: stateHex(),
  flag: ledger(initial.currentContractState.data).flag,
};
process.stdout.write(JSON.stringify({ afterInit, afterPing }, null, 2) + '\n');
