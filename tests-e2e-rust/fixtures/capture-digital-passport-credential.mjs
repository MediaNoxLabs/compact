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

// SPDX-License-Identifier: Apache-2.0
//
// Capture TS reference outcomes for the vendored digital-passport dogfood
// contract (examples/dogfood/digital-passport-credential/ — third-party
// enclave, see its PROVENANCE.md). Unlike the small fixtures, which only
// capture the constructor's state bytes, this contract is a pure-circuit
// library, so the reference pins OUTCOMES: every case below records either
// "ok" or the exact `failed assert:` message the TS backend produced, plus
// the byte values (hex) of every derived root/commitment/point the Rust
// parity test (tests/digital_passport_credential.rs) must reproduce.
//
// Coverage (per the change's representative-subset contract):
//   1. Civil-date helper behavior — `assertCivilDateMatchesEpochDays` is
//      internal on both backends, so every one of its ternary sites and
//      assert-fail paths is driven through the exported
//      `assertValidDigitalPassportAgePredicate` (the only call site — the
//      same route upstream's own vitest suite takes). Distinct assert
//      messages attribute each failure to the exact check.
//   2. The full age predicate: calendar-anniversary semantics, leap-day
//      holders, the `beforeBirthdayThisYear ? 1 : 0` ternary both ways,
//      commitment mismatch, day ordering, forged decompositions.
//   3. One issuance/presentation/verification protocol round-trip over the
//      upstream-shaped fixture (real Jubjub keys, real proofs — the result
//      and submission validators verify them), plus the two tamper
//      rejections upstream's protocol test pins.
//
// Fixture construction is a faithful port of upstream
// src/testing/{credential-fixtures,civil-date,jubjub-utils}.ts: same
// labels, scalars, and constants. The reference originals live upstream
// at the pinned rev (see PROVENANCE.md) — only the `.compact` sources are
// vendored in this repo, so the port has no in-tree copy to diff against.
//
// Usage:
//   compactc --target ts --skip-zk \
//     examples/dogfood/digital-passport-credential/src/digital-passport-credential.compact \
//     /tmp/dpp-ts-driver/
//   echo '{"type":"module"}' > /tmp/dpp-ts-driver/contract/package.json
//   ln -sfn "$PWD/node_modules" /tmp/dpp-ts-driver/contract/node_modules
//   node tests-e2e-rust/fixtures/capture-digital-passport-credential.mjs \
//     > tests-e2e-rust/fixtures/digital-passport-credential-ts-state.json

import { createHash } from 'node:crypto';
import { TextEncoder } from 'node:util';

import { HolderBindingProfile, pureCircuits } from '/tmp/dpp-ts-driver/contract/index.js';
import * as cr from '@midnight-ntwrk/compact-runtime';

// =====================
// Upstream testing-utils port (rev cdeb860b)
// =====================

const JUBJUB_SUBGROUP_ORDER =
  6554484396890773809930967563523245729705921265872317281365359162392183254199n;
const mod = (value) => {
  const reduced = value % JUBJUB_SUBGROUP_ORDER;
  return reduced >= 0n ? reduced : reduced + JUBJUB_SUBGROUP_ORDER;
};

const sha256 = (value) => new Uint8Array(createHash('sha256').update(value).digest());

const padText = (value, length = 32) => {
  const bytes = new TextEncoder().encode(value);
  if (bytes.length >= length) {
    return bytes.subarray(0, length);
  }
  const padded = new Uint8Array(length);
  padded.set(bytes);
  return padded;
};

// Howard Hinnant's days_from_civil (upstream src/testing/civil-date.ts).
const epochDaysFromCivil = (year, month, day) => {
  const yearAdjusted = month <= 2 ? year - 1 : year;
  const era = Math.floor(yearAdjusted / 400);
  const yearOfEra = yearAdjusted - era * 400;
  const dayOfyear = Math.floor((153 * (month + (month > 2 ? -3 : 9)) + 2) / 5) + day - 1;
  const dayOfEra = yearOfEra * 365 + Math.floor(yearOfEra / 4) - Math.floor(yearOfEra / 100) + dayOfyear;
  return BigInt(era * 146097 + dayOfEra - 719468);
};

// The circuit-facing decomposition (upstream src/testing/civil-date.ts).
const civilDateFromEpochDays = (epochDays) => {
  const z = Number(epochDays) + 719468;
  const era = Math.floor(z / 146097);
  const dayOfEra = z - era * 146097;
  const yearOfEra = Math.floor(
    (dayOfEra - Math.floor(dayOfEra / 1460) + Math.floor(dayOfEra / 36524) - Math.floor(dayOfEra / 146096)) / 365,
  );
  const year = yearOfEra + era * 400;
  const dayOfYear = dayOfEra - (365 * yearOfEra + Math.floor(yearOfEra / 4) - Math.floor(yearOfEra / 100));
  const monthPosition = Math.floor((5 * dayOfYear + 2) / 153);
  const day = dayOfYear - Math.floor((153 * monthPosition + 2) / 5) + 1;
  const month = monthPosition + (monthPosition < 10 ? 3 : -9);
  const civilYear = year + (month <= 2 ? 1 : 0);
  const yearAdjusted = month <= 2 ? civilYear - 1 : civilYear;
  const shiftedMonth = month >= 3 ? month - 3 : month + 9;
  return {
    year: BigInt(civilYear),
    month: BigInt(month),
    day: BigInt(day),
    yearAdjustedQuotient4: BigInt(Math.floor(yearAdjusted / 4)),
    yearAdjustedQuotient100: BigInt(Math.floor(yearAdjusted / 100)),
    yearAdjustedQuotient400: BigInt(Math.floor(yearAdjusted / 400)),
    marchBasedMonthDayOffset: BigInt(Math.floor((153 * shiftedMonth + 2) / 5)),
  };
};

