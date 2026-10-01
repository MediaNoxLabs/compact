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

// Compile witness_uint_arithmetic with --skip-zk, link its generated contract
// to this branch's runtime, then pass contract/index.js as the argument.
import { pathToFileURL } from 'node:url';
import * as runtime from '../../../runtime/dist/index.js';

const [contractPath] = process.argv.slice(2);
if (!contractPath) throw new Error('expected contract/index.js');
const { Contract } = await import(pathToFileURL(contractPath).href);
const contract = new Contract({
  echo: ({ privateState }, value) => [privateState + 1, value],
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

function output(name, value) {
  const result = contract.circuits[name](initialContext(), value);
  return {
    result: result.result.toString(),
    privateState: result.context.currentPrivateState,
    privateTranscriptOutputs: result.proofData.privateTranscriptOutputs.map(
      ({ value, alignment }) => ({
        valueAtoms: value.map((atom) => Array.from(atom)),
        alignment,
      }),
    ),
  };
}

function rejected(name, value) {
  try {
    contract.circuits[name](initialContext(), value);
    return false;
  } catch {
    return true;
  }
}

process.stdout.write(JSON.stringify({
  add: output('add_echo', 2n),
  subtract: output('subtract_echo', 2n),
  multiply: output('multiply_echo', 2n),
  addOverflowRejected: rejected('add_echo', 65535n),
  subtractUnderflowRejected: rejected('subtract_echo', 0n),
  multiplyOverflowRejected: rejected('multiply_echo', 65535n),
}, null, 2) + '\n');
