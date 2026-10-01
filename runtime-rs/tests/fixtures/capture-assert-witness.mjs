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

// Compile assert_witness.compact with --skip-zk, link generated contract to
// this branch's runtime, then pass contract/index.js as the argument.
import { pathToFileURL } from 'node:url';
import * as runtime from '../../../runtime/dist/index.js';

const [contractPath] = process.argv.slice(2);
if (!contractPath) throw new Error('expected contract/index.js');
const { Contract } = await import(pathToFileURL(contractPath).href);
let observed = [];
const contract = new Contract({
  echo: ({ privateState }, value) => {
    observed.push({ privateState, value });
    return [privateState + 1, value];
  },
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
function run(first, second) {
  observed = [];
  try {
    const result = contract.circuits.checked_value(initialContext(), first, second, 42n);
    return {
      result: result.result.toString(),
      privateState: result.context.currentPrivateState,
      privateTranscriptOutputs: result.proofData.privateTranscriptOutputs.map(
        ({ value, alignment }) => ({ valueAtoms: value.map((atom) => Array.from(atom)), alignment }),
      ),
      observed,
    };
  } catch (error) {
    return { error: error.message, observed };
  }
}
function write(flag) {
  observed = [];
  try {
    const written = contract.circuits.checked_write(initialContext(), flag, 42n);
    const read = contract.circuits.read_cell(written.context);
    return { value: read.result.toString(), privateState: written.context.currentPrivateState, observed };
  } catch (error) {
    return { error: error.message, observed };
  }
}
process.stdout.write(JSON.stringify({
  pass: run(true, true),
  firstFails: run(false, false),
  secondFails: run(true, false),
  writePass: write(true),
  writeFails: write(false),
}, null, 2) + '\n');
