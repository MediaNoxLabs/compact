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

// Compile boolean_logic.compact with --skip-zk, link generated contract to
// this branch's runtime, then pass contract/index.js as the argument.
import { pathToFileURL } from 'node:url';
import * as runtime from '../../../runtime/dist/index.js';

const [contractPath] = process.argv.slice(2);
if (!contractPath) throw new Error('expected contract/index.js');
const { Contract, pureCircuits } = await import(pathToFileURL(contractPath).href);
const contract = new Contract({ echo: ({ privateState }, value) => [privateState + 1, value] });
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
function witnessed(name, ...args) {
  const result = contract.circuits[name](initialContext(), ...args);
  return {
    result: result.result,
    privateState: result.context.currentPrivateState,
    privateTranscriptOutputs: result.proofData.privateTranscriptOutputs.map(
      ({ value, alignment }) => ({ valueAtoms: value.map((atom) => Array.from(atom)), alignment }),
    ),
  };
}
process.stdout.write(JSON.stringify({
  pure: {
    invertTrue: pureCircuits.invert(true),
    invertFalse: pureCircuits.invert(false),
    bothTrue: pureCircuits.both(true, true),
    bothFalse: pureCircuits.both(false, true),
    eitherTrue: pureCircuits.either(true, false),
    eitherFalse: pureCircuits.either(false, false),
  },
  andSkip: witnessed('witnessed_both', false, true),
  andBoth: witnessed('witnessed_both', true, false),
  orSkip: witnessed('witnessed_either', true, false),
  orBoth: witnessed('witnessed_either', false, true),
  notTrue: witnessed('witnessed_not', true),
}, null, 2) + '\n');