const contractAddress = (label) => ({ bytes: sha256(`contract:${label}`) });

const createSigner = (label, secretKey, methodId = `#${label}-key-1`) => ({
  label,
  secretKey,
  publicKey: cr.ecMulGenerator(secretKey),
  verificationMethodRef: {
    didContractAddress: contractAddress(label),
    methodId: padText(methodId),
  },
});

const createProtocolEnvelope = ({ label, threadLabel, initialMessage, respondsToMessageId, createdAt }) => ({
  version: 1n,
  messageId: sha256(`protocol:message:${label}`),
  threadId: sha256(`protocol:thread:${threadLabel}`),
  initialMessage,
  respondsToMessageId: respondsToMessageId ?? pureCircuits.noProtocolResponseReference(),
  createdAt,
  hasExpiresAt: false,
  expiresAt: 0n,
});

const deriveProofChallenge = (bodyRoot, proof, context) =>
  context === 'issuance'
    ? pureCircuits.issuanceProofChallenge(bodyRoot, proof)
    : pureCircuits.presentationProofChallenge(bodyRoot, proof);

const signProof = ({ bodyRoot, context, signer, createdAt, challengeHash, nonceScalar }) => {
  const proof = {
    signerVerificationMethodRef: signer.verificationMethodRef,
    createdAt,
    challengeHash,
    publicKey: signer.publicKey,
    signature: { r: cr.ecMulGenerator(nonceScalar), s: 0n },
  };
  const challenge = deriveProofChallenge(bodyRoot, proof, context);
  return {
    ...proof,
    signature: { r: proof.signature.r, s: mod(nonceScalar + challenge * signer.secretKey) },
  };
};

// =====================
// Fixture (upstream createDigitalPassportFixture / …ProtocolFixture)
// =====================

const ISSUER = createSigner('issuer', 123456789n);
const HOLDER = createSigner('holder', 987654321n);
const VERIFIER_CHALLENGE = sha256('challenge:verifier');

const buildFixture = (dateOfBirthDays = 3650n, hasDocumentNumber = true) => {
  const claimValues = {
    firstNameValuePadded: padText('Alice', 64),
    lastNameValuePadded: padText('Example', 64),
    dateOfBirthDays,
    documentNumberValue: hasDocumentNumber ? padText('AB1234567', 32) : new Uint8Array(32),
    issuingStateValue: padText('US', 32),
  };
  const openings = {
    firstNameOpening: sha256('opening:first-name'),
    lastNameOpening: sha256('opening:last-name'),
    dateOfBirthOpening: sha256('opening:date-of-birth'),
    documentNumberOpening: hasDocumentNumber ? sha256('opening:document-number') : new Uint8Array(32),
    issuingStateOpening: sha256('opening:issuing-state'),
  };
  const claimCommitments = {
    firstNameCommitment: pureCircuits.firstNameCommitment(claimValues.firstNameValuePadded, openings.firstNameOpening),
    lastNameCommitment: pureCircuits.lastNameCommitment(claimValues.lastNameValuePadded, openings.lastNameOpening),
    dateOfBirthCommitment: pureCircuits.dateOfBirthCommitment(claimValues.dateOfBirthDays, openings.dateOfBirthOpening),
    documentNumberCommitment: hasDocumentNumber
      ? pureCircuits.documentNumberCommitment(claimValues.documentNumberValue, openings.documentNumberOpening)
      : pureCircuits.documentNumberNullCommitment(),
    issuingStateCommitment: pureCircuits.issuingStateCommitment(claimValues.issuingStateValue, openings.issuingStateOpening),
  };
  const schema = {
    packageId: padText('midnight:vc:digital-passport'),
    schemaId: padText('digital-passport:v1'),
    majorVersion: 1n,
    minorVersion: 0n,
  };
  const credential = {
    version: 1n,
    schema,
    issuerVerificationMethodRef: ISSUER.verificationMethodRef,
    holderBinding: { holderVerificationMethodRef: HOLDER.verificationMethodRef },
    statusBinding: {},
    issuedAt: 10_000n,
    hasExpiration: true,
    expiresAt: 20_000n,
    claims: {},
    claimCommitments,
    claimRoot: pureCircuits.digitalPassportClaimRoot(claimCommitments),
  };
  const presentation = {
    version: 1n,
    schema,
    credentialClaimRoot: credential.claimRoot,
    issuerVerificationMethodRef: credential.issuerVerificationMethodRef,
    holderBinding: credential.holderBinding,
    disclosed: {
      revealFirstName: false,
      firstNameValuePadded: new Uint8Array(64),
      firstNameOpening: new Uint8Array(32),
      revealLastName: true,
      lastNameValuePadded: claimValues.lastNameValuePadded,
      lastNameOpening: openings.lastNameOpening,
      proveAgeOverThreshold: true,
      ageThresholdYears: 18n,
      revealDocumentNumber: false,
      documentNumberValue: new Uint8Array(32),
      documentNumberOpening: new Uint8Array(32),
      revealIssuingState: false,
      issuingStateValue: new Uint8Array(32),
      issuingStateOpening: new Uint8Array(32),
    },
  };
  return { claimValues, openings, claimCommitments, schema, credential, presentation };
};

