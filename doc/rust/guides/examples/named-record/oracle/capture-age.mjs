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

import { readFileSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

if (process.argv.length !== 6) {
  throw new Error('usage: node capture-age.mjs <generated-index.js> <age-inputs.json> <profile> <output.json>');
}
const { pureCircuits } = await import(pathToFileURL(resolve(process.argv[2])).href);
const inputs = JSON.parse(readFileSync(process.argv[3], 'utf8')).scenarios;
const bytes = (hex) => Uint8Array.from(Buffer.from(hex, 'hex'));
const date = (parts) => Object.fromEntries(Object.entries(parts).map(([key, value]) => [key, BigInt(value)]));
const rows = Object.entries(inputs).map(([name, input]) => {
  const zero = () => new Uint8Array(32);
  const verificationMethodRef = () => ({ controllerAddress: { bytes: zero() }, methodId: zero() });
  const schema = () => ({ packageId: zero(), schemaId: zero(), majorVersion: 0n, minorVersion: 0n });
  const holderBinding = () => ({ holderVerificationMethodRef: verificationMethodRef() });
  const credential = {
    version: 0n, schema: schema(), issuerVerificationMethodRef: verificationMethodRef(),
    holderBinding: holderBinding(), statusBinding: {}, issuedAt: 0n, hasExpiration: false,
    expiresAt: 0n, claims: {},
    claimCommitments: {
      firstNameCommitment: zero(), lastNameCommitment: zero(),
      dateOfBirthCommitment: bytes(input.dateOfBirthCommitment),
      documentNumberCommitment: zero(), issuingStateCommitment: zero(),
    }, claimRoot: zero(),
  };
  const presentation = {
    version: 0n, schema: schema(), credentialClaimRoot: zero(),
    issuerVerificationMethodRef: verificationMethodRef(), holderBinding: holderBinding(),
    disclosed: {
      revealFirstName: false, firstNameValuePadded: new Uint8Array(64), firstNameOpening: zero(),
      revealLastName: false, lastNameValuePadded: new Uint8Array(64), lastNameOpening: zero(),
      proveAgeOverThreshold: input.proveAgeOverThreshold, ageThresholdYears: BigInt(input.ageThresholdYears),
      revealDocumentNumber: false, documentNumberValue: zero(), documentNumberOpening: zero(),
      revealIssuingState: false, issuingStateValue: zero(), issuingStateOpening: zero(),
    },
  };
  try {
    pureCircuits.assertValidDigitalPassportAgePredicate(
      credential, presentation, BigInt(input.currentDay), BigInt(input.dateOfBirthDays),
      bytes(input.dateOfBirthOpening), date(input.currentDate), date(input.dateOfBirthDate));
    return { name, outcome: 'ok' };
  } catch (error) {
    return { name, outcome: 'error', message: String(error.message) };
  }
});
writeFileSync(process.argv[5], JSON.stringify({ profile: process.argv[4], rows }, null, 2) + '\n');
console.log(`${rows.length} age cases: ${rows.filter(row => row.outcome === 'ok').length} ok`);
