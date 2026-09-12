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

// Capture the TypeScript reference behaviour for the vendored dogfood contract
// `examples/dogfood/digital-passport-credential/src/digital-passport-credential.compact`.
//
// The contract is a pure helper library (no ledger, no constructor, no impure
// circuits), so instead of a `stateHex` byte snapshot this capture records the
// observable *outcome* of the exported pure circuits: either `{ ok: true }`
// (the circuit returned without asserting) or `{ ok: false, error: <exact
// message> }` (an `assert` fired). The Rust parity test written by
// `add-digital-passport-dogfood-fixture` replays the same inputs and compares
// outcomes.
//
// Two families of steps are captured:
//
//   1. Civil-date helpers. `assertCivilDateMatchesEpochDays` is a non-exported
//      helper, reachable only through the exported
//      `assertValidDigitalPassportAgePredicate` (which calls it twice). The
//      scenario table below drives every conditional-expression site of both
//      helpers (the `yearAdjusted` / `isLeap` / `shiftedMonth` ternaries, the
//      31-/30-day and February leap/non-leap branches, `beforeBirthdayThisYear`
//      and the `? 1 : 0` age correction) and every `assert`-fail path.
//
//   2. One issuance/presentation/verification round-trip. Honest credential and
//      presentation proofs are produced in-script with the contract's own
//      challenge-derivation circuits (Schnorr over the embedded curve), then
//      the exported message/credential/presentation validators are driven
//      through the full offer → request → result and request → submission →
//      result sequences, plus the derived body roots.
//
// Usage:
//   compactc --target ts --skip-zk \
//     examples/dogfood/digital-passport-credential/src/digital-passport-credential.compact \
//     /tmp/digital-passport-credential-ts-driver/
//   echo '{"type":"module"}' > /tmp/digital-passport-credential-ts-driver/contract/package.json
//   ln -sfn "$PWD/node_modules" /tmp/digital-passport-credential-ts-driver/contract/node_modules
//   node tests-e2e-rust/fixtures/capture-digital-passport-credential.mjs \
//     > tests-e2e-rust/fixtures/digital-passport-credential-ts-state.json

import { pureCircuits } from '/tmp/digital-passport-credential-ts-driver/contract/index.js';
import * as cr from '@midnight-ntwrk/compact-runtime';

// ---------------------------------------------------------------------------
// Small deterministic helpers
// ---------------------------------------------------------------------------

// The order of the embedded (Jubjub) group; scalars in `ecMul`/`ecMulGenerator`
// must lie in [0, order).
const EMBEDDED_ORDER =
  6554484396890773809930967563523245729705921265872317281365359162392183254199n;

function pad32(s) {
  const out = new Uint8Array(32);
  out.set(Buffer.from(s, 'utf8'), 0);
  return out;
}
function bytes32(fill) {
  return new Uint8Array(32).fill(fill);
}
function hex(u8) {
  return Buffer.from(u8).toString('hex');
}

// Proleptic-Gregorian civil-date decomposition, mirroring the witness the
// contract verifies by exact reconstruction (`days_from_civil`, day 0 =
// 1970-01-01).
function civilDate(year, month, day) {
  const yearAdjusted = month <= 2 ? year - 1 : year;
  const shiftedMonth = month >= 3 ? month - 3 : month + 9;
  return {
    year: BigInt(year),
    month: BigInt(month),
    day: BigInt(day),
    yearAdjustedQuotient4: BigInt(Math.floor(yearAdjusted / 4)),
    yearAdjustedQuotient100: BigInt(Math.floor(yearAdjusted / 100)),
    yearAdjustedQuotient400: BigInt(Math.floor(yearAdjusted / 400)),
    marchBasedMonthDayOffset: BigInt(Math.floor((153 * shiftedMonth + 2) / 5)),
  };
}
function daysFromCivil(year, month, day) {
  year -= month <= 2 ? 1 : 0;
  const era = Math.floor(year / 400);
  const yoe = year - era * 400;
  const doy = Math.floor((153 * (month + (month > 2 ? -3 : 9)) + 2) / 5) + day - 1;
  const doe = yoe * 365 + Math.floor(yoe / 4) - Math.floor(yoe / 100) + doy;
  return BigInt(era * 146097 + doe - 719468);
}