const BASE = buildFixture();
const CURRENT_DAY = 3650n + 365n * 25n;

const credentialProof = signProof({
  bodyRoot: pureCircuits.digitalPassportCredentialBodyRoot(BASE.credential),
  context: 'issuance',
  signer: ISSUER,
  createdAt: 10_001n,
  challengeHash: sha256('challenge:issuance'),
  nonceScalar: 11n,
});

const presentationRequest = {
  version: 1n,
  schema: BASE.schema,
  issuerVerificationMethodRef: BASE.credential.issuerVerificationMethodRef,
  requireFirstNameDisclosure: false,
  requireLastNameDisclosure: true,
  requireAgeOverThreshold: true,
  requestedAgeThresholdYears: 18n,
  requireDocumentNumberDisclosure: false,
  requireIssuingStateDisclosure: false,
  verifierChallengeHash: VERIFIER_CHALLENGE,
};

const presentationProof = signProof({
  bodyRoot: pureCircuits.digitalPassportPresentationBodyRoot(BASE.presentation),
  context: 'presentation',
  signer: HOLDER,
  createdAt: 10_100n,
  challengeHash: presentationRequest.verifierChallengeHash,
  nonceScalar: 17n,
});

const FEATURES = {
  supportsSelectiveDisclosure: true,
  supportsPredicateProofs: true,
  supportsVerifierScopedPseudonym: false,
  supportsSameHolderProof: false,
};

const privateParts = { claimValues: BASE.claimValues, openings: BASE.openings };

const issuanceOffer = {
  envelope: createProtocolEnvelope({
    label: 'issuance-offer',
    threadLabel: 'digital-passport-issuance',
    initialMessage: true,
    createdAt: 20_000n,
  }),
  schema: BASE.schema,
  issuerVerificationMethodRef: BASE.credential.issuerVerificationMethodRef,
  holderBindingProfile: HolderBindingProfile.explicitDid,
  features: FEATURES,
  body: { supportsExpiration: true, defaultExpirationDays: 365n, requiresHolderPublicKey: true },
};

const issuanceRequest = {
  envelope: createProtocolEnvelope({
    label: 'issuance-request',
    threadLabel: 'digital-passport-issuance',
    initialMessage: false,
    respondsToMessageId: issuanceOffer.envelope.messageId,
    createdAt: 20_010n,
  }),
  schema: BASE.schema,
  issuerVerificationMethodRef: BASE.credential.issuerVerificationMethodRef,
  holderBindingProfile: HolderBindingProfile.explicitDid,
  body: {
    holderBinding: BASE.credential.holderBinding,
    holderPublicKey: HOLDER.publicKey,
    holderChallengeHash: credentialProof.challengeHash,
    requestExpiration: true,
    requestedExpirationDays: 365n,
  },
};

const issuanceResult = {
  envelope: createProtocolEnvelope({
    label: 'issuance-result',
    threadLabel: 'digital-passport-issuance',
    initialMessage: false,
    respondsToMessageId: issuanceRequest.envelope.messageId,
    createdAt: 20_020n,
  }),
  schema: BASE.schema,
  issuerVerificationMethodRef: BASE.credential.issuerVerificationMethodRef,
  holderBindingProfile: HolderBindingProfile.explicitDid,
  body: {
    credential: BASE.credential,
    credentialProof,
    holderPublicKey: HOLDER.publicKey,
    issuanceChallengeHash: credentialProof.challengeHash,
    privateParts,
  },
};

const verificationRequest = {
  envelope: createProtocolEnvelope({
    label: 'verification-request',
    threadLabel: 'digital-passport-verification',
    initialMessage: true,
    createdAt: 21_000n,
  }),
  schema: BASE.schema,
  issuerVerificationMethodRef: BASE.credential.issuerVerificationMethodRef,
  holderBindingProfile: HolderBindingProfile.explicitDid,
  features: FEATURES,
  verifierChallengeHash: presentationRequest.verifierChallengeHash,
  body: {
    requireFirstNameDisclosure: presentationRequest.requireFirstNameDisclosure,
    requireLastNameDisclosure: presentationRequest.requireLastNameDisclosure,
    requireAgeOverThreshold: presentationRequest.requireAgeOverThreshold,
    requestedAgeThresholdYears: presentationRequest.requestedAgeThresholdYears,
    requireDocumentNumberDisclosure: presentationRequest.requireDocumentNumberDisclosure,
    requireIssuingStateDisclosure: presentationRequest.requireIssuingStateDisclosure,
  },
};

