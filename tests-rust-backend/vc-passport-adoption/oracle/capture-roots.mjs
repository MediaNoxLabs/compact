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

const { pureCircuits: pure } = await import(pathToFileURL(process.argv[2]).href);
const [profile, output] = process.argv.slice(3);
const b32 = seed => Uint8Array.from({ length: 32 }, (_, index) => (seed + index) & 255);
const b64 = seed => Uint8Array.from({ length: 64 }, (_, index) => (seed + index) & 255);
const padded = text => {
  const bytes = new Uint8Array(32);
  bytes.set(Buffer.from(text));
  return bytes;
};
const schema = {
  packageId: padded('midnight:vc:digital-passport'),
  schemaId: padded('digital-passport:v1'),
  majorVersion: 1n,
  minorVersion: 0n,
};
const method = { controllerAddress: { bytes: b32(1) }, methodId: b32(2) };
const commitments = {
  firstNameCommitment: b32(10), lastNameCommitment: b32(11),
  dateOfBirthCommitment: b32(12), documentNumberCommitment: b32(13),
  issuingStateCommitment: b32(14),
};
const holderBinding = { holderVerificationMethodRef: method };
const claimRoot = pure.digitalPassportClaimRoot(commitments);
const credential = {
  version: 1n, schema, issuerVerificationMethodRef: method, holderBinding,
  statusBinding: {}, issuedAt: 100n, hasExpiration: false, expiresAt: 0n,
  claims: {}, claimCommitments: commitments, claimRoot,
};
const disclosed = {
  revealFirstName: false, firstNameValuePadded: b64(20), firstNameOpening: b32(21),
  revealLastName: false, lastNameValuePadded: b64(22), lastNameOpening: b32(23),
  proveAgeOverThreshold: true, ageThresholdYears: 18n,
  revealDocumentNumber: false, documentNumberValue: b32(24), documentNumberOpening: b32(25),
  revealIssuingState: false, issuingStateValue: b32(26), issuingStateOpening: b32(27),
};
const presentation = {
  version: 1n, schema, credentialClaimRoot: claimRoot,
  issuerVerificationMethodRef: method, holderBinding, disclosed,
};
const proof = {
  signerVerificationMethodRef: method, createdAt: 100n, challengeHash: b32(55),
  publicKey: { x: 0n, y: 1n },
  signature: { r: { x: 0n, y: 1n }, s: 0n },
};
const bodyRoot = b32(33);
const encode = value => value instanceof Uint8Array
  ? Buffer.from(value).toString('hex')
  : typeof value === 'bigint' ? value.toString(16) : value;
const run = (name, evaluate) => {
  try { return { name, outcome: 'ok', value: encode(evaluate()) }; }
  catch (error) { return { name, outcome: 'error', message: String(error.message) }; }
};
const rows = [
  run('credential_body_root_base', () => pure.digitalPassportCredentialBodyRoot(credential)),
  run('credential_body_root_changed_issued_at', () => pure.digitalPassportCredentialBodyRoot({ ...credential, issuedAt: 101n })),
  run('presentation_body_root_base', () => pure.digitalPassportPresentationBodyRoot(presentation)),
  run('presentation_body_root_changed_disclosure', () => pure.digitalPassportPresentationBodyRoot({ ...presentation, disclosed: { ...disclosed, revealFirstName: true } })),
  run('issuance_payload_root', () => pure.issuanceProofPayloadRoot(bodyRoot, proof)),
  run('presentation_payload_root', () => pure.presentationProofPayloadRoot(bodyRoot, proof)),
  run('issuance_challenge', () => pure.issuanceProofChallenge(bodyRoot, proof)),
  run('presentation_challenge', () => pure.presentationProofChallenge(bodyRoot, proof)),
  run('signer_authorization_challenge', () => pure.signerAuthorizationProofChallenge(bodyRoot, proof)),
  run('verifier_request_challenge', () => pure.verifierRequestProofChallenge(bodyRoot, proof)),
];
writeFileSync(output, `${JSON.stringify({ profile, rows }, null, 2)}\n`);