function normDate(d) {
  return {
    year: d.year.toString(),
    month: d.month.toString(),
    day: d.day.toString(),
    yearAdjustedQuotient4: d.yearAdjustedQuotient4.toString(),
    yearAdjustedQuotient100: d.yearAdjustedQuotient100.toString(),
    yearAdjustedQuotient400: d.yearAdjustedQuotient400.toString(),
    marchBasedMonthDayOffset: d.marchBasedMonthDayOffset.toString(),
  };
}

// ---------------------------------------------------------------------------
// Civil-date helper scenarios
// ---------------------------------------------------------------------------

const DOB_OPENING = bytes32(0x0a);

// Minimal well-typed credential/presentation. Only
// `presentation.disclosed.proveAgeOverThreshold`, `ageThresholdYears`, and the
// credential's `dateOfBirthCommitment` are read by the age predicate.
function makeCredential(dobDays, dobOpening) {
  return {
    version: 1n,
    schema: {
      packageId: pad32('midnight:vc:digital-passport'),
      schemaId: pad32('digital-passport:v1'),
      majorVersion: 1n,
      minorVersion: 0n,
    },
    issuerVerificationMethodRef: {
      didContractAddress: { bytes: bytes32(0x11) },
      methodId: bytes32(0x22),
    },
    holderBinding: {
      holderVerificationMethodRef: {
        didContractAddress: { bytes: bytes32(0x33) },
        methodId: bytes32(0x44),
      },
    },
    statusBinding: {},
    issuedAt: 0n,
    hasExpiration: false,
    expiresAt: 0n,
    claims: {},
    claimCommitments: {
      firstNameCommitment: bytes32(0x01),
      lastNameCommitment: bytes32(0x02),
      dateOfBirthCommitment: pureCircuits.dateOfBirthCommitment(dobDays, dobOpening),
      documentNumberCommitment: pureCircuits.documentNumberNullCommitment(),
      issuingStateCommitment: bytes32(0x05),
    },
    claimRoot: bytes32(0x09),
  };
}
function makePresentation(proveAgeOverThreshold, ageThresholdYears) {
  return {
    version: 1n,
    schema: {
      packageId: pad32('midnight:vc:digital-passport'),
      schemaId: pad32('digital-passport:v1'),
      majorVersion: 1n,
      minorVersion: 0n,
    },
    credentialClaimRoot: bytes32(0x09),
    issuerVerificationMethodRef: {
      didContractAddress: { bytes: bytes32(0x11) },
      methodId: bytes32(0x22),
    },
    holderBinding: {
      holderVerificationMethodRef: {
        didContractAddress: { bytes: bytes32(0x33) },
        methodId: bytes32(0x44),
      },
    },
    disclosed: {
      revealFirstName: false,
      firstNameValuePadded: new Uint8Array(64),
      firstNameOpening: bytes32(0),
      revealLastName: false,
      lastNameValuePadded: new Uint8Array(64),
      lastNameOpening: bytes32(0),
      proveAgeOverThreshold,
      ageThresholdYears,
      revealDocumentNumber: false,
      documentNumberValue: bytes32(0),
      documentNumberOpening: bytes32(0),
      revealIssuingState: false,
      issuingStateValue: bytes32(0),
      issuingStateOpening: bytes32(0),
    },
  };
}