const verificationSubmission = {
  envelope: createProtocolEnvelope({
    label: 'verification-submission',
    threadLabel: 'digital-passport-verification',
    initialMessage: false,
    respondsToMessageId: verificationRequest.envelope.messageId,
    createdAt: 21_010n,
  }),
  schema: BASE.schema,
  issuerVerificationMethodRef: BASE.credential.issuerVerificationMethodRef,
  holderBindingProfile: HolderBindingProfile.explicitDid,
  challengeHash: presentationProof.challengeHash,
  body: {
    credential: BASE.credential,
    credentialProof,
    presentation: BASE.presentation,
    presentationProof,
  },
};

const verificationResult = {
  envelope: createProtocolEnvelope({
    label: 'verification-result',
    threadLabel: 'digital-passport-verification',
    initialMessage: false,
    respondsToMessageId: verificationSubmission.envelope.messageId,
    createdAt: 21_020n,
  }),
  approved: true,
  body: {
    credentialRoot: pureCircuits.digitalPassportCredentialBodyRoot(BASE.credential),
    verifiedThresholdYears: BASE.presentation.disclosed.ageThresholdYears,
  },
};

// =====================
// Outcome capture
// =====================

const outcome = (fn) => {
  try {
    fn();
    return 'ok';
  } catch (e) {
    if (e instanceof cr.CompactError && typeof e.message === 'string' && e.message.startsWith('failed assert: ')) {
      return { assertionFailed: e.message.slice('failed assert: '.length) };
    }
    throw e; // a non-assert failure is a capture bug, not a contract outcome
  }
};

// An age-predicate case: rebuilds the fixture committed to `dobDays` (the
// canonical opening) and drives the exported predicate circuit with the
// exact decompositions recorded in the case.
const predicateCase = ({ name, dobDays, currentDay, thresholdYears, predicateRequested = true,
                         dobDate, currentDate, dobOpening = BASE.openings.dateOfBirthOpening }) => {
  const fixture = dobDays === 3650n ? BASE : buildFixture(dobDays);
  const presentation = {
    ...fixture.presentation,
    disclosed: {
      ...fixture.presentation.disclosed,
      proveAgeOverThreshold: predicateRequested,
      ageThresholdYears: BigInt(thresholdYears),
    },
  };
  return {
    name,
    dobDays: Number(dobDays),
    currentDay: Number(currentDay),
    thresholdYears,
    predicateRequested,
    dobDate,
    currentDate,
    dobOpeningHex: Buffer.from(dobOpening).toString('hex'),
    outcome: outcome(() =>
      pureCircuits.assertValidDigitalPassportAgePredicate(
        fixture.credential,
        presentation,
        currentDay,
        dobDays,
        dobOpening,
        currentDate,
        dobDate,
      ),
    ),
  };
};

const asDate = (date) => ({
  year: Number(date.year),
  month: Number(date.month),
  day: Number(date.day),
  yearAdjustedQuotient4: Number(date.yearAdjustedQuotient4),
  yearAdjustedQuotient100: Number(date.yearAdjustedQuotient100),
  yearAdjustedQuotient400: Number(date.yearAdjustedQuotient400),
  marchBasedMonthDayOffset: Number(date.marchBasedMonthDayOffset),
});

const at = (year, month, day) => civilDateFromEpochDays(epochDaysFromCivil(year, month, day));
const day = (year, month, dayNumber) => epochDaysFromCivil(year, month, dayNumber);

// The upstream fixture's date-of-birth witness: epoch day 3650 (1979-12-30).
const DOB_DAY = 3650n;
const DOB_DATE = civilDateFromEpochDays(DOB_DAY);

// =====================
// 1. Civil-date helper ternary sites + assert-fail paths, driven through
//    the exported age predicate (the helper is internal on both backends).
// =====================

