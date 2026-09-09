// This file is part of midnightntwrk/midnight-verifiable-credential-digital-passport.
// Copyright (C) Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

import { setNetworkId } from '@midnight-ntwrk/midnight-js-network-id';
import { describe, expect, it } from 'vitest';

import { pureCircuits } from '../managed/digital-passport-credential/contract/index.js';
import { civilDateFromEpochDays, epochDaysFromCivil } from '../testing/civil-date.js';
import {
  createDigitalPassportFixture,
  createDigitalPassportFixtureWithDateOfBirth,
  signProof,
} from '../testing/credential-fixtures.js';

setNetworkId('undeployed');

describe('digital-passport credential: age predicate', () => {
  it('checks the private age witness against the committed date of birth', () => {
    const fixture = createDigitalPassportFixture();

    expect(() =>
      pureCircuits.assertValidDigitalPassportAgePredicate(
        fixture.credential,
        fixture.presentation,
        fixture.currentDay,
        fixture.privateParts.claimValues.dateOfBirthDays,
        fixture.privateParts.openings.dateOfBirthOpening,
        civilDateFromEpochDays(fixture.currentDay),
        civilDateFromEpochDays(fixture.privateParts.claimValues.dateOfBirthDays),
      ),
    ).not.toThrow();

    expect(() =>
      pureCircuits.assertValidDigitalPassportAgePredicate(
        fixture.credential,
        fixture.presentation,
        fixture.currentDay,
        fixture.privateParts.claimValues.dateOfBirthDays,
        new Uint8Array(32).fill(1),
        civilDateFromEpochDays(fixture.currentDay),
        civilDateFromEpochDays(fixture.privateParts.claimValues.dateOfBirthDays),
      ),
    ).toThrow(/Date-of-birth witness does not match credential commitment/);
  });

  it('rejects a predicate proof when the holder is below the requested threshold', () => {
    const fixture = createDigitalPassportFixture();

    const strictPresentation = {
      ...fixture.presentation,
      disclosed: {
        ...fixture.presentation.disclosed,
        ageThresholdYears: 30n,
      },
    };
    const strictPresentationProof = signProof({
      bodyRoot: pureCircuits.digitalPassportPresentationBodyRoot(strictPresentation),
      context: 'presentation',
      signer: fixture.holder,
      createdAt: fixture.presentationProof.createdAt + 1n,
      challengeHash: fixture.presentationProof.challengeHash,
      nonceScalar: 23n,
    });

    expect(() =>
      pureCircuits.assertValidDigitalPassportPresentation(
        fixture.credential,
        fixture.credentialProof,
        strictPresentation,
        strictPresentationProof,
      ),
    ).not.toThrow();

    expect(() =>
      pureCircuits.assertValidDigitalPassportAgePredicate(
        fixture.credential,
        strictPresentation,
        fixture.currentDay,
        fixture.privateParts.claimValues.dateOfBirthDays,
        fixture.privateParts.openings.dateOfBirthOpening,
        civilDateFromEpochDays(fixture.currentDay),
        civilDateFromEpochDays(fixture.privateParts.claimValues.dateOfBirthDays),
      ),
    ).toThrow(/Age predicate does not satisfy the requested threshold/);
  });

  it('rejects a leap-day-adjacent proof that satisfies the flat 365-days-per-year count', () => {
    // Born 2000-03-01, threshold 4 years, evaluated on 2004-02-29: the flat
    // day count is exactly 4 * 365 = 1460 days, which the old threshold * 365
    // check accepted even though the holder is still 3 calendar years old.
    const dateOfBirthDays = epochDaysFromCivil(2000, 3, 1);
    const currentDay = epochDaysFromCivil(2004, 2, 29);
    const fixture = createDigitalPassportFixtureWithDateOfBirth(dateOfBirthDays);
    const presentation = {
      ...fixture.presentation,
      disclosed: {
        ...fixture.presentation.disclosed,
        ageThresholdYears: 4n,
      },
    };

    expect(currentDay - dateOfBirthDays).toBe(4n * 365n);

    expect(() =>
      pureCircuits.assertValidDigitalPassportAgePredicate(
        fixture.credential,
        presentation,
        currentDay,
        fixture.privateParts.claimValues.dateOfBirthDays,
        fixture.privateParts.openings.dateOfBirthOpening,
        civilDateFromEpochDays(currentDay),
        civilDateFromEpochDays(dateOfBirthDays),
      ),
    ).toThrow(/Age predicate does not satisfy the requested threshold/);
  });

  it('accepts the same holder on the calendar anniversary', () => {
    const dateOfBirthDays = epochDaysFromCivil(2000, 3, 1);
    const currentDay = epochDaysFromCivil(2004, 3, 1);
    const fixture = createDigitalPassportFixtureWithDateOfBirth(dateOfBirthDays);
    const presentation = {
      ...fixture.presentation,
      disclosed: {
        ...fixture.presentation.disclosed,
        ageThresholdYears: 4n,
      },
    };

    expect(() =>
      pureCircuits.assertValidDigitalPassportAgePredicate(
        fixture.credential,
        presentation,
        currentDay,
        fixture.privateParts.claimValues.dateOfBirthDays,
        fixture.privateParts.openings.dateOfBirthOpening,
        civilDateFromEpochDays(currentDay),
        civilDateFromEpochDays(dateOfBirthDays),
      ),
    ).not.toThrow();
  });

  it('rejects a February-29 holder the day before the leap-day anniversary', () => {
    // Born 2000-02-29, threshold 4 years: on 2004-02-28 the elapsed flat day
    // count already reaches 4 * 365, but the holder reaches age 4 only on
    // 2004-02-29.
    const dateOfBirthDays = epochDaysFromCivil(2000, 2, 29);
    const currentDay = epochDaysFromCivil(2004, 2, 28);
    const fixture = createDigitalPassportFixtureWithDateOfBirth(dateOfBirthDays);
    const presentation = {
      ...fixture.presentation,
      disclosed: {
        ...fixture.presentation.disclosed,
        ageThresholdYears: 4n,
      },
    };

    expect(currentDay - dateOfBirthDays).toBe(4n * 365n);

    expect(() =>
      pureCircuits.assertValidDigitalPassportAgePredicate(
        fixture.credential,
        presentation,
        currentDay,
        fixture.privateParts.claimValues.dateOfBirthDays,
        fixture.privateParts.openings.dateOfBirthOpening,
        civilDateFromEpochDays(currentDay),
        civilDateFromEpochDays(dateOfBirthDays),
      ),
    ).toThrow(/Age predicate does not satisfy the requested threshold/);
  });

  it('accepts a February-29 holder on the leap-day anniversary', () => {
    const dateOfBirthDays = epochDaysFromCivil(2000, 2, 29);
    const currentDay = epochDaysFromCivil(2004, 2, 29);
    const fixture = createDigitalPassportFixtureWithDateOfBirth(dateOfBirthDays);
    const presentation = {
      ...fixture.presentation,
      disclosed: {
        ...fixture.presentation.disclosed,
        ageThresholdYears: 4n,
      },
    };

    expect(() =>
      pureCircuits.assertValidDigitalPassportAgePredicate(
        fixture.credential,
        presentation,
        currentDay,
        fixture.privateParts.claimValues.dateOfBirthDays,
        fixture.privateParts.openings.dateOfBirthOpening,
        civilDateFromEpochDays(currentDay),
        civilDateFromEpochDays(dateOfBirthDays),
      ),
    ).not.toThrow();
  });

  it('rejects a civil-date decomposition that does not reconstruct the day number', () => {
    const fixture = createDigitalPassportFixture();

    expect(() =>
      pureCircuits.assertValidDigitalPassportAgePredicate(
        fixture.credential,
        fixture.presentation,
        fixture.currentDay,
        fixture.privateParts.claimValues.dateOfBirthDays,
        fixture.privateParts.openings.dateOfBirthOpening,
        civilDateFromEpochDays(fixture.currentDay + 1n),
        civilDateFromEpochDays(fixture.privateParts.claimValues.dateOfBirthDays),
      ),
    ).toThrow(/Civil date does not match the day number/);
  });
});