// Each scenario names the code site it exercises, builds the seven predicate
// arguments, and exposes a normalized, replayable input record.
function defineScenario(name, site, opts) {
  const dob = opts.dob ?? [1990, 5, 15];
  const current = opts.current ?? [2024, 6, 20];
  const threshold = opts.threshold ?? 18n;
  const proveAgeOverThreshold = opts.proveAgeOverThreshold ?? true;

  const dateOfBirthDays =
    opts.dateOfBirthDays ?? daysFromCivil(dob[0], dob[1], dob[2]);
  const currentDay = opts.currentDay ?? daysFromCivil(current[0], current[1], current[2]);
  const dateOfBirthDate = opts.dateOfBirthDate ?? civilDate(dob[0], dob[1], dob[2]);
  const currentDate = opts.currentDate ?? civilDate(current[0], current[1], current[2]);
  const commitmentDays = opts.commitmentDays ?? dateOfBirthDays;

  const credential = makeCredential(commitmentDays, DOB_OPENING);
  const presentation = makePresentation(proveAgeOverThreshold, threshold);

  return {
    name,
    site,
    inputs: {
      proveAgeOverThreshold,
      ageThresholdYears: threshold.toString(),
      currentDay: currentDay.toString(),
      dateOfBirthDays: dateOfBirthDays.toString(),
      dateOfBirthOpening: hex(DOB_OPENING),
      dateOfBirthCommitment: hex(credential.claimCommitments.dateOfBirthCommitment),
      currentDate: normDate(currentDate),
      dateOfBirthDate: normDate(dateOfBirthDate),
    },
    run() {
      pureCircuits.assertValidDigitalPassportAgePredicate(
        credential,
        presentation,
        currentDay,
        dateOfBirthDays,
        DOB_OPENING,
        currentDate,
        dateOfBirthDate,
      );
    },
  };
}

// `mutate` rewrites specific decomposition fields of a valid date so a single
// downstream `assert` fires; everything upstream of it still passes.
function mutate(base, patch) {
  return { ...base, ...patch };
}

