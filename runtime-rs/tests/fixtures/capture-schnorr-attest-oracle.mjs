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
const normalize = (value) => {
  if (typeof value === 'bigint') return value.toString();
  if (value instanceof Uint8Array) return Array.from(value);
  if (Array.isArray(value)) return value.map(normalize);
  if (value && typeof value === 'object') return Object.fromEntries(
    Object.entries(value).map(([key, inner]) => [key, normalize(inner)]),
  );
  return value;
};
const queries = [];
const originalQuery = runtime.QueryContext.prototype.query;
runtime.QueryContext.prototype.query = function (...args) {
  const result = originalQuery.call(this, ...args);
  queries.push({ gasCost: normalize(result.gasCost),
    opTags: args[0].map((op) => typeof op === 'string' ? op : Object.keys(op)[0]),
    raw: args[0] });
  return result;
};
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
const trace = (output, start, prior) => {
  const own = queries.slice(start);
  const replay = prior.currentQueryContext.query(own.flatMap(({ raw }) => raw), prior.costModel);
  return { resultGas: normalize(output.gasCost), vmProgram: normalize(output.proofData.publicTranscript),
    queries: own.map(({ gasCost, opTags }) => ({ gasCost, opTags })),
    replayGas: normalize(replay.gasCost) };
};
const verifyStart = queries.length;
const verified = contract.circuits.verifyAttestation(context, digest, signature);
const verifyTrace = trace(verified, verifyStart, context);
const acceptStart = queries.length;
const accepted = contract.circuits.acceptAttestation(verified.context, digest, signature);
const acceptTrace = trace(accepted, acceptStart, verified.context);
const rejected = (label, witnesses, sig) => {
  const local = new Contract(witnesses);
  const start = queries.length;
  try {
    local.circuits.verifyAttestation(context, digest, sig);
    throw new Error(`${label} unexpectedly accepted`);
  } catch (error) {
    if (String(error.message).includes('unexpectedly accepted')) throw error;
    return { error: String(error.message), queries: queries.slice(start)
      .map(({ gasCost, opTags }) => ({ gasCost, opTags })) };
  }
};
const badReduction = rejected('bad reduction', {
  getSchnorrReduction: ({ privateState }) => [privateState + 1, [0n, 0n]],
  localAttestorKey: ({ privateState }) => [privateState + 1, generator],
}, signature);
const identityAnnouncement = rejected('identity announcement', {
  getSchnorrReduction: ({ privateState }, challenge) =>
    [privateState + 1, [challenge >> 248n, challenge & ((1n << 248n) - 1n)]],
  localAttestorKey: ({ privateState }) => [privateState + 1, generator],
}, { announcement: runtime.ecMulGenerator(0n), response: signature.response });
const closedContext = runtime.createCircuitContext(
  runtime.dummyContractAddress(), coinPublicKey,
  initial.currentContractState.data, initial.currentPrivateState,
);
const byteIndex = new runtime.CompactTypeUnsignedInteger(255n, 1);
closedContext.currentQueryContext = closedContext.currentQueryContext.query([
  { push: { storage: false, value: runtime.StateValue.newCell({
    value: byteIndex.toValue(2n), alignment: byteIndex.alignment(),
  }).encode() } },
  { push: { storage: true, value: runtime.StateValue.newCell({
    value: runtime.CompactTypeBoolean.toValue(false),
    alignment: runtime.CompactTypeBoolean.alignment(),
  }).encode() } },
  { ins: { cached: false, n: 1 } },
], closedContext.costModel).context;
const closedStart = queries.length;
let closed;
try {
  contract.circuits.verifyAttestation(closedContext, digest, signature);
  throw new Error('closed attestor unexpectedly accepted');
} catch (error) {
  if (String(error.message).includes('unexpectedly accepted')) throw error;
  closed = { error: String(error.message), queries: queries.slice(closedStart)
    .map(({ gasCost, opTags }) => ({ gasCost, opTags })) };
}
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
  verifyTrace, acceptTrace, badReduction, identityAnnouncement, closed,
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