const agePredicateCases = [
  // --- valid dates across every branch of the decomposition ternaries ---
  // Jan (month <= 2 -> year-1; Jan/Feb leap rule; shiftedMonth = month + 9).
  predicateCase({
    name: 'civil-date-valid-epoch-day-zero-1970-01-01',
    dobDays: 0n, currentDay: 0n, thresholdYears: 0,
    dobDate: civilDateFromEpochDays(0n), currentDate: at(1970, 1, 1),
  }),
  // Feb 29 on a 400-year leap century (isLeap true branch of the Feb ternary).
  predicateCase({
    name: 'civil-date-valid-leap-century-feb-29-2000',
    dobDays: 3650n, currentDay: day(2000, 2, 29), thresholdYears: 0,
    dobDate: DOB_DATE, currentDate: at(2000, 2, 29),
  }),
  // Feb 28 on a common year (isLeap false branch).
  predicateCase({
    name: 'civil-date-valid-common-year-feb-28-2001',
    dobDays: 3650n, currentDay: day(2001, 2, 28), thresholdYears: 0,
    dobDate: DOB_DATE, currentDate: at(2001, 2, 28),
  }),
  // Feb 29 on a 400-year leap year far up the cycle.
  predicateCase({
    name: 'civil-date-valid-leap-century-feb-29-2400',
    dobDays: 3650n, currentDay: day(2400, 2, 29), thresholdYears: 0,
    dobDate: DOB_DATE, currentDate: at(2400, 2, 29),
  }),
  // March (month >= 3 -> year unchanged; shiftedMonth = month - 3).
  predicateCase({
    name: 'civil-date-valid-march-shifted-month-minus-3',
    dobDays: 3650n, currentDay: day(2024, 3, 1), thresholdYears: 0,
    dobDate: DOB_DATE, currentDate: at(2024, 3, 1),
  }),
  // December 31 (31-day month branch, shiftedMonth = 9).
  predicateCase({
    name: 'civil-date-valid-december-31',
    dobDays: 3650n, currentDay: day(2023, 12, 31), thresholdYears: 0,
    dobDate: DOB_DATE, currentDate: at(2023, 12, 31),
  }),
  // April 30 (30-day month branch).
  predicateCase({
    name: 'civil-date-valid-april-30',
    dobDays: 3650n, currentDay: day(2024, 4, 30), thresholdYears: 0,
    dobDate: DOB_DATE, currentDate: at(2024, 4, 30),
  }),
  // The default upstream fixture value set: currentDay = 3650 + 365*25.
  predicateCase({
    name: 'age-predicate-default-fixture-passes-threshold-18',
    dobDays: DOB_DAY, currentDay: CURRENT_DAY, thresholdYears: 18,
    dobDate: DOB_DATE, currentDate: civilDateFromEpochDays(CURRENT_DAY),
  }),

  // --- assert-fail paths of the civil-date helper itself ---
  predicateCase({
    name: 'civil-date-reject-year-before-epoch',
    dobDays: 3650n, currentDay: day(2024, 6, 15), thresholdYears: 0,
    dobDate: DOB_DATE,
    currentDate: { ...at(2024, 6, 15), year: 1969n, yearAdjustedQuotient4: 492n },
  }),
  predicateCase({
    name: 'civil-date-reject-month-zero',
    dobDays: 3650n, currentDay: day(2024, 6, 15), thresholdYears: 0,
    dobDate: DOB_DATE, currentDate: { ...at(2024, 6, 15), month: 0n },
  }),
  predicateCase({
    name: 'civil-date-reject-month-thirteen',
    dobDays: 3650n, currentDay: day(2024, 6, 15), thresholdYears: 0,
    dobDate: DOB_DATE, currentDate: { ...at(2024, 6, 15), month: 13n },
  }),
  predicateCase({
    name: 'civil-date-reject-day-zero',
    dobDays: 3650n, currentDay: day(2024, 6, 15), thresholdYears: 0,
    dobDate: DOB_DATE, currentDate: { ...at(2024, 6, 15), day: 0n },
  }),
  predicateCase({
    name: 'civil-date-reject-january-32',
    dobDays: 3650n, currentDay: day(2024, 1, 31), thresholdYears: 0,
    dobDate: DOB_DATE, currentDate: { ...at(2024, 1, 31), day: 32n },
  }),
  predicateCase({
    name: 'civil-date-reject-april-31',
    dobDays: 3650n, currentDay: day(2024, 4, 30), thresholdYears: 0,
    dobDate: DOB_DATE, currentDate: { ...at(2024, 4, 30), day: 31n },
  }),
  predicateCase({
    name: 'civil-date-reject-feb-30-on-leap-year',
    dobDays: 3650n, currentDay: day(2000, 2, 29), thresholdYears: 0,
    dobDate: DOB_DATE, currentDate: { ...at(2000, 2, 29), day: 30n },
  }),
  predicateCase({
    name: 'civil-date-reject-feb-29-on-common-year',
    dobDays: 3650n, currentDay: day(2100, 2, 28), thresholdYears: 0,
    dobDate: DOB_DATE, currentDate: { ...at(2100, 2, 28), day: 29n },
  }),
  predicateCase({
    name: 'civil-date-reject-feb-29-on-common-year-within-cycle',
    dobDays: 3650n, currentDay: day(2023, 2, 28), thresholdYears: 0,
    dobDate: DOB_DATE, currentDate: { ...at(2023, 2, 28), day: 29n },
  }),
  predicateCase({
    name: 'civil-date-reject-quotient4-out-of-range',
    dobDays: 3650n, currentDay: day(2024, 6, 15), thresholdYears: 0,
    dobDate: DOB_DATE,
    currentDate: { ...at(2024, 6, 15), yearAdjustedQuotient4: at(2024, 6, 15).yearAdjustedQuotient4 + 1n },
  }),
  predicateCase({
    name: 'civil-date-reject-quotient100-out-of-range',
    dobDays: 3650n, currentDay: day(2024, 6, 15), thresholdYears: 0,
    dobDate: DOB_DATE,
    currentDate: { ...at(2024, 6, 15), yearAdjustedQuotient100: at(2024, 6, 15).yearAdjustedQuotient100 - 1n },
  }),
  predicateCase({
    name: 'civil-date-reject-month-day-offset-out-of-range',
    dobDays: 3650n, currentDay: day(2024, 6, 15), thresholdYears: 0,
    dobDate: DOB_DATE,
    currentDate: { ...at(2024, 6, 15), marchBasedMonthDayOffset: at(2024, 6, 15).marchBasedMonthDayOffset + 1n },
  }),
  predicateCase({
    name: 'civil-date-reject-day-number-mismatch-current-date',
    dobDays: 3650n, currentDay: day(2024, 6, 15), thresholdYears: 0,
    dobDate: DOB_DATE, currentDate: civilDateFromEpochDays(day(2024, 6, 15) + 1n),
  }),
  predicateCase({
    name: 'civil-date-reject-day-number-mismatch-dob-date',
    dobDays: 3650n, currentDay: day(2024, 6, 15), thresholdYears: 0,
    dobDate: civilDateFromEpochDays(DOB_DAY + 1n), currentDate: at(2024, 6, 15),
  }),

  // --- age-predicate behavior: calendar semantics + remaining ternary ---
  // beforeBirthdayThisYear ? 1 : 0 — false branch: birthday is today.
  predicateCase({
    name: 'age-predicate-exact-threshold-on-birthday-passes',
    dobDays: day(1990, 6, 15), currentDay: day(2008, 6, 15), thresholdYears: 18,
    dobDate: at(1990, 6, 15), currentDate: at(2008, 6, 15),
  }),
  // beforeBirthdayThisYear — true branch: the day before the birthday.
  predicateCase({
    name: 'age-predicate-day-before-birthday-rejected',
    dobDays: day(1990, 6, 15), currentDay: day(2008, 6, 14), thresholdYears: 18,
    dobDate: at(1990, 6, 15), currentDate: at(2008, 6, 14),
  }),
  // Upstream's leap-day traps: flat 365-day count satisfied but calendar age not.
  predicateCase({
    name: 'age-predicate-flat-365-leap-trap-rejected',
    dobDays: day(2000, 3, 1), currentDay: day(2004, 2, 29), thresholdYears: 4,
    dobDate: at(2000, 3, 1), currentDate: at(2004, 2, 29),
  }),
  predicateCase({
    name: 'age-predicate-calendar-anniversary-passes',
    dobDays: day(2000, 3, 1), currentDay: day(2004, 3, 1), thresholdYears: 4,
    dobDate: at(2000, 3, 1), currentDate: at(2004, 3, 1),
  }),
  predicateCase({
    name: 'age-predicate-leap-day-holder-day-before-anniversary-rejected',
    dobDays: day(2000, 2, 29), currentDay: day(2004, 2, 28), thresholdYears: 4,
    dobDate: at(2000, 2, 29), currentDate: at(2004, 2, 28),
  }),
  predicateCase({
    name: 'age-predicate-leap-day-holder-on-anniversary-passes',
    dobDays: day(2000, 2, 29), currentDay: day(2004, 2, 29), thresholdYears: 4,
    dobDate: at(2000, 2, 29), currentDate: at(2004, 2, 29),
  }),
  predicateCase({
    name: 'age-predicate-threshold-too-strict-rejected',
    dobDays: 3650n, currentDay: CURRENT_DAY, thresholdYears: 30,
    dobDate: DOB_DATE, currentDate: civilDateFromEpochDays(CURRENT_DAY),
  }),
  predicateCase({
    name: 'age-predicate-predicate-not-requested-rejected',
    dobDays: 3650n, currentDay: CURRENT_DAY, thresholdYears: 18, predicateRequested: false,
    dobDate: DOB_DATE, currentDate: civilDateFromEpochDays(CURRENT_DAY),
  }),
  predicateCase({
    name: 'age-predicate-wrong-opening-rejected',
    dobDays: 3650n, currentDay: CURRENT_DAY, thresholdYears: 18,
    dobDate: DOB_DATE, currentDate: civilDateFromEpochDays(CURRENT_DAY),
    dobOpening: new Uint8Array(32).fill(1),
  }),
  predicateCase({
    name: 'age-predicate-current-before-birth-rejected',
    dobDays: DOB_DAY, currentDay: day(1979, 12, 29), thresholdYears: 18,
    dobDate: DOB_DATE, currentDate: at(1979, 12, 29),
  }),
];

