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

// Compile uint_compare.compact with --skip-zk, link generated contract to
// this branch's runtime, then pass contract/index.js as the argument.
import { pathToFileURL } from 'node:url';
import * as runtime from '../../../runtime/dist/index.js';

const [contractPath] = process.argv.slice(2);
if (!contractPath) throw new Error('expected contract/index.js');
const { Contract, pureCircuits } = await import(pathToFileURL(contractPath).href);
const contract = new Contract({
  echo: ({ privateState }, value) => [privateState + 1, value + BigInt(privateState)],
});
const coinPublicKey = { bytes: new Uint8Array(32) };
function initialContext() {
  const initial = contract.initialState({
    initialPrivateState: 7,
    initialZswapLocalState: runtime.emptyZswapLocalState(coinPublicKey),
  });
  return runtime.createCircuitContext(
    runtime.dummyContractAddress(), coinPublicKey,
    initial.currentContractState.data, initial.currentPrivateState,
  );
}
function witnessed(left, right) {
  const result = contract.circuits.witnessed_less(initialContext(), left, right);
  return {
    result: result.result,
    privateState: result.context.currentPrivateState,
    privateTranscriptOutputs: result.proofData.privateTranscriptOutputs.map(
      ({ value, alignment }) => ({ valueAtoms: value.map((atom) => Array.from(atom)), alignment }),
    ),
  };
}
process.stdout.write(JSON.stringify({
  less: pureCircuits.less(7n, 9n),
  lessEqualSame: pureCircuits.less_equal(7n, 7n),
  lessEqualDifferent: pureCircuits.less_equal(9n, 7n),
  greater: pureCircuits.greater(9n, 7n),
  greaterEqualSame: pureCircuits.greater_equal(7n, 7n),
  greaterEqualDifferent: pureCircuits.greater_equal(7n, 9n),
  mixedWidth: pureCircuits.mixed_width(8n, 300n),
  witnessTrue: witnessed(2n, 3n),
  witnessFalse: witnessed(3n, 2n),
}, null, 2) + '\n');
