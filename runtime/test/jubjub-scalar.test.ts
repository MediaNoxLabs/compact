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
import { afterEach, describe, expect, test, vi } from 'vitest';

const entropy = vi.hoisted(() => ({ bytes: vi.fn() }));
vi.mock('@noble/hashes/utils.js', async (original) => ({
  ...(await original<typeof import('@noble/hashes/utils.js')>()),
  randomBytes: entropy.bytes,
}));
import {
  jubjubSampleScalar,
  sampleJubjubSchnorrSk,
  JUBJUB_SCALAR_MODULUS as q,
  jubjubSchnorrSign,
  jubjubSchnorrVerify,
  jubjubSchnorrVerifyingKey,
  ecMulGenerator,
  CompactTypeField,
} from '../src/index.js';

function littleEndian(value: bigint): Uint8Array {
  return Uint8Array.from({ length: 32 }, (_, i) => Number((value >> BigInt(8 * i)) & 255n));
}
afterEach(() => entropy.bytes.mockReset());

describe('uniform JubJub scalar rejection sampling', () => {
  test.each([0n, q - 1n])('accepts scalar boundary %s', (value) => {
    entropy.bytes.mockReturnValueOnce(littleEndian(value));
    expect(jubjubSampleScalar()).toBe(value);
    expect(entropy.bytes).toHaveBeenCalledExactlyOnceWith(32);
  });
  test('rejects q and the maximum candidate, redrawing fresh entropy each time', () => {
    entropy.bytes.mockReturnValueOnce(littleEndian(q));
    entropy.bytes.mockReturnValueOnce(littleEndian((1n << 252n) - 1n));
    entropy.bytes.mockReturnValueOnce(littleEndian(42n));
    expect(jubjubSampleScalar()).toBe(42n);
    expect(entropy.bytes.mock.calls).toEqual([[32], [32], [32]]);
  });
  test('masks unused high bits without changing the little-endian value', () => {
    const bytes = littleEndian(0x01020304n);
    bytes[31] = 0xf0;
    entropy.bytes.mockReturnValueOnce(bytes);
    expect(sampleJubjubSchnorrSk()).toBe(0x01020304n);
    expect(entropy.bytes).toHaveBeenCalledExactlyOnceWith(32);
  });
  test('preserves the highest meaningful candidate bits', () => {
    entropy.bytes.mockReturnValueOnce(littleEndian((1n << 251n) + 1n));
    expect(jubjubSampleScalar()).toBe((1n << 251n) + 1n);
    expect(entropy.bytes).toHaveBeenCalledExactlyOnceWith(32);
  });
  test('propagates entropy-provider failure without a fallback', () => {
    const error = new Error('CSPRNG unavailable');
    entropy.bytes.mockImplementationOnce(() => {
      throw error;
    });
    expect(() => jubjubSampleScalar()).toThrow(error);
    expect(entropy.bytes).toHaveBeenCalledExactlyOnceWith(32);
  });
});

describe('canonical Schnorr signing-key boundary', () => {
  test.each([-1n, -q, q, q + 1n, -(1n << 512n), 1n << 512n])(
    'rejects key %s before requesting entropy or encoding the message',
    (key) => {
      const messageType = {
        alignment: vi.fn(() => []),
        toValue: vi.fn(() => []),
        fromValue: vi.fn(() => 0n),
      };
      expect(() => jubjubSchnorrSign(messageType, 42n, key)).toThrow(
        new RangeError('jubjubSchnorrSign: signing key must be in [0, JUBJUB_SCALAR_MODULUS)'),
      );
      expect(entropy.bytes).not.toHaveBeenCalled();
      expect(messageType.alignment).not.toHaveBeenCalled();
      expect(messageType.toValue).not.toHaveBeenCalled();
      expect(messageType.fromValue).not.toHaveBeenCalled();
    },
  );
  test.each([0n, q - 1n])('signs with canonical boundary key %s and the supplied nonce', (key) => {
    entropy.bytes.mockReturnValueOnce(littleEndian(1n));
    const signature = jubjubSchnorrSign(CompactTypeField, 42n, key);
    expect(entropy.bytes).toHaveBeenCalledExactlyOnceWith(32);
    expect(signature.announcement).toEqual(ecMulGenerator(1n));
    expect(signature.response).toBeGreaterThanOrEqual(0n);
    expect(signature.response).toBeLessThan(q);
    expect(jubjubSchnorrVerify(CompactTypeField, 42n, jubjubSchnorrVerifyingKey(key), signature)).toBe(true);
  });
});
