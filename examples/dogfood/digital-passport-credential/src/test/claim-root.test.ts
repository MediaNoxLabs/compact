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

import { readFileSync } from 'node:fs';
import path from 'node:path';

import { setNetworkId } from '@midnight-ntwrk/midnight-js-network-id';
import { describe, expect, it } from 'vitest';

import { pureCircuits } from '../managed/digital-passport-credential/contract/index.js';
import { createDigitalPassportFixture } from '../testing/credential-fixtures.js';

setNetworkId('undeployed');

const claimsSource = readFileSync(
  path.resolve(import.meta.dirname, '..', 'digital-passport-credential', 'claims.compact'),
  'utf8',
);

describe('digital-passport claim root', () => {
  it('uses a family-scoped domain separation tag', () => {
    expect(claimsSource).toContain('midnight:vc:digital-passport:v1');
  });

  it('commits each claim field through a domain-separated claim root', () => {
    const fixture = createDigitalPassportFixture();

    // The claim root should change if any commitment changes.
    const root1 = pureCircuits.digitalPassportClaimRoot(fixture.credential.claimCommitments);
    expect(root1).toBeInstanceOf(Uint8Array);
    expect(root1.length).toBe(32);

    // Altering one commitment must produce a different root.
    const alteredCommitments = {
      ...fixture.credential.claimCommitments,
      dateOfBirthCommitment: new Uint8Array(32).fill(99),
    };
    const root2 = pureCircuits.digitalPassportClaimRoot(alteredCommitments);
    expect(root2).not.toEqual(root1);
  });

  it('produces deterministic commitments for each field', () => {
    const fixture = createDigitalPassportFixture();

    const firstNameCommit = pureCircuits.firstNameCommitment(
      fixture.privateParts.claimValues.firstNameValuePadded,
      fixture.privateParts.openings.firstNameOpening,
    );
    const lastNameCommit = pureCircuits.lastNameCommitment(
      fixture.privateParts.claimValues.lastNameValuePadded,
      fixture.privateParts.openings.lastNameOpening,
    );
    const dateOfBirthCommit = pureCircuits.dateOfBirthCommitment(
      fixture.privateParts.claimValues.dateOfBirthDays,
      fixture.privateParts.openings.dateOfBirthOpening,
    );
    const documentNumberCommit = pureCircuits.documentNumberCommitment(
      fixture.privateParts.claimValues.documentNumberValue,
      fixture.privateParts.openings.documentNumberOpening,
    );
    const issuingStateCommit = pureCircuits.issuingStateCommitment(
      fixture.privateParts.claimValues.issuingStateValue,
      fixture.privateParts.openings.issuingStateOpening,
    );

    expect(firstNameCommit).toEqual(fixture.credential.claimCommitments.firstNameCommitment);
    expect(lastNameCommit).toEqual(fixture.credential.claimCommitments.lastNameCommitment);
    expect(dateOfBirthCommit).toEqual(fixture.credential.claimCommitments.dateOfBirthCommitment);
    expect(documentNumberCommit).toEqual(
      fixture.credential.claimCommitments.documentNumberCommitment,
    );
    expect(issuingStateCommit).toEqual(fixture.credential.claimCommitments.issuingStateCommitment);

    // Opening the same value with a different opening must produce a different commitment.
    const differentOpening = new Uint8Array(32).fill(42);
    const firstNameCommit2 = pureCircuits.firstNameCommitment(
      fixture.privateParts.claimValues.firstNameValuePadded,
      differentOpening,
    );
    expect(firstNameCommit2).not.toEqual(firstNameCommit);
  });

  it('produces a deterministic null commitment for absent documentNumber', () => {
    const nullCommit1 = pureCircuits.documentNumberNullCommitment();
    const nullCommit2 = pureCircuits.documentNumberNullCommitment();
    expect(nullCommit1).toBeInstanceOf(Uint8Array);
    expect(nullCommit1.length).toBe(32);
    expect(nullCommit1).toEqual(nullCommit2);

    // The null commitment must differ from any real documentNumber commitment.
    const fixture = createDigitalPassportFixture();
    expect(nullCommit1).not.toEqual(fixture.credential.claimCommitments.documentNumberCommitment);
  });
});