// =====================
// 2. Derived-value parity: every root/commitment/challenge the Rust side
//    must re-derive byte-identically from the same inputs.
// =====================

const hex = (u8) => Buffer.from(u8).toString('hex');
// 256-bit scalar, little-endian bytes (matches Rust Fr::from_le_bytes).
const frLeHex = (v) => Buffer.from(Buffer.from(v.toString(16).padStart(64, '0'), 'hex')).reverse().toString('hex');
const point = (p) => ({ xHex: frLeHex(p.x), yHex: frLeHex(p.y) });
const proofJson = (p) => ({
  createdAt: Number(p.createdAt),
  challengeHashHex: hex(p.challengeHash),
  publicKey: point(p.publicKey),
  r: point(p.signature.r),
  sHex: frLeHex(p.signature.s),
});

// The challenge is independent of the response scalar s (it hashes r), so a
// proof with s = 0 reproduces it — exactly how signProof derives it.
const challengeOf = (p) => ({ ...p, signature: { ...p.signature, s: 0n } });

const derivedValues = {
  firstNameCommitmentHex: hex(BASE.claimCommitments.firstNameCommitment),
  lastNameCommitmentHex: hex(BASE.claimCommitments.lastNameCommitment),
  dateOfBirthCommitmentHex: hex(BASE.claimCommitments.dateOfBirthCommitment),
  documentNumberCommitmentHex: hex(BASE.claimCommitments.documentNumberCommitment),
  issuingStateCommitmentHex: hex(BASE.claimCommitments.issuingStateCommitment),
  claimRootHex: hex(BASE.credential.claimRoot),
  credentialBodyRootHex: hex(pureCircuits.digitalPassportCredentialBodyRoot(BASE.credential)),
  presentationBodyRootHex: hex(pureCircuits.digitalPassportPresentationBodyRoot(BASE.presentation)),
  presentationRequestBodyRootHex: hex(pureCircuits.digitalPassportPresentationRequestBodyRoot(presentationRequest)),
  issuanceProofChallengeHex: frLeHex(
    pureCircuits.issuanceProofChallenge(
      pureCircuits.digitalPassportCredentialBodyRoot(BASE.credential),
      challengeOf(credentialProof),
    ),
  ),
  presentationProofChallengeHex: frLeHex(
    pureCircuits.presentationProofChallenge(
      pureCircuits.digitalPassportPresentationBodyRoot(BASE.presentation),
      challengeOf(presentationProof),
    ),
  ),
};

