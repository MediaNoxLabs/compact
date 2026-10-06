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
const zero = new Uint8Array(32);
const padded = text => { const bytes = new Uint8Array(32); bytes.set(Buffer.from(text)); return bytes; };
const schema = { packageId: padded('midnight:vc:digital-passport'), schemaId: padded('digital-passport:v1'), majorVersion: 1n, minorVersion: 0n };
const method = { controllerAddress: { bytes: b32(1) }, methodId: b32(2) };
const features = { supportsSelectiveDisclosure: true, supportsPredicateProofs: true, supportsVerifierScopedPseudonym: false, supportsSameHolderProof: false };
const initial = { version: 1n, messageId: b32(10), threadId: b32(11), initialMessage: true, respondsToMessageId: pure.noProtocolResponseReference(), createdAt: 100n, hasExpiresAt: false, expiresAt: 0n };
const response = { ...initial, messageId: b32(12), initialMessage: false, respondsToMessageId: initial.messageId, createdAt: 101n };
const finalResponse = { ...response, messageId: b32(16), respondsToMessageId: response.messageId, createdAt: 102n };
const offer = { envelope: initial, schema, issuerVerificationMethodRef: method, holderBindingProfile: 0, features,
  body: { supportsExpiration: false, defaultExpirationDays: 0n, requiresHolderPublicKey: false } };
const issuanceRequest = { envelope: response, schema, issuerVerificationMethodRef: method, holderBindingProfile: 0,
  body: { holderBinding: { holderVerificationMethodRef: method }, holderPublicKey: { x: 0n, y: 1n }, holderChallengeHash: b32(13), requestExpiration: false, requestedExpirationDays: 0n } };
const claimCommitments = { firstNameCommitment: b32(20), lastNameCommitment: b32(21), dateOfBirthCommitment: b32(22), documentNumberCommitment: b32(23), issuingStateCommitment: b32(24) };
const credential = { version: 1n, schema, issuerVerificationMethodRef: method, holderBinding: { holderVerificationMethodRef: method }, statusBinding: {}, issuedAt: 100n, hasExpiration: false, expiresAt: 0n, claims: {}, claimCommitments, claimRoot: b32(25) };
const proof = { signerVerificationMethodRef: method, createdAt: 101n, challengeHash: b32(13), publicKey: { x: 0n, y: 1n }, signature: { r: { x: 0n, y: 1n }, s: 0n } };
const privateParts = { claimValues: { firstNameValuePadded: b64(30), lastNameValuePadded: b64(31), dateOfBirthDays: 12345n, documentNumberValue: b32(32), issuingStateValue: b32(33) }, openings: { firstNameOpening: b32(40), lastNameOpening: b32(41), dateOfBirthOpening: b32(42), documentNumberOpening: b32(43), issuingStateOpening: b32(44) } };
const issuanceResult = { envelope: finalResponse, schema, issuerVerificationMethodRef: method, holderBindingProfile: 0,
  body: { credential, credentialProof: proof, holderPublicKey: { x: 0n, y: 1n }, issuanceChallengeHash: b32(13), privateParts } };
const verificationRequest = { envelope: initial, schema, issuerVerificationMethodRef: method, holderBindingProfile: 0, features,
  verifierChallengeHash: b32(14), body: { requireFirstNameDisclosure: true, requireLastNameDisclosure: false,
    requireAgeOverThreshold: true, requestedAgeThresholdYears: 18n, requireDocumentNumberDisclosure: false, requireIssuingStateDisclosure: false } };
