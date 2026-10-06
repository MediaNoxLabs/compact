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
const padded = text => {
  const bytes = new Uint8Array(32);
  bytes.set(Buffer.from(text));
  return bytes;
};
const schema = {
  packageId: padded('midnight:vc:digital-passport'),
  schemaId: padded('digital-passport:v1'), majorVersion: 1n, minorVersion: 0n,
};
const method = { controllerAddress: { bytes: b32(1) }, methodId: b32(2) };
const privateParts = {
  claimValues: {
    firstNameValuePadded: b64(10), lastNameValuePadded: b64(20),
    dateOfBirthDays: 12345n, documentNumberValue: b32(30), issuingStateValue: b32(40),
  },
  openings: {
    firstNameOpening: b32(100), lastNameOpening: b32(110), dateOfBirthOpening: b32(120),
    documentNumberOpening: b32(130), issuingStateOpening: b32(140),
  },
};
const values = privateParts.claimValues;
const openings = privateParts.openings;
const claimCommitments = {
  firstNameCommitment: pure.firstNameCommitment(values.firstNameValuePadded, openings.firstNameOpening),
  lastNameCommitment: pure.lastNameCommitment(values.lastNameValuePadded, openings.lastNameOpening),
  dateOfBirthCommitment: pure.dateOfBirthCommitment(values.dateOfBirthDays, openings.dateOfBirthOpening),
  documentNumberCommitment: pure.documentNumberCommitment(values.documentNumberValue, openings.documentNumberOpening),
  issuingStateCommitment: pure.issuingStateCommitment(values.issuingStateValue, openings.issuingStateOpening),
};
const credential = {
  version: 1n, schema, issuerVerificationMethodRef: method,
  holderBinding: { holderVerificationMethodRef: method }, statusBinding: {},
  issuedAt: 100n, hasExpiration: false, expiresAt: 0n, claims: {},
  claimCommitments, claimRoot: pure.digitalPassportClaimRoot(claimCommitments),
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
  revealFirstName: true, firstNameValuePadded: values.firstNameValuePadded,
  firstNameOpening: openings.firstNameOpening,
  revealLastName: false, lastNameValuePadded: values.lastNameValuePadded,
  lastNameOpening: openings.lastNameOpening,
  proveAgeOverThreshold: false, ageThresholdYears: 0n,
  revealDocumentNumber: false, documentNumberValue: values.documentNumberValue,
  documentNumberOpening: openings.documentNumberOpening,
  revealIssuingState: false, issuingStateValue: values.issuingStateValue,
  issuingStateOpening: openings.issuingStateOpening,
};
const presentation = {
  version: 1n, schema, credentialClaimRoot: credential.claimRoot,
  issuerVerificationMethodRef: method, holderBinding: credential.holderBinding, disclosed,
};
const presentationBodyRoot = pure.digitalPassportPresentationBodyRoot(presentation);
const presentationProof = sign(presentationBodyRoot, 'presentation', 11n);
const initial = {
  version: 1n, messageId: b32(10), threadId: b32(11), initialMessage: true,
  respondsToMessageId: pure.noProtocolResponseReference(), createdAt: 100n,
  hasExpiresAt: false, expiresAt: 0n,
};
const response = {
  ...initial, messageId: b32(12), initialMessage: false,
  respondsToMessageId: initial.messageId, createdAt: 101n,
};
const finalResponse = {
  ...response, messageId: b32(16), respondsToMessageId: response.messageId, createdAt: 102n,
};
const features = {
  supportsSelectiveDisclosure: true, supportsPredicateProofs: true,
  supportsVerifierScopedPseudonym: false, supportsSameHolderProof: false,
};
const offer = {
  envelope: initial, schema, issuerVerificationMethodRef: method,
  holderBindingProfile: 0, features,
  body: { supportsExpiration: false, defaultExpirationDays: 0n, requiresHolderPublicKey: false },
};
const issuanceRequest = {
  envelope: response, schema, issuerVerificationMethodRef: method, holderBindingProfile: 0,
  body: {
    holderBinding: { holderVerificationMethodRef: method }, holderPublicKey: ecMulGenerator(5n),
    holderChallengeHash: b32(55), requestExpiration: false, requestedExpirationDays: 0n,
  },
};
const issuanceResult = {
  envelope: finalResponse, schema, issuerVerificationMethodRef: method, holderBindingProfile: 0,
  body: { credential, credentialProof, holderPublicKey: issuanceRequest.body.holderPublicKey,
    issuanceChallengeHash: b32(55), privateParts },
};
const verificationRequest = {
  envelope: initial, schema, issuerVerificationMethodRef: method, holderBindingProfile: 0,
  features, verifierChallengeHash: b32(55),
  body: {
    requireFirstNameDisclosure: true, requireLastNameDisclosure: false,
    requireAgeOverThreshold: false, requestedAgeThresholdYears: 0n,
    requireDocumentNumberDisclosure: false, requireIssuingStateDisclosure: false,
  },
};
const submission = {
  envelope: response, schema, issuerVerificationMethodRef: method,
  holderBindingProfile: 0, challengeHash: b32(55),
  body: { credential, credentialProof, presentation, presentationProof },
};
const verificationResult = {
  envelope: finalResponse, approved: true,
  body: { credentialRoot: credentialBodyRoot, verifiedThresholdYears: 0n },
};
const run = (name, evaluate) => {
  try { return { name, outcome: 'ok', value: evaluate() }; }
  catch (error) { return { name, outcome: 'error', message: String(error.message) }; }
};
const rows = [
  run('full_issuance_result_valid', () => pure.assertValidDigitalPassportIssuanceResult(issuanceResult)),
  run('full_issuance_result_private_tamper', () => pure.assertValidDigitalPassportIssuanceResult({
    ...issuanceResult, body: { ...issuanceResult.body,
      privateParts: { ...privateParts, openings: { ...openings, firstNameOpening: b32(101) } } },
  })),
  run('full_issuance_result_matches_request', () => pure.assertDigitalPassportIssuanceResultMatchesRequest(issuanceRequest, issuanceResult)),
  run('full_issuance_result_other_holder', () => pure.assertDigitalPassportIssuanceResultMatchesRequest(issuanceRequest, {
    ...issuanceResult, body: { ...issuanceResult.body, holderPublicKey: ecMulGenerator(6n) },
  })),
  run('full_verification_submission_valid', () => pure.assertValidDigitalPassportVerificationSubmissionMessage(submission)),
  run('full_verification_submission_other_challenge', () => pure.assertValidDigitalPassportVerificationSubmissionMessage({ ...submission, challengeHash: b32(56) })),
  run('full_submission_matches_request', () => pure.assertDigitalPassportVerificationSubmissionMatchesRequest(verificationRequest, submission)),
  run('full_submission_missing_disclosure', () => pure.assertDigitalPassportVerificationSubmissionMatchesRequest({
    ...verificationRequest, body: { ...verificationRequest.body, requireLastNameDisclosure: true },
  }, submission)),
  run('full_result_matches_submission', () => pure.assertDigitalPassportVerificationResultMatchesSubmission(submission, verificationResult)),
  run('full_result_other_root', () => pure.assertDigitalPassportVerificationResultMatchesSubmission(submission, {
    ...verificationResult, body: { ...verificationResult.body, credentialRoot: b32(99) },
  })),
];
const vectors = {
  issuanceScalar: credentialProof.signature.s.toString(16),
  presentationScalar: presentationProof.signature.s.toString(16),
  credentialBodyRoot: Buffer.from(credentialBodyRoot).toString('hex'),
  presentationBodyRoot: Buffer.from(presentationBodyRoot).toString('hex'),
};
writeFileSync(output, `${JSON.stringify({ profile, vectors, rows }, null, 2)}\n`);