// =====================
// 3. Protocol round-trip: issuance -> presentation -> verification, plus
//    the two tamper rejections upstream's protocol test pins.
// =====================

const normalizedRequest = pureCircuits.digitalPassportPresentationRequestFromProtocol(verificationRequest);

const protocolCases = [
  { name: 'issuance-offer-valid', outcome: outcome(() => pureCircuits.assertValidDigitalPassportIssuanceOffer(issuanceOffer)) },
  { name: 'issuance-request-valid', outcome: outcome(() => pureCircuits.assertValidDigitalPassportIssuanceRequest(issuanceRequest)) },
  { name: 'issuance-request-matches-offer', outcome: outcome(() => pureCircuits.assertDigitalPassportIssuanceRequestMatchesOffer(issuanceOffer, issuanceRequest)) },
  { name: 'issuance-result-valid', outcome: outcome(() => pureCircuits.assertValidDigitalPassportIssuanceResult(issuanceResult)) },
  { name: 'issuance-result-matches-request', outcome: outcome(() => pureCircuits.assertDigitalPassportIssuanceResultMatchesRequest(issuanceRequest, issuanceResult)) },
  { name: 'verification-request-valid', outcome: outcome(() => pureCircuits.assertValidDigitalPassportVerificationRequestMessage(verificationRequest)) },
  { name: 'verification-submission-valid', outcome: outcome(() => pureCircuits.assertValidDigitalPassportVerificationSubmissionMessage(verificationSubmission)) },
  { name: 'verification-submission-matches-request', outcome: outcome(() => pureCircuits.assertDigitalPassportVerificationSubmissionMatchesRequest(verificationRequest, verificationSubmission)) },
  { name: 'verification-result-valid', outcome: outcome(() => pureCircuits.assertValidDigitalPassportVerificationResultMessage(verificationResult)) },
  { name: 'verification-result-matches-submission', outcome: outcome(() => pureCircuits.assertDigitalPassportVerificationResultMatchesSubmission(verificationSubmission, verificationResult)) },
  {
    name: 'issuance-result-challenge-tampered-rejected',
    outcome: outcome(() =>
      pureCircuits.assertDigitalPassportIssuanceResultMatchesRequest(issuanceRequest, {
        ...issuanceResult,
        body: { ...issuanceResult.body, issuanceChallengeHash: new Uint8Array(32).fill(3) },
      }),
    ),
  },
  {
    name: 'verification-challenge-tampered-rejected',
    outcome: outcome(() =>
      pureCircuits.assertDigitalPassportVerificationSubmissionMatchesRequest(
        { ...verificationRequest, verifierChallengeHash: new Uint8Array(32).fill(9) },
        verificationSubmission,
      ),
    ),
  },
];

// =====================
// Fixture inputs the Rust side replays verbatim (everything not derivable
// through the circuits themselves).
// =====================

