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

// Compile a source fixture with compactc --skip-zk, link its contract
// node_modules/@midnight-ntwrk/compact-runtime to this branch's runtime,
// then run: node capture-collection-transition.mjs <contract/index.js> <set|map|list> [sequence].
import { pathToFileURL } from 'node:url';
import * as runtime from '../../../runtime/dist/index.js';

const [contractPath, kind, mode] = process.argv.slice(2);
if (!contractPath || !['set', 'map', 'list'].includes(kind)) {
  throw new Error('expected contract/index.js and set, map, or list');
}
if (mode && mode !== 'sequence') {
  throw new Error('optional third argument must be sequence');
}

const { Contract } = await import(pathToFileURL(contractPath).href);
const contract = new Contract({});
const coinPublicKey = { bytes: new Uint8Array(32) };
const initialZswapLocalState = runtime.emptyZswapLocalState(coinPublicKey);
const initial = contract.initialState({ initialPrivateState: null, initialZswapLocalState });
let context = runtime.createCircuitContext(
  runtime.dummyContractAddress(),
  coinPublicKey,
  initial.currentContractState.data,
  initial.currentPrivateState,
);
const operations = {
  set: [
    ['add', () => contract.circuits.add(context, true)],
    ['remove', () => contract.circuits.remove(context, true)],
    ['add_field', () => contract.circuits.add_field(context, 42n)],
    ['reset_fields', () => contract.circuits.reset_fields(context)],
  ],
  map: [
    ['put', () => contract.circuits.put(context, true, 42n)],
    ['remove_key', () => contract.circuits.remove_key(context, true)],
    ['put_default', () => contract.circuits.put_default(context, true)],
    ['reset_table', () => contract.circuits.reset_table(context)],
  ],
  list: [
    ['prepend', () => contract.circuits.prepend(context, 42n)],
    ['prepend', () => contract.circuits.prepend(context, 7n)],
    ['drop_first', () => contract.circuits.drop_first(context)],
    ['clear_items', () => contract.circuits.clear_items(context)],
  ],
}[kind];

function execute([operation, call]) {
  const result = call();
  context = result.context;
  initial.currentContractState.data = new runtime.ChargedState(
    context.currentQueryContext.state.state,
  );
  return {
    operation,
    stateHex: Buffer.from(initial.currentContractState.serialize()).toString('hex'),
  };
}

const output = mode === 'sequence'
  ? { steps: operations.map(execute) }
  : { stateHex: execute(operations[0]).stateHex };
process.stdout.write(JSON.stringify(output, null, 2) + '\n');
