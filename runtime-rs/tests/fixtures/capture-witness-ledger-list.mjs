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

// Compile witness_ledger_list with --skip-zk, link its contract runtime to this
// branch's runtime, then run: node capture-witness-ledger-list.mjs <contract/index.js>.
import { pathToFileURL } from 'node:url';
import * as runtime from '../../../runtime/dist/index.js';

const [contractPath] = process.argv.slice(2);
if (!contractPath) throw new Error('expected contract/index.js');
const { Contract } = await import(pathToFileURL(contractPath).href);
const witnesses = {
  first_is_42: ({ ledger, privateState }) => {
    const list = ledger.items;
    const head = list.head();
    const empty = list.isEmpty();
    if (empty !== (list.length() === 0n)) throw new Error('List length mismatch');
    if (empty !== !head.is_some) throw new Error('List head mismatch');
    return [privateState + 1, head.is_some && head.value === 42n];
  },
};
const contract = new Contract(witnesses);
const coinPublicKey = { bytes: new Uint8Array(32) };
const initial = contract.initialState({
  initialPrivateState: 7,
  initialZswapLocalState: runtime.emptyZswapLocalState(coinPublicKey),
});
let context = runtime.createCircuitContext(
  runtime.dummyContractAddress(),
  coinPublicKey,
  initial.currentContractState.data,
  initial.currentPrivateState,
);

function output(result) {
  return {
    result: result.result,
    privateState: result.context.currentPrivateState,
    privateTranscriptOutputs: result.proofData.privateTranscriptOutputs.map(
      ({ value, alignment }) => ({
        valueAtoms: value.map((atom) => Array.from(atom)),
        alignment,
      }),
    ),
  };
}

const before = contract.circuits.private_first_is_42(context);
context = contract.circuits.prepend(before.context, 42n).context;
const after = contract.circuits.private_first_is_42(context);
context = contract.circuits.prepend(after.context, 7n).context;
const covered = contract.circuits.private_first_is_42(context);
context = contract.circuits.drop_first(covered.context).context;
const restored = contract.circuits.private_first_is_42(context);
process.stdout.write(JSON.stringify({
  before: output(before), after: output(after),
  covered: output(covered), restored: output(restored),
}, null, 2) + '\n');
