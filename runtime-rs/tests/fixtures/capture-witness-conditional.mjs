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

// Compile witness_conditional with --skip-zk, link the contract runtime to
// this branch's runtime, then pass contract/index.js as the only argument.
import { pathToFileURL } from 'node:url';
import * as runtime from '../../../runtime/dist/index.js';

const [contractPath] = process.argv.slice(2);
if (!contractPath) throw new Error('expected contract/index.js');
const { Contract } = await import(pathToFileURL(contractPath).href);
const contract = new Contract({
  secret: ({ privateState }, seed) => [
    privateState + 1,
    seed + BigInt(privateState),
  ],
});
const coinPublicKey = { bytes: new Uint8Array(32) };

function initialContext() {
  const initial = contract.initialState({
    initialPrivateState: 7,
    initialZswapLocalState: runtime.emptyZswapLocalState(coinPublicKey),
  });
  return runtime.createCircuitContext(
    runtime.dummyContractAddress(),
    coinPublicKey,
    initial.currentContractState.data,
    initial.currentPrivateState,
  );
}

function output(result) {
  return {
    result: Array.isArray(result.result)
      ? result.result.map((value) => value.toString())
      : result.result.toString(),
    privateState: result.context.currentPrivateState,
    privateTranscriptOutputs: result.proofData.privateTranscriptOutputs.map(
      ({ value, alignment }) => ({
        valueAtoms: value.map((atom) => Array.from(atom)),
        alignment,
      }),
    ),
  };
}

process.stdout.write(JSON.stringify({
  selected: output(contract.circuits.choose_secret(initialContext(), true, 2n, 20n)),
  unselected: output(contract.circuits.choose_secret(initialContext(), false, 2n, 20n)),
  pair: output(contract.circuits.pair_secret(initialContext(), 2n)),
  nested: output(contract.circuits.nested_secret(initialContext(), 2n)),
  local: output(contract.circuits.local_secret(initialContext(), 2n)),
}, null, 2) + '\n');
