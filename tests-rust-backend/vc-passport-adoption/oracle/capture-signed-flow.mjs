// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//   http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

import { pathToFileURL } from 'node:url';
import { writeFileSync } from 'node:fs';

const generated = new URL(pathToFileURL(process.argv[2]).href);
const { pureCircuits: pure } = await import(generated.href);
const { ecMulGenerator } = await import(new URL('./node_modules/@midnight-ntwrk/compact-runtime/dist/index.js', generated).href);
const [profile, output] = process.argv.slice(3);
const b32 = seed => Uint8Array.from({ length: 32 }, (_, index) => (seed + index) & 255);
const b64 = seed => Uint8Array.from({ length: 64 }, (_, index) => (seed + index) & 255);
const padded = text => { const bytes = new Uint8Array(32); bytes.set(Buffer.from(text)); return bytes; };
const method = { controllerAddress: { bytes: b32(1) }, methodId: b32(2) };
const schema = { packageId: padded('midnight:vc:digital-passport'), schemaId: padded('digital-passport:v1'), majorVersion: 1n, minorVersion: 0n };
const firstName = b64(20);
const opening = b32(21);
const commitments = {
  firstNameCommitment: pure.firstNameCommitment(firstName, opening),
  lastNameCommitment: b32(11), dateOfBirthCommitment: b32(12),
  documentNumberCommitment: b32(13), issuingStateCommitment: b32(14),
};
const credential = {
  version: 1n, schema, issuerVerificationMethodRef: method,
  holderBinding: { holderVerificationMethodRef: method }, statusBinding: {},
  issuedAt: 100n, hasExpiration: false, expiresAt: 0n,
  claims: {}, claimCommitments: commitments, claimRoot: pure.digitalPassportClaimRoot(commitments),
};
const subgroupOrder = 6554484396890773809930967563523245729705921265872317281365359162392183254199n;
const sign = (bodyRoot, context, nonce) => {
  const unsigned = {
    signerVerificationMethodRef: method, createdAt: 101n, challengeHash: b32(55),
    publicKey: ecMulGenerator(3n), signature: { r: ecMulGenerator(nonce), s: 0n },
  };
  const challenge = context === 'issuance'
    ? pure.issuanceProofChallenge(bodyRoot, unsigned)
    : pure.presentationProofChallenge(bodyRoot, unsigned);
  return { ...unsigned, signature: { ...unsigned.signature, s: (nonce + challenge * 3n) % subgroupOrder } };
};
const credentialBodyRoot = pure.digitalPassportCredentialBodyRoot(credential);
const credentialProof = sign(credentialBodyRoot, 'issuance', 7n);
const disclosed = {
  revealFirstName: true, firstNameValuePadded: firstName, firstNameOpening: opening,
  revealLastName: false, lastNameValuePadded: b64(22), lastNameOpening: b32(23),
  proveAgeOverThreshold: false, ageThresholdYears: 0n,
  revealDocumentNumber: false, documentNumberValue: b32(24), documentNumberOpening: b32(25),
  revealIssuingState: false, issuingStateValue: b32(26), issuingStateOpening: b32(27),
};
const presentation = {
  version: 1n, schema, credentialClaimRoot: credential.claimRoot,
  issuerVerificationMethodRef: method, holderBinding: credential.holderBinding, disclosed,
};
const presentationBodyRoot = pure.digitalPassportPresentationBodyRoot(presentation);
const presentationProof = sign(presentationBodyRoot, 'presentation', 11n);
const request = {
  version: 1n, schema, issuerVerificationMethodRef: method,
  requireFirstNameDisclosure: true, requireLastNameDisclosure: false,
  requireAgeOverThreshold: false, requestedAgeThresholdYears: 0n,
  requireDocumentNumberDisclosure: false, requireIssuingStateDisclosure: false,
  verifierChallengeHash: b32(55),
};
const run = (name, evaluate) => {
  try { return { name, outcome: 'ok', value: evaluate() }; }
  catch (error) { return { name, outcome: 'error', message: String(error.message) }; }
};
const rows = [
  run('issuance_context_signed', () => pure.assertValidIssuanceContextProof(credentialBodyRoot, credentialProof)),
  run('credential_signed', () => pure.assertValidDigitalPassportCredential(credential, credentialProof)),
  run('credential_mutated_after_signing', () => pure.assertValidDigitalPassportCredential({ ...credential, issuedAt: 101n }, credentialProof)),
  run('presentation_context_signed', () => pure.assertValidPresentationContextProof(presentationBodyRoot, presentationProof)),
  run('presentation_signed_and_disclosed', () => pure.assertValidDigitalPassportPresentation(credential, credentialProof, presentation, presentationProof)),
  run('presentation_disclosure_mutated', () => pure.assertValidDigitalPassportPresentation(credential, credentialProof, { ...presentation, disclosed: { ...disclosed, firstNameValuePadded: b64(30) } }, presentationProof)),
  run('request_satisfied', () => pure.assertDigitalPassportPresentationSatisfiesRequest(credential, request, presentation, presentationProof)),
  run('request_challenge_changed', () => pure.assertDigitalPassportPresentationSatisfiesRequest(credential, { ...request, verifierChallengeHash: b32(56) }, presentation, presentationProof)),
  run('request_adds_last_name', () => pure.assertDigitalPassportPresentationSatisfiesRequest(credential, { ...request, requireLastNameDisclosure: true }, presentation, presentationProof)),
];
const fieldHex = field => field.toString(16);
const point = value => ({ x: fieldHex(value.x), y: fieldHex(value.y) });
const vectors = {
  publicKey: point(credentialProof.publicKey),
  issuanceNoncePoint: point(credentialProof.signature.r),
  presentationNoncePoint: point(presentationProof.signature.r),
  credentialBodyRoot: Buffer.from(credentialBodyRoot).toString('hex'),
  presentationBodyRoot: Buffer.from(presentationBodyRoot).toString('hex'),
  issuanceScalar: fieldHex(credentialProof.signature.s),
  presentationScalar: fieldHex(presentationProof.signature.s),
};
writeFileSync(output, `${JSON.stringify({ profile, vectors, rows }, null, 2)}\n`);
