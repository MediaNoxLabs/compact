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

// Compile schnorr_attest_oracle.compact with --skip-zk into a fresh target,
// link its package to this branch's runtime, then pass contract/index.js.
import { pathToFileURL } from 'node:url';
import * as runtime from '../../../runtime/dist/index.js';

const [contractPath] = process.argv.slice(2);
if (!contractPath) throw new Error('expected contract/index.js');
const { Contract, pureCircuits } = await import(pathToFileURL(contractPath).href);
const generator = runtime.ecMulGenerator(1n);
const contract = new Contract({
  getSchnorrReduction: ({ privateState }, challengeHash) => [
    privateState + 1,
    [challengeHash >> 248n, challengeHash & ((1n << 248n) - 1n)],
  ],
  localAttestorKey: ({ privateState }) => [privateState + 1, generator],
});
const coinPublicKey = { bytes: new Uint8Array(32) };
const initial = contract.initialState({
  initialPrivateState: 7,
  initialZswapLocalState: runtime.emptyZswapLocalState(coinPublicKey),
});
const digest = pureCircuits.attestationDigest(new Uint8Array(32), 1n, 2n);
const initialHex = Buffer.from(initial.currentContractState.serialize()).toString('hex');
const challengeHash = contract._transientHash_3({
  ann_x: generator.x,
  ann_y: generator.y,
  pk_x: generator.x,
  pk_y: generator.y,
  msg: digest,
});
const signature = {
  announcement: generator,
  response: 1n + (challengeHash & ((1n << 248n) - 1n)),
};
const context = runtime.createCircuitContext(
  runtime.dummyContractAddress(), coinPublicKey,
  initial.currentContractState.data, initial.currentPrivateState,
);
const verified = contract.circuits.verifyAttestation(context, digest, signature);
const accepted = contract.circuits.acceptAttestation(verified.context, digest, signature);
initial.currentContractState.data = new runtime.ChargedState(
  accepted.context.currentQueryContext.state.state,
);
const fieldHex = (n) => {
  const bytes = new Uint8Array(32);
  for (let i = 0; i < 32; i++) {
    bytes[i] = Number(n & 255n);
    n >>= 8n;
  }
  return Buffer.from(bytes).toString('hex');
};
process.stdout.write(JSON.stringify({
  initialHex,
  privateState: initial.currentPrivateState,
  digestHex: digest.map(fieldHex),
  generator: { x: fieldHex(generator.x), y: fieldHex(generator.y) },
  signatureResponseHex: fieldHex(signature.response),
  afterAcceptHex: Buffer.from(initial.currentContractState.serialize()).toString('hex'),
  afterVerifyPrivateState: verified.context.currentPrivateState,
  afterAcceptPrivateState: accepted.context.currentPrivateState,
  verifyTranscript: verified.proofData.privateTranscriptOutputs.map(
    ({ value, alignment }) => ({
      valueAtoms: value.map((atom) => Array.from(atom)),
      alignment,
    }),
  ),
  acceptTranscript: accepted.proofData.privateTranscriptOutputs.map(
    ({ value, alignment }) => ({
      valueAtoms: value.map((atom) => Array.from(atom)),
      alignment,
    }),
  ),
}, null, 2) + '\n');
