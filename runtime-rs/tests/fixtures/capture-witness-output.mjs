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

// Compile examples/rust_backend/witness_minimal.compact with compactc --skip-zk,
// link its contract's @midnight-ntwrk/compact-runtime to this branch's runtime,
// then run: node capture-witness-output.mjs <contract/index.js> [minimal|argument|expression|expression_pair].
import { pathToFileURL } from 'node:url';
import * as runtime from '../../../runtime/dist/index.js';

const [contractPath, kind = 'minimal'] = process.argv.slice(2);
if (!contractPath) {
  throw new Error('expected contract/index.js');
}
if (!['minimal', 'argument', 'expression', 'expression_pair'].includes(kind)) {
  throw new Error('expected minimal, argument, expression, or expression_pair');
}

const { Contract } = await import(pathToFileURL(contractPath).href);
const witnesses = kind === 'minimal'
  ? { private_value: ({ privateState }) => [privateState + 1, 42n] }
  : kind === 'argument' ? { private_offset: ({ privateState }, value) => {
    if (value !== 2n) throw new Error('unexpected witness argument');
    return [privateState + 1, value + 40n];
  } } : { secret: ({ privateState }, seed) => {
    if (seed !== 2n) throw new Error('unexpected witness argument');
    return [privateState + 1, seed + BigInt(privateState)];
  } };
const contract = new Contract(witnesses);
const coinPublicKey = { bytes: new Uint8Array(32) };
const initial = contract.initialState({
  initialPrivateState: 7,
  initialZswapLocalState: runtime.emptyZswapLocalState(coinPublicKey),
});
const context = runtime.createCircuitContext(
  runtime.dummyContractAddress(),
  coinPublicKey,
  initial.currentContractState.data,
  initial.currentPrivateState,
);
const result = kind === 'minimal'
  ? contract.circuits.read_private(context)
  : kind === 'argument'
    ? contract.circuits.apply_offset(context, 2n)
    : kind === 'expression'
      ? contract.circuits.add_secret(context, 2n)
      : contract.circuits.sum_secrets(context, 2n);
process.stdout.write(JSON.stringify({
  result: result.result.toString(),
  privateState: result.context.currentPrivateState,
  privateTranscriptOutputs: result.proofData.privateTranscriptOutputs.map(
    ({ value, alignment }) => ({
      valueAtoms: value.map((atom) => Array.from(atom)),
      alignment,
    }),
  ),
}, null, 2) + '\n');