const civilDateScenarios = [
  // --- accepting paths, one per conditional branch ---
  defineScenario(
    'accept_birthday_passed',
    'month >= 3 (yearAdjusted, shiftedMonth); 30-day current month; 31-day birth month; beforeBirthday=false; ? 1 : 0 -> 0',
    {},
  ),
  defineScenario(
    'accept_birthday_pending',
    'month >= 3; beforeBirthday=true; ? 1 : 0 -> 1',
    { current: [2024, 3, 10] },
  ),
  defineScenario(
    'accept_february_leap_dob',
    'month <= 2 (yearAdjusted, shiftedMonth); isLeap=true; February day<=29 branch',
    { dob: [2000, 2, 29], current: [2024, 2, 29], threshold: 20n },
  ),
  defineScenario(
    'accept_february_nonleap',
    'month <= 2; isLeap=false; February day<=28 branch',
    { dob: [2001, 2, 28], current: [2024, 2, 28], threshold: 20n },
  ),
  defineScenario(
    'accept_30day_month',
    '30-day month branch (current and birth month both April)',
    { dob: [1990, 4, 15], current: [2024, 4, 30] },
  ),

  // --- assert-fail paths of assertValidDigitalPassportAgePredicate ---
  defineScenario(
    'reject_prove_age_disabled',
    "assert(presentation.disclosed.proveAgeOverThreshold) — 'Presentation must request the age-over-threshold predicate'",
    { proveAgeOverThreshold: false },
  ),
  defineScenario(
    'reject_dob_commitment_mismatch',
    "assert(dateOfBirthCommitment == credential commitment) — 'Date-of-birth witness does not match credential commitment'",
    { commitmentDays: 9999n },
  ),
  defineScenario(
    'reject_current_before_dob',
    "assert(currentDay >= dateOfBirthDays) — 'Current day must not precede the date-of-birth witness'",
    { currentDay: daysFromCivil(1980, 1, 1) },
  ),

  // --- assert-fail paths of assertCivilDateMatchesEpochDays, via currentDate ---
  defineScenario(
    'reject_year_before_1970',
    "assert(date.year >= 1970) — 'Civil date year must be at least 1970'",
    { currentDate: civilDate(1969, 6, 20) },
  ),
  defineScenario(
    'reject_month_zero',
    "assert(date.month >= 1) — 'Civil date month must be at least 1'",
    { currentDate: civilDate(2024, 0, 20) },
  ),
  defineScenario(
    'reject_month_thirteen',
    "assert(date.month <= 12) — 'Civil date month must be at most 12'",
    { currentDate: civilDate(2024, 13, 20) },
  ),
  defineScenario(
    'reject_day_zero',
    "assert(date.day >= 1) — 'Civil date day must be at least 1'",
    { currentDate: civilDate(2024, 6, 0) },
  ),
  defineScenario(
    'reject_quotient4_invalid',
    "assert(quotient4 range) — 'Civil date quotient for 4 is invalid'",
    { currentDate: mutate(civilDate(2024, 6, 20), { yearAdjustedQuotient4: civilDate(2024, 6, 20).yearAdjustedQuotient4 + 1n }) },
  ),
  defineScenario(
    'reject_quotient100_invalid',
    "assert(quotient100 range) — 'Civil date quotient for 100 is invalid'",
    { currentDate: mutate(civilDate(2024, 6, 20), { yearAdjustedQuotient100: civilDate(2024, 6, 20).yearAdjustedQuotient100 + 1n }) },
  ),
  defineScenario(
    'reject_quotient400_invalid',
    "assert(quotient400 range) — 'Civil date quotient for 400 is invalid'",
    { currentDate: mutate(civilDate(2024, 6, 20), { yearAdjustedQuotient400: civilDate(2024, 6, 20).yearAdjustedQuotient400 + 1n }) },
  ),
  defineScenario(
    'reject_month_day_offset_invalid',
    "assert(marchBasedMonthDayOffset range) — 'Civil date month day offset is invalid'",
    { currentDate: mutate(civilDate(2024, 6, 20), { marchBasedMonthDayOffset: civilDate(2024, 6, 20).marchBasedMonthDayOffset + 1n }) },
  ),
  defineScenario(
    'reject_31day_month_overflow',
    "if (31-day month) assert(day <= 31) — 'Civil date day exceeds the 31-day month length'",
    { currentDate: mutate(civilDate(2024, 1, 1), { day: 32n }) },
  ),
  defineScenario(
    'reject_february_nonleap_29',
    "if (month == 2) isLeap=false -> assert(day <= 28) — 'Civil date day exceeds the February month length'",
    { currentDate: mutate(civilDate(2023, 2, 1), { day: 29n }) },
  ),
  defineScenario(
    'reject_february_leap_30',
    "if (month == 2) isLeap=true -> assert(day <= 29) — 'Civil date day exceeds the February month length'",
    { currentDate: mutate(civilDate(2024, 2, 1), { day: 30n }) },
  ),
  defineScenario(
    'reject_30day_month_overflow',
    "else assert(day <= 30) — 'Civil date day exceeds the 30-day month length'",
    { currentDate: mutate(civilDate(2023, 4, 1), { day: 31n }) },
  ),
  defineScenario(
    'reject_epoch_days_mismatch',
    "assert(epochDays == days_from_civil(...)) — 'Civil date does not match the day number'",
    { currentDay: daysFromCivil(2024, 6, 20) + 1n },
  ),
  defineScenario(
    'reject_age_below_threshold',
    "assert(ageInYears >= presentation.disclosed.ageThresholdYears) — 'Age predicate does not satisfy the requested threshold'",
    { threshold: 40n },
  ),
];

// ---------------------------------------------------------------------------
// Issuance / presentation / verification round-trip
// ---------------------------------------------------------------------------

const ROUND_TRIP_DOMAIN = {
  packageId: pad32('midnight:vc:digital-passport'),
  schemaId: pad32('digital-passport:v1'),
  majorVersion: 1n,
  minorVersion: 0n,
};
const ISSUER_METHOD = {
  didContractAddress: { bytes: bytes32(0x11) },
  methodId: bytes32(0x22),
};
const HOLDER_METHOD = {
  didContractAddress: { bytes: bytes32(0x33) },
  methodId: bytes32(0x44),
};
const HOLDER_BINDING = { holderVerificationMethodRef: HOLDER_METHOD };
const NO_RESPONSE = pad32('midnight:vc:protocol:none');

