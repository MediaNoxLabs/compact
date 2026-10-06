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

if (process.argv.length !== 5) {
  throw new Error('usage: node capture.mjs <generated-contract-index.js> <profile> <output.json>');
}
const { pureCircuits } = await import(pathToFileURL(resolve(process.argv[2])).href);

const b32 = (seed) => Uint8Array.from({ length: 32 }, (_, i) => (seed + i) & 255);
const b64 = (seed) => Uint8Array.from({ length: 64 }, (_, i) => (seed + i) & 255);
const hex = (bytes) => Buffer.from(bytes).toString('hex');
const noResponse = pureCircuits.noProtocolResponseReference();
const run = (name, fn) => {
  try {
    const value = fn();
    return { name, outcome: 'ok', value: value instanceof Uint8Array ? hex(value) : 'unit' };
  } catch (error) {
    return { name, outcome: 'error', message: String(error.message) };
  }
};
const claims = {
  firstNameCommitment: b32(1),
  lastNameCommitment: b32(2),
  dateOfBirthCommitment: b32(3),
  documentNumberCommitment: b32(4),
  issuingStateCommitment: b32(5),
};
const schema = { packageId: b32(1), schemaId: b32(2), majorVersion: 1n, minorVersion: 0n };
const envelope = {
  version: 1n,
  messageId: b32(1),
  threadId: b32(2),
  initialMessage: true,
  respondsToMessageId: noResponse,
  createdAt: 100n,
  hasExpiresAt: true,
  expiresAt: 200n,
};
const rows = [
  run('protocol_none_reference', () => pureCircuits.noProtocolResponseReference()),
  run('document_number_null_commitment', () => pureCircuits.documentNumberNullCommitment()),
  run('issuance_context_tag', () => pureCircuits.issuanceContextTag()),
  run('presentation_context_tag', () => pureCircuits.presentationContextTag()),
  run('signer_authorization_context_tag', () => pureCircuits.signerAuthorizationContextTag()),
  run('verifier_request_context_tag', () => pureCircuits.verifierRequestContextTag()),
  run('last_name_commitment', () => pureCircuits.lastNameCommitment(b64(7), b32(71))),
  run('document_number_commitment', () => pureCircuits.documentNumberCommitment(b32(13), b32(72))),
  run('issuing_state_commitment', () => pureCircuits.issuingStateCommitment(b32(14), b32(73))),
  run('first_name_commitment_opening_a', () => pureCircuits.firstNameCommitment(b64(0), b32(160))),
  run('first_name_commitment_opening_b', () => pureCircuits.firstNameCommitment(b64(0), b32(161))),
  run('date_of_birth_commitment_zero', () => pureCircuits.dateOfBirthCommitment(0n, b32(32))),
  run('date_of_birth_commitment_max', () => pureCircuits.dateOfBirthCommitment(4294967295n, b32(32))),
  run('claim_root_base', () => pureCircuits.digitalPassportClaimRoot(claims)),
  run('claim_root_changed_document', () => pureCircuits.digitalPassportClaimRoot({ ...claims, documentNumberCommitment: b32(99) })),
  run('claim_root_changed_date', () => pureCircuits.digitalPassportClaimRoot({ ...claims, dateOfBirthCommitment: b32(99) })),
  run('schema_valid', () => pureCircuits.assertValidSchemaRef(schema)),
  run('schema_missing_package', () => pureCircuits.assertValidSchemaRef({ ...schema, packageId: new Uint8Array(32) })),
  run('schema_missing_schema', () => pureCircuits.assertValidSchemaRef({ ...schema, schemaId: new Uint8Array(32) })),
  run('schema_zero_major', () => pureCircuits.assertValidSchemaRef({ ...schema, majorVersion: 0n })),
  run('envelope_valid_initial', () => pureCircuits.assertValidProtocolMessageEnvelope(envelope)),
  run('envelope_valid_response', () => pureCircuits.assertValidProtocolMessageEnvelope({ ...envelope, initialMessage: false, respondsToMessageId: b32(3) })),
  run('envelope_invalid_version', () => pureCircuits.assertValidProtocolMessageEnvelope({ ...envelope, version: 2n })),
  run('envelope_initial_with_reference', () => pureCircuits.assertValidProtocolMessageEnvelope({ ...envelope, respondsToMessageId: b32(3) })),
  run('envelope_expiry_before_creation', () => pureCircuits.assertValidProtocolMessageEnvelope({ ...envelope, expiresAt: 99n })),
];
const capture = { profile: process.argv[3], rows };
writeFileSync(process.argv[4], JSON.stringify(capture, null, 2) + '\n');
console.log(`${rows.length} cases: ${rows.filter(x => x.outcome === 'ok').length} ok, ${rows.filter(x => x.outcome === 'error').length} errors`);
