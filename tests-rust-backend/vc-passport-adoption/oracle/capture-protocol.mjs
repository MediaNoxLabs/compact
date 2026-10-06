// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//  http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

import { writeFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

if (process.argv.length !== 5) throw new Error('usage: node capture-protocol.mjs <generated-index.js> <profile> <output.json>');
const { pureCircuits: pure } = await import(pathToFileURL(resolve(process.argv[2])).href);
const b32 = (seed) => Uint8Array.from({ length: 32 }, (_, index) => (seed + index) & 255);
const pad = (text) => { const out = new Uint8Array(32); out.set(Buffer.from(text)); return out; };
const schema = { packageId: pad('midnight:vc:digital-passport'), schemaId: pad('digital-passport:v1'), majorVersion: 1n, minorVersion: 0n };
const method = { controllerAddress: { bytes: b32(1) }, methodId: b32(2) };
const features = { supportsSelectiveDisclosure: true, supportsPredicateProofs: true, supportsVerifierScopedPseudonym: false, supportsSameHolderProof: false };
const initial = { version: 1n, messageId: b32(10), threadId: b32(11), initialMessage: true, respondsToMessageId: pure.noProtocolResponseReference(), createdAt: 100n, hasExpiresAt: false, expiresAt: 0n };
const response = { ...initial, messageId: b32(12), initialMessage: false, respondsToMessageId: initial.messageId, createdAt: 101n };
const offer = { envelope: initial, schema, issuerVerificationMethodRef: method, holderBindingProfile: 0, features,
  body: { supportsExpiration: false, defaultExpirationDays: 0n, requiresHolderPublicKey: false } };
const issuanceRequest = { envelope: response, schema, issuerVerificationMethodRef: method, holderBindingProfile: 0,
  body: { holderBinding: { holderVerificationMethodRef: method }, holderPublicKey: { x: 0n, y: 1n }, holderChallengeHash: b32(13), requestExpiration: false, requestedExpirationDays: 0n } };
const verificationRequest = { envelope: initial, schema, issuerVerificationMethodRef: method, holderBindingProfile: 0, features,
  verifierChallengeHash: b32(14), body: { requireFirstNameDisclosure: true, requireLastNameDisclosure: false, requireAgeOverThreshold: true,
    requestedAgeThresholdYears: 18n, requireDocumentNumberDisclosure: false, requireIssuingStateDisclosure: false } };
const verificationResult = { envelope: response, approved: true, body: { credentialRoot: b32(15), verifiedThresholdYears: 18n } };
const norm = (value) => value instanceof Uint8Array ? Buffer.from(value).toString('hex') :
  typeof value === 'bigint' ? value.toString() : Array.isArray(value) ? value.map(norm) :
  value && typeof value === 'object' ? Object.fromEntries(Object.entries(value).map(([key, entry]) => [key, norm(entry)])) :
  value === undefined ? 'unit' : value;
const run = (name, fn) => { try { return { name, outcome: 'ok', value: norm(fn()) }; }
  catch (error) { return { name, outcome: 'error', message: String(error.message) }; } };
const rows = [
  run('response_envelope_valid', () => pure.assertProtocolResponseEnvelope(initial, response)),
  run('response_envelope_wrong_thread', () => pure.assertProtocolResponseEnvelope(initial, { ...response, threadId: b32(99) })),
  run('response_envelope_wrong_previous', () => pure.assertProtocolResponseEnvelope(initial, { ...response, respondsToMessageId: b32(98) })),
  run('response_envelope_early', () => pure.assertProtocolResponseEnvelope(initial, { ...response, createdAt: 99n })),
  run('response_envelope_initial', () => pure.assertProtocolResponseEnvelope(initial, { ...initial, messageId: b32(12) })),
  run('issuance_offer_valid', () => pure.DigitalPassportIssuance_assertValidOfferMessage(offer)),
  run('issuance_offer_response_envelope', () => pure.DigitalPassportIssuance_assertValidOfferMessage({ ...offer, envelope: response })),
  run('issuance_request_valid', () => pure.DigitalPassportIssuance_assertValidRequestMessage(issuanceRequest)),
  run('issuance_request_initial_envelope', () => pure.DigitalPassportIssuance_assertValidRequestMessage({ ...issuanceRequest, envelope: initial })),
  run('issuance_alignment_valid', () => pure.DigitalPassportIssuance_assertOfferRequestAlignment(offer, issuanceRequest)),
  run('issuance_alignment_wrong_method', () => pure.DigitalPassportIssuance_assertOfferRequestAlignment(offer, { ...issuanceRequest, issuerVerificationMethodRef: { ...method, methodId: b32(55) } })),
  run('issuance_alignment_wrong_schema', () => pure.DigitalPassportIssuance_assertOfferRequestAlignment(offer, { ...issuanceRequest, schema: { ...schema, minorVersion: 1n } })),
  run('issuance_alignment_wrong_thread', () => pure.DigitalPassportIssuance_assertOfferRequestAlignment(offer, { ...issuanceRequest, envelope: { ...response, threadId: b32(88) } })),
  run('verification_request_valid', () => pure.DigitalPassportVerification_assertValidRequestMessage(verificationRequest)),
  run('verification_request_response_envelope', () => pure.DigitalPassportVerification_assertValidRequestMessage({ ...verificationRequest, envelope: response })),
  run('verification_result_valid', () => pure.DigitalPassportVerification_assertValidResultMessage(verificationResult)),
  run('verification_result_initial_envelope', () => pure.DigitalPassportVerification_assertValidResultMessage({ ...verificationResult, envelope: initial })),
  run('presentation_from_protocol', () => pure.digitalPassportPresentationRequestFromProtocol(verificationRequest)),
  run('presentation_from_future_transport', () => pure.digitalPassportPresentationRequestFromProtocol({ ...verificationRequest, envelope: { ...initial, version: 2n } })),
  run('presentation_request_body_root', () => pure.digitalPassportPresentationRequestBodyRoot(pure.digitalPassportPresentationRequestFromProtocol(verificationRequest))),
];
writeFileSync(process.argv[4], JSON.stringify({ profile: process.argv[3], rows }, null, 2) + '\n');
console.log(`${rows.length} protocol cases: ${rows.filter(row => row.outcome === 'ok').length} ok`);
