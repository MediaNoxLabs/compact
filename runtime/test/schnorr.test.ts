// This file is part of Compact.
// Copyright (C) 2025 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

import { describe, expect, test } from 'vitest';
import * as runtime from '../src/index.js';

const MSG_LEN = 3;
const msgType = new runtime.CompactTypeVector(MSG_LEN, runtime.CompactTypeField);
const sampleMsg = (): bigint[] => [1n, 2n, 3n];

describe('JubJub Schnorr (TypeScript)', () => {
  test('sign/verify roundtrip', () => {
    const sk = runtime.jubjubSampleScalar();
    const pk = runtime.jubjubSchnorrVerifyingKey(sk);
    const msg = sampleMsg();

    const sig = runtime.jubjubSchnorrSign(msgType, msg, sk);

    expect(runtime.jubjubSchnorrVerify(msgType, msg, pk, sig)).toBe(true);
  });

  test('verify rejects a bad signature', () => {
    const sk = runtime.jubjubSampleScalar();
    const pk = runtime.jubjubSchnorrVerifyingKey(sk);
    const msg = sampleMsg();

    const sig = runtime.jubjubSchnorrSign(msgType, msg, sk);
    const badSig = { ...sig, response: (sig.response + 1n) % runtime.JUBJUB_SCALAR_MODULUS };

    expect(runtime.jubjubSchnorrVerify(msgType, msg, pk, badSig)).toBe(false);
  });
});

describe('JubJub sampler compatibility', () => {
  test('uses the pinned scalar order and retains the public alias', () => {
    expect(runtime.JUBJUB_SCALAR_MODULUS).toBe(0x0e7db4ea6533afa906673b0101343b00a6682093ccc81082d0970e5ed6f72cb7n);
    expect(runtime.sampleJubjubSchnorrSk).toBe(runtime.jubjubSampleScalar);
    // The pinned primitive requires a canonical scalar, rather than reducing q.
    expect(() => runtime.ecMulGenerator(runtime.JUBJUB_SCALAR_MODULUS)).toThrow();
    expect(runtime.ecAdd(runtime.ecMulGenerator(runtime.JUBJUB_SCALAR_MODULUS - 1n), runtime.ecMulGenerator(1n))).toEqual({
      x: 0n,
      y: 1n,
    });
    for (let i = 0; i < 16; i++) {
      const scalar = runtime.sampleJubjubSchnorrSk();
      expect(scalar).toBeGreaterThanOrEqual(0n);
      expect(scalar).toBeLessThan(runtime.JUBJUB_SCALAR_MODULUS);
    }
  });
  test('real signatures bind the message and verifying key', () => {
    const key = runtime.jubjubSampleScalar();
    const verifyingKey = runtime.jubjubSchnorrVerifyingKey(key);
    const signature = runtime.jubjubSchnorrSign(msgType, sampleMsg(), key);
    expect(runtime.jubjubSchnorrVerify(msgType, [1n, 2n, 4n], verifyingKey, signature)).toBe(false);
    const other = runtime.jubjubSchnorrVerifyingKey((key + 1n) % runtime.JUBJUB_SCALAR_MODULUS);
    expect(runtime.jubjubSchnorrVerify(msgType, sampleMsg(), other, signature)).toBe(false);
  });
});
