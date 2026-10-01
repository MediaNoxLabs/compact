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

// Compile keccak.compact with --skip-zk, link its generated contract to this
// branch's runtime, then pass contract/index.js as the argument.
import { pathToFileURL } from 'node:url';
import * as runtime from '../../../runtime/dist/index.js';

const [contractPath] = process.argv.slice(2);
if (!contractPath) throw new Error('expected contract/index.js');
const { Contract, pureCircuits } = await import(pathToFileURL(contractPath).href);
const field = 42n;
const bytes = Uint8Array.of(1, 2, 3, 4);
const pair = [3n, 5n];
const pairType = new runtime.CompactTypeVector(2, runtime.CompactTypeField);
const contract = new Contract({
  echo: ({ privateState }, value) => [privateState + 1, value + BigInt(privateState)],
});
const coinPublicKey = { bytes: new Uint8Array(32) };

function byteHex(value) { return Buffer.from(value).toString('hex'); }
function check(name, compiled, native) {
  const actual = byteHex(compiled);
  if (actual !== byteHex(native)) throw new Error(`${name}: generated TypeScript differs from native`);
  return actual;
}
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
const witnessed = contract.circuits.hash_echo(initialContext(), 2n);
const witnessNative = runtime.keccak256(runtime.CompactTypeField, 9n);
process.stdout.write(JSON.stringify({
  field: check('field', pureCircuits.hash_field(field), runtime.keccak256(runtime.CompactTypeField, field)),
  bytes: check('bytes', pureCircuits.hash_bytes(bytes), runtime.keccak256(new runtime.CompactTypeBytes(4), bytes)),
  pair: check('pair', pureCircuits.hash_pair(pair), runtime.keccak256(pairType, pair)),
  witness: {
    result: check('witness', witnessed.result, witnessNative),
    privateState: witnessed.context.currentPrivateState,
    privateTranscriptOutputs: witnessed.proofData.privateTranscriptOutputs.map(
      ({ value, alignment }) => ({ valueAtoms: value.map((atom) => Array.from(atom)), alignment }),
    ),
  },
}, null, 2) + '\n');
