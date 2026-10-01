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

// Compile chunked_ledger_oracle.compact with --skip-zk in a fresh directory,
// link its contract/node_modules/@midnight-ntwrk/compact-runtime to runtime,
// then pass contract/index.js as the first argument.
import { pathToFileURL } from 'node:url';
import * as runtime from '../../../runtime/dist/index.js';

const [contractPath] = process.argv.slice(2);
if (!contractPath) throw new Error('expected contract/index.js');
const { Contract, ledger, pureCircuits } = await import(pathToFileURL(contractPath).href);
const contract = new Contract({});
const coinPublicKey = { bytes: new Uint8Array(32) };
const initial = contract.initialState({
  initialPrivateState: null,
  initialZswapLocalState: runtime.emptyZswapLocalState(coinPublicKey),
});
const view = ledger(initial.currentContractState.data);
const values = Array.from({ length: 17 }, (_, i) =>
  String(view[`f${String(i).padStart(2, '0')}`]));
process.stdout.write(JSON.stringify({
  stateHex: Buffer.from(initial.currentContractState.serialize()).toString('hex'),
  values,
  active: view.active,
  ping: pureCircuits.ping(true),
}, null, 2) + '\n');
