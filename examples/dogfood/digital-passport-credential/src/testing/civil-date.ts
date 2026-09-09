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

// Calendar helpers for the age-over-threshold predicate. Compact has no
// integer division, so callers of `assertValidDigitalPassportAgePredicate`
// supply the civil-date decomposition of each Unix-epoch day number and the
// circuit verifies it by exact reconstruction (see
// `assertCivilDateMatchesEpochDays` in
// `src/digital-passport-credential/helpers.compact`). These helpers compute
// that decomposition off-chain.

import type { DigitalPassportCivilDate } from '../managed/digital-passport-credential/contract/index.js';

// Howard Hinnant's days_from_civil: the proleptic-Gregorian day number
// (day 0 = 1970-01-01) of a valid civil date.
export const epochDaysFromCivil = (year: number, month: number, day: number): bigint => {
  const yearAdjusted = month <= 2 ? year - 1 : year;
  const era = Math.floor(yearAdjusted / 400);
  const yearOfEra = yearAdjusted - era * 400;
  const dayOfyear = Math.floor((153 * (month + (month > 2 ? -3 : 9)) + 2) / 5) + day - 1;
  const dayOfEra =
    yearOfEra * 365 + Math.floor(yearOfEra / 4) - Math.floor(yearOfEra / 100) + dayOfyear;
  return BigInt(era * 146097 + dayOfEra - 719468);
};

// The circuit-facing decomposition of a non-negative Unix-epoch day number:
// its unique proleptic-Gregorian civil date plus the range-checked quotient
// witnesses the circuit uses to re-derive the day number.
export const civilDateFromEpochDays = (epochDays: bigint): DigitalPassportCivilDate => {
  const z = Number(epochDays) + 719468;
  const era = Math.floor(z / 146097);
  const dayOfEra = z - era * 146097;
  const yearOfEra = Math.floor(
    (dayOfEra -
      Math.floor(dayOfEra / 1460) +
      Math.floor(dayOfEra / 36524) -
      Math.floor(dayOfEra / 146096)) /
      365,
  );
  const year = yearOfEra + era * 400;
  const dayOfYear =
    dayOfEra - (365 * yearOfEra + Math.floor(yearOfEra / 4) - Math.floor(yearOfEra / 100));
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
