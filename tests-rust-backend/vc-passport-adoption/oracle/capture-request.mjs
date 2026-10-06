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

if (process.argv.length !== 5) throw new Error('usage: node capture-request.mjs <generated-index.js> <profile> <output.json>');
const { pureCircuits } = await import(pathToFileURL(resolve(process.argv[2])).href);
const pad = (text) => { const out = new Uint8Array(32); out.set(Buffer.from(text, 'utf8')); return out; };
const nonzero = (seed) => Uint8Array.from({ length: 32 }, (_, index) => (seed + index) & 255);
const schema = { packageId: pad('midnight:vc:digital-passport'), schemaId: pad('digital-passport:v1'), majorVersion: 1n, minorVersion: 0n };
const method = { controllerAddress: { bytes: nonzero(1) }, methodId: nonzero(2) };
const request = {
  version: 1n, schema, issuerVerificationMethodRef: method,
  requireFirstNameDisclosure: false, requireLastNameDisclosure: false,
  requireAgeOverThreshold: false, requestedAgeThresholdYears: 0n,
  requireDocumentNumberDisclosure: false, requireIssuingStateDisclosure: false,
  verifierChallengeHash: nonzero(3),
};
const run = (name, fn) => {
  try { fn(); return { name, outcome: 'ok' }; }
  catch (error) { return { name, outcome: 'error', message: String(error.message) }; }
};
const rows = [
  run('family_schema_valid', () => pureCircuits.assertValidDigitalPassportSchemaRef(schema)),
  run('family_schema_wrong_package', () => pureCircuits.assertValidDigitalPassportSchemaRef({ ...schema, packageId: nonzero(4) })),
  run('family_schema_wrong_schema', () => pureCircuits.assertValidDigitalPassportSchemaRef({ ...schema, schemaId: nonzero(5) })),
  run('family_schema_wrong_major', () => pureCircuits.assertValidDigitalPassportSchemaRef({ ...schema, majorVersion: 2n })),
  run('family_schema_wrong_minor', () => pureCircuits.assertValidDigitalPassportSchemaRef({ ...schema, minorVersion: 1n })),
  run('matching_schema_valid', () => pureCircuits.assertMatchingSchemaRefs(schema, { ...schema })),
  run('matching_schema_minor_mismatch', () => pureCircuits.assertMatchingSchemaRefs(schema, { ...schema, minorVersion: 1n })),
  run('matching_schema_invalid_expected', () => pureCircuits.assertMatchingSchemaRefs({ ...schema, packageId: new Uint8Array(32) }, schema)),
  run('verification_method_valid', () => pureCircuits.assertValidVerificationMethodRef(method)),
  run('verification_method_missing_controller', () => pureCircuits.assertValidVerificationMethodRef({ ...method, controllerAddress: { bytes: new Uint8Array(32) } })),
  run('verification_method_missing_id', () => pureCircuits.assertValidVerificationMethodRef({ ...method, methodId: new Uint8Array(32) })),
  run('presentation_request_valid', () => pureCircuits.assertValidDigitalPassportPresentationRequest(request)),
  run('presentation_request_empty_challenge', () => pureCircuits.assertValidDigitalPassportPresentationRequest({ ...request, verifierChallengeHash: new Uint8Array(32) })),
  run('presentation_request_zero_age_required', () => pureCircuits.assertValidDigitalPassportPresentationRequest({ ...request, requireAgeOverThreshold: true })),
  run('presentation_request_nonzero_age_disabled', () => pureCircuits.assertValidDigitalPassportPresentationRequest({ ...request, requestedAgeThresholdYears: 18n })),
  run('presentation_request_wrong_schema_minor', () => pureCircuits.assertValidDigitalPassportPresentationRequest({ ...request, schema: { ...schema, minorVersion: 2n } })),
];
writeFileSync(process.argv[4], JSON.stringify({ profile: process.argv[3], rows }, null, 2) + '\n');
console.log(`${rows.length} request cases: ${rows.filter(row => row.outcome === 'ok').length} ok`);