const fixtureValues = {
  issuer: {
    secretKeyHex: frLeHex(ISSUER.secretKey),
    publicKey: point(ISSUER.publicKey),
    didContractAddressHex: hex(ISSUER.verificationMethodRef.didContractAddress.bytes),
    methodIdHex: hex(ISSUER.verificationMethodRef.methodId),
  },
  holder: {
    secretKeyHex: frLeHex(HOLDER.secretKey),
    publicKey: point(HOLDER.publicKey),
    didContractAddressHex: hex(HOLDER.verificationMethodRef.didContractAddress.bytes),
    methodIdHex: hex(HOLDER.verificationMethodRef.methodId),
  },
  issuerNonce: 11,
  holderNonce: 17,
  credentialProof: proofJson(credentialProof),
  presentationProof: proofJson(presentationProof),
  verifierChallengeHex: hex(VERIFIER_CHALLENGE),
  issuanceChallengeHex: hex(sha256('challenge:issuance')),
  schema: {
    packageIdHex: hex(BASE.schema.packageId),
    schemaIdHex: hex(BASE.schema.schemaId),
  },
  claimValues: {
    firstNameValuePaddedHex: hex(BASE.claimValues.firstNameValuePadded),
    lastNameValuePaddedHex: hex(BASE.claimValues.lastNameValuePadded),
    documentNumberValueHex: hex(BASE.claimValues.documentNumberValue),
    issuingStateValueHex: hex(BASE.claimValues.issuingStateValue),
  },
  openings: {
    firstNameOpeningHex: hex(BASE.openings.firstNameOpening),
    lastNameOpeningHex: hex(BASE.openings.lastNameOpening),
    dateOfBirthOpeningHex: hex(BASE.openings.dateOfBirthOpening),
    documentNumberOpeningHex: hex(BASE.openings.documentNumberOpening),
    issuingStateOpeningHex: hex(BASE.openings.issuingStateOpening),
  },
  credential: {
    issuedAt: 10_000,
    hasExpiration: true,
    expiresAt: 20_000,
  },
  presentation: {
    revealFirstName: BASE.presentation.disclosed.revealFirstName,
    revealLastName: BASE.presentation.disclosed.revealLastName,
    revealDocumentNumber: BASE.presentation.disclosed.revealDocumentNumber,
    revealIssuingState: BASE.presentation.disclosed.revealIssuingState,
  },
  presentationRequest: {
    requireFirstNameDisclosure: presentationRequest.requireFirstNameDisclosure,
    requireLastNameDisclosure: presentationRequest.requireLastNameDisclosure,
    requireAgeOverThreshold: presentationRequest.requireAgeOverThreshold,
    requestedAgeThresholdYears: Number(presentationRequest.requestedAgeThresholdYears),
    requireDocumentNumberDisclosure: presentationRequest.requireDocumentNumberDisclosure,
    requireIssuingStateDisclosure: presentationRequest.requireIssuingStateDisclosure,
  },
  noProtocolResponseReferenceHex: hex(pureCircuits.noProtocolResponseReference()),
  normalizedPresentationRequestRootHex: hex(
    pureCircuits.digitalPassportPresentationRequestBodyRoot(normalizedRequest),
  ),
};

const envelopeJson = (e) => ({
  version: Number(e.version),
  messageIdHex: hex(e.messageId),
  threadIdHex: hex(e.threadId),
  initialMessage: e.initialMessage,
  respondsToMessageIdHex: hex(e.respondsToMessageId),
  createdAt: Number(e.createdAt),
  hasExpiresAt: e.hasExpiresAt,
  expiresAt: Number(e.expiresAt),
});

const protocolMessages = {
  features: FEATURES,
  issuanceOffer: {
    envelope: envelopeJson(issuanceOffer.envelope),
    supportsExpiration: issuanceOffer.body.supportsExpiration,
    defaultExpirationDays: Number(issuanceOffer.body.defaultExpirationDays),
    requiresHolderPublicKey: issuanceOffer.body.requiresHolderPublicKey,
  },
  issuanceRequest: {
    envelope: envelopeJson(issuanceRequest.envelope),
    holderChallengeHashHex: hex(issuanceRequest.body.holderChallengeHash),
    requestExpiration: issuanceRequest.body.requestExpiration,
    requestedExpirationDays: Number(issuanceRequest.body.requestedExpirationDays),
  },
  issuanceResult: {
    envelope: envelopeJson(issuanceResult.envelope),
    issuanceChallengeHashHex: hex(issuanceResult.body.issuanceChallengeHash),
  },
  verificationRequest: {
    envelope: envelopeJson(verificationRequest.envelope),
    verifierChallengeHashHex: hex(verificationRequest.verifierChallengeHash),
    requireFirstNameDisclosure: verificationRequest.body.requireFirstNameDisclosure,
    requireLastNameDisclosure: verificationRequest.body.requireLastNameDisclosure,
    requireAgeOverThreshold: verificationRequest.body.requireAgeOverThreshold,
    requestedAgeThresholdYears: Number(verificationRequest.body.requestedAgeThresholdYears),
    requireDocumentNumberDisclosure: verificationRequest.body.requireDocumentNumberDisclosure,
    requireIssuingStateDisclosure: verificationRequest.body.requireIssuingStateDisclosure,
  },
  verificationSubmission: {
    envelope: envelopeJson(verificationSubmission.envelope),
    challengeHashHex: hex(verificationSubmission.challengeHash),
  },
  verificationResult: {
    envelope: envelopeJson(verificationResult.envelope),
    approved: verificationResult.approved,
    verifiedThresholdYears: Number(verificationResult.body.verifiedThresholdYears),
  },
};

const fixture = {
  fixtureValues,
  derivedValues,
  agePredicateCases: agePredicateCases.map((c) => ({
    ...c,
    dobDate: asDate(c.dobDate),
    currentDate: asDate(c.currentDate),
  })),
  protocolMessages,
  protocolCases,
};

process.stdout.write(JSON.stringify(fixture, null, 2) + '\n');