const ISSUER_SK = 0x1234n;
const ISSUER_PK = cr.ecMulGenerator(ISSUER_SK);
const NONCE = 0x5678n;
const NONCE_POINT = cr.ecMulGenerator(NONCE);

// Produce an honest Schnorr proof over `bodyRoot`. The challenge is derived by
// the contract itself (the same pure circuit the verifier uses); the proof's
// `createdAt` is varied until that challenge falls in the embedded scalar
// range, then `s = nonce + challenge * sk (mod order)` satisfies
// `s*G == r + challenge*pk`.
function signProof(bodyRoot, challengeFn, signerRef, challengeBytes, createdAt) {
  let challenge = null;
  let proof = null;
  for (let i = 0n; ; i += 1n) {
    proof = {
      signerVerificationMethodRef: signerRef,
      createdAt: createdAt + i,
      challengeHash: challengeBytes,
      publicKey: ISSUER_PK,
      signature: { r: NONCE_POINT, s: 0n },
    };
    challenge = challengeFn(bodyRoot, proof);
    if (challenge < EMBEDDED_ORDER) break;
  }
  proof.signature.s = (NONCE + challenge * ISSUER_SK) % EMBEDDED_ORDER;
  return proof;
}

function envelope(overrides) {
  return {
    version: 1n,
    messageId: bytes32(0x01),
    threadId: bytes32(0x02),
    initialMessage: true,
    respondsToMessageId: NO_RESPONSE,
    createdAt: 0n,
    hasExpiresAt: false,
    expiresAt: 0n,
    ...overrides,
  };
}