const disclosed = { revealFirstName: false, firstNameValuePadded: b64(50), firstNameOpening: b32(51), revealLastName: false, lastNameValuePadded: b64(52), lastNameOpening: b32(53), proveAgeOverThreshold: false, ageThresholdYears: 0n, revealDocumentNumber: false, documentNumberValue: b32(54), documentNumberOpening: b32(55), revealIssuingState: false, issuingStateValue: b32(56), issuingStateOpening: b32(57) };
const presentation = { version: 1n, schema, credentialClaimRoot: b32(25), issuerVerificationMethodRef: method, holderBinding: { holderVerificationMethodRef: method }, disclosed };
const submission = { envelope: response, schema, issuerVerificationMethodRef: method, holderBindingProfile: 0, challengeHash: b32(14), body: { credential, credentialProof: proof, presentation, presentationProof: proof } };
const verificationResult = { envelope: finalResponse, approved: true, body: { credentialRoot: b32(15), verifiedThresholdYears: 18n } };
const run = (name, evaluate) => {
  try { return { name, outcome: 'ok', value: evaluate() }; }
  catch (error) { return { name, outcome: 'error', message: String(error.message) }; }
};
const rows = [
  run('generic_issuance_result_valid', () => pure.DigitalPassportIssuance_assertValidResultMessage(issuanceResult)),
  run('generic_issuance_result_initial', () => pure.DigitalPassportIssuance_assertValidResultMessage({ ...issuanceResult, envelope: initial })),
  run('generic_issuance_result_alignment', () => pure.DigitalPassportIssuance_assertRequestResultAlignment(issuanceRequest, issuanceResult)),
  run('generic_issuance_result_wrong_thread', () => pure.DigitalPassportIssuance_assertRequestResultAlignment(issuanceRequest, { ...issuanceResult, envelope: { ...finalResponse, threadId: b32(88) } })),
  run('generic_submission_valid', () => pure.DigitalPassportVerification_assertValidSubmissionMessage(submission)),
  run('generic_submission_initial', () => pure.DigitalPassportVerification_assertValidSubmissionMessage({ ...submission, envelope: initial })),
  run('generic_submission_alignment', () => pure.DigitalPassportVerification_assertRequestSubmissionAlignment(verificationRequest, submission)),
  run('generic_submission_wrong_challenge', () => pure.DigitalPassportVerification_assertRequestSubmissionAlignment(verificationRequest, { ...submission, challengeHash: b32(90) })),
  run('generic_result_alignment', () => pure.DigitalPassportVerification_assertSubmissionResultAlignment(submission, verificationResult)),
  run('generic_result_wrong_previous', () => pure.DigitalPassportVerification_assertSubmissionResultAlignment(submission, { ...verificationResult, envelope: { ...finalResponse, respondsToMessageId: b32(89) } })),
  run('passport_offer_valid', () => pure.assertValidDigitalPassportIssuanceOffer(offer)),
  run('passport_offer_bad_expiration', () => pure.assertValidDigitalPassportIssuanceOffer({ ...offer, body: { ...offer.body, defaultExpirationDays: 1n } })),
  run('passport_request_valid', () => pure.assertValidDigitalPassportIssuanceRequest(issuanceRequest)),
  run('passport_request_missing_challenge', () => pure.assertValidDigitalPassportIssuanceRequest({ ...issuanceRequest, body: { ...issuanceRequest.body, holderChallengeHash: pure.noProtocolResponseReference() } })),
  run('passport_offer_request_match', () => pure.assertDigitalPassportIssuanceRequestMatchesOffer(offer, issuanceRequest)),
  run('passport_offer_request_unsupported_expiration', () => pure.assertDigitalPassportIssuanceRequestMatchesOffer(offer, { ...issuanceRequest, body: { ...issuanceRequest.body, requestExpiration: true, requestedExpirationDays: 1n } })),
  run('passport_verification_request_valid', () => pure.assertValidDigitalPassportVerificationRequestMessage(verificationRequest)),
  run('passport_verification_request_missing_method', () => pure.assertValidDigitalPassportVerificationRequestMessage({ ...verificationRequest, issuerVerificationMethodRef: { ...method, methodId: zero } })),
  run('passport_verification_result_valid', () => pure.assertValidDigitalPassportVerificationResultMessage(verificationResult)),
  run('passport_verification_result_missing_root', () => pure.assertValidDigitalPassportVerificationResultMessage({ ...verificationResult, body: { ...verificationResult.body, credentialRoot: pure.noProtocolResponseReference() } })),
];
writeFileSync(output, `${JSON.stringify({ profile, rows }, null, 2)}\n`);