function buildRoundTrip() {
  const firstNameValuePadded = new Uint8Array(64).fill(0x01);
  const lastNameValuePadded = new Uint8Array(64).fill(0x02);
  const dateOfBirthDays = 7400n;
  const dateOfBirthOpening = bytes32(0x0a);
  const issuingStateValue = bytes32(0x05);
  const issuingStateOpening = bytes32(0x0b);

  const claimCommitments = {
    firstNameCommitment: pureCircuits.firstNameCommitment(firstNameValuePadded, bytes32(0x03)),
    lastNameCommitment: pureCircuits.lastNameCommitment(lastNameValuePadded, bytes32(0x04)),
    dateOfBirthCommitment: pureCircuits.dateOfBirthCommitment(dateOfBirthDays, dateOfBirthOpening),
    documentNumberCommitment: pureCircuits.documentNumberNullCommitment(),
    issuingStateCommitment: pureCircuits.issuingStateCommitment(issuingStateValue, issuingStateOpening),
  };
  const claimRoot = pureCircuits.digitalPassportClaimRoot(claimCommitments);

  const credential = {
    version: 1n,
    schema: ROUND_TRIP_DOMAIN,
    issuerVerificationMethodRef: ISSUER_METHOD,
    holderBinding: HOLDER_BINDING,
    statusBinding: {},
    issuedAt: 1000n,
    hasExpiration: false,
    expiresAt: 0n,
    claims: {},
    claimCommitments,
    claimRoot,
  };

  const proofChallengeBytes = bytes32(0x77);
  const credentialBodyRoot = pureCircuits.digitalPassportCredentialBodyRoot(credential);
  const credentialProof = signProof(
    credentialBodyRoot,
    (br, p) => pureCircuits.issuanceProofChallenge(br, p),
    ISSUER_METHOD,
    proofChallengeBytes,
    0n,
  );

  const presentation = {
    version: 1n,
    schema: ROUND_TRIP_DOMAIN,
    credentialClaimRoot: claimRoot,
    issuerVerificationMethodRef: ISSUER_METHOD,
    holderBinding: HOLDER_BINDING,
    disclosed: {
      revealFirstName: false,
      firstNameValuePadded,
      firstNameOpening: bytes32(0x03),
      revealLastName: false,
      lastNameValuePadded,
      lastNameOpening: bytes32(0x04),
      proveAgeOverThreshold: false,
      ageThresholdYears: 0n,
      revealDocumentNumber: false,
      documentNumberValue: bytes32(0),
      documentNumberOpening: bytes32(0),
      revealIssuingState: false,
      issuingStateValue,
      issuingStateOpening,
    },
  };
  const presentationChallengeBytes = bytes32(0x88);
  const presentationBodyRoot = pureCircuits.digitalPassportPresentationBodyRoot(presentation);
  const presentationProof = signProof(
    presentationBodyRoot,
    (br, p) => pureCircuits.presentationProofChallenge(br, p),
    HOLDER_METHOD,
    presentationChallengeBytes,
    0n,
  );

  const threadId = bytes32(0x20);
  const offerMessageId = bytes32(0x10);

  const offer = {
    envelope: envelope({ messageId: offerMessageId, threadId, initialMessage: true, respondsToMessageId: NO_RESPONSE, createdAt: 1n }),
    schema: ROUND_TRIP_DOMAIN,
    issuerVerificationMethodRef: ISSUER_METHOD,
    holderBindingProfile: 0,
    features: {
      supportsSelectiveDisclosure: true,
      supportsPredicateProofs: true,
      supportsVerifierScopedPseudonym: false,
      supportsSameHolderProof: false,
    },
    body: { supportsExpiration: false, defaultExpirationDays: 0n, requiresHolderPublicKey: false },
  };
  const requestMessageId = bytes32(0x11);
  const issuanceRequest = {
    envelope: envelope({ messageId: requestMessageId, threadId, initialMessage: false, respondsToMessageId: offerMessageId, createdAt: 2n }),
    schema: ROUND_TRIP_DOMAIN,
    issuerVerificationMethodRef: ISSUER_METHOD,
    holderBindingProfile: 0,
    body: {
      holderBinding: HOLDER_BINDING,
      holderPublicKey: ISSUER_PK,
      holderChallengeHash: proofChallengeBytes,
      requestExpiration: false,
      requestedExpirationDays: 0n,
    },
  };
  const resultMessageId = bytes32(0x12);
  const issuanceResult = {
    envelope: envelope({ messageId: resultMessageId, threadId, initialMessage: false, respondsToMessageId: requestMessageId, createdAt: 3n }),
    schema: ROUND_TRIP_DOMAIN,
    issuerVerificationMethodRef: ISSUER_METHOD,
    holderBindingProfile: 0,
    body: {
      credential,
      credentialProof,
      holderPublicKey: ISSUER_PK,
      issuanceChallengeHash: proofChallengeBytes,
      privateParts: {
        claimValues: {
          firstNameValuePadded,
          lastNameValuePadded,
          dateOfBirthDays,
          documentNumberValue: bytes32(0),
          issuingStateValue,
        },
        openings: {
          firstNameOpening: bytes32(0x03),
          lastNameOpening: bytes32(0x04),
          dateOfBirthOpening,
          documentNumberOpening: bytes32(0),
          issuingStateOpening,
        },
      },
    },
  };

  const verificationRequestMessageId = bytes32(0x30);
  const verificationRequest = {
    envelope: envelope({ messageId: verificationRequestMessageId, threadId, initialMessage: true, respondsToMessageId: NO_RESPONSE, createdAt: 4n }),
    schema: ROUND_TRIP_DOMAIN,
    issuerVerificationMethodRef: ISSUER_METHOD,
    holderBindingProfile: 0,
    features: {
      supportsSelectiveDisclosure: true,
      supportsPredicateProofs: true,
      supportsVerifierScopedPseudonym: false,
      supportsSameHolderProof: false,
    },
    verifierChallengeHash: presentationChallengeBytes,
    body: {
      requireFirstNameDisclosure: false,
      requireLastNameDisclosure: false,
      requireAgeOverThreshold: false,
      requestedAgeThresholdYears: 0n,
      requireDocumentNumberDisclosure: false,
      requireIssuingStateDisclosure: false,
    },
  };
  const submissionMessageId = bytes32(0x31);
  const verificationSubmission = {
    envelope: envelope({ messageId: submissionMessageId, threadId, initialMessage: false, respondsToMessageId: verificationRequestMessageId, createdAt: 5n }),
    schema: ROUND_TRIP_DOMAIN,
    issuerVerificationMethodRef: ISSUER_METHOD,
    holderBindingProfile: 0,
    challengeHash: presentationChallengeBytes,
    body: { credential, credentialProof, presentation, presentationProof },
  };
  const verificationResultMessageId = bytes32(0x32);
  const verificationResult = {
    envelope: envelope({ messageId: verificationResultMessageId, threadId, initialMessage: false, respondsToMessageId: submissionMessageId, createdAt: 6n }),
    approved: true,
    body: { credentialRoot: credentialBodyRoot, verifiedThresholdYears: 0n },
  };

  const presentationRequest =
    pureCircuits.digitalPassportPresentationRequestFromProtocol(verificationRequest);

  const steps = [
    ['credentialBodyRoot', () => pureCircuits.digitalPassportCredentialBodyRoot(credential)],
    ['presentationBodyRoot', () => pureCircuits.digitalPassportPresentationBodyRoot(presentation)],
    ['presentationRequestBodyRoot', () => pureCircuits.digitalPassportPresentationRequestBodyRoot(presentationRequest)],
    ['assertValidDigitalPassportIssuanceOffer', () => pureCircuits.assertValidDigitalPassportIssuanceOffer(offer)],
    ['assertValidDigitalPassportIssuanceRequest', () => pureCircuits.assertValidDigitalPassportIssuanceRequest(issuanceRequest)],
    ['assertDigitalPassportIssuanceRequestMatchesOffer', () => pureCircuits.assertDigitalPassportIssuanceRequestMatchesOffer(offer, issuanceRequest)],
    ['assertValidDigitalPassportIssuanceResult', () => pureCircuits.assertValidDigitalPassportIssuanceResult(issuanceResult)],
    ['assertDigitalPassportIssuanceResultMatchesRequest', () => pureCircuits.assertDigitalPassportIssuanceResultMatchesRequest(issuanceRequest, issuanceResult)],
    ['assertValidDigitalPassportVerificationRequestMessage', () => pureCircuits.assertValidDigitalPassportVerificationRequestMessage(verificationRequest)],
    ['assertValidDigitalPassportVerificationSubmissionMessage', () => pureCircuits.assertValidDigitalPassportVerificationSubmissionMessage(verificationSubmission)],
    ['assertDigitalPassportVerificationSubmissionMatchesRequest', () => pureCircuits.assertDigitalPassportVerificationSubmissionMatchesRequest(verificationRequest, verificationSubmission)],
    ['assertValidDigitalPassportVerificationResultMessage', () => pureCircuits.assertValidDigitalPassportVerificationResultMessage(verificationResult)],
    ['assertDigitalPassportVerificationResultMatchesSubmission', () => pureCircuits.assertDigitalPassportVerificationResultMatchesSubmission(verificationSubmission, verificationResult)],
  ];

  const record = {};
  for (const [name, fn] of steps) {
    try {
      const result = fn();
      record[name] = {
        ok: true,
        result: Array.isArray(result) && result.length === 0 ? null : hex(result),
      };
    } catch (e) {
      record[name] = { ok: false, error: String((e && e.message) || e) };
    }
  }
  return record;
}

// ---------------------------------------------------------------------------
// Emit the reference
// ---------------------------------------------------------------------------

function runScenario(scenario) {
  try {
    scenario.run();
    return { ok: true };
  } catch (e) {
    return { ok: false, error: String((e && e.message) || e) };
  }
}

const civilDateHelpers = {};
for (const scenario of civilDateScenarios) {
  civilDateHelpers[scenario.name] = {
    site: scenario.site,
    inputs: scenario.inputs,
    outcome: runScenario(scenario),
  };
}

const fixture = {
  contract:
    'examples/dogfood/digital-passport-credential/src/digital-passport-credential.compact',
  target: 'ts',
  producer: 'compactc --target ts --skip-zk',
  civilDateHelpers,
  roundTrip: buildRoundTrip(),
};

process.stdout.write(JSON.stringify(fixture, null, 2) + '\n');
