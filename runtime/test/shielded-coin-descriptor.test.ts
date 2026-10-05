// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
import { describe, expect, test } from 'vitest';
import * as r from '../src/index.js';
import * as ocrt from '@midnight-ntwrk/onchain-runtime-v3';

const max64 = (1n << 64n) - 1n;
const max128 = (1n << 128n) - 1n;
const values = [0n, 42n, max64, 1n << 64n, max128];
const coin = (value: bigint) => ({ nonce: new Uint8Array(32).fill(3), color: new Uint8Array(32).fill(4), value });
const recipient = (is_left: boolean): r.EncodedRecipient => ({
  is_left,
  left: { bytes: new Uint8Array(32).fill(is_left ? 7 : 0) },
  right: { bytes: new Uint8Array(32).fill(is_left ? 0 : 8) },
});
function commitment(value: bigint, is_left: boolean, legacy = false): string {
  const alignment = legacy
    ? [...r.Bytes32Descriptor.alignment(), ...r.Bytes32Descriptor.alignment(), ...r.MaxUint8Descriptor.alignment()]
    : r.ShieldedCoinInfoDescriptor.alignment();
  const aligned = ocrt.runtimeCoinCommitment(
    { value: r.ShieldedCoinInfoDescriptor.toValue(coin(value)), alignment },
    {
      value: r.ShieldedCoinRecipientDescriptor.toValue(recipient(is_left)),
      alignment: r.ShieldedCoinRecipientDescriptor.alignment(),
    },
  );
  return Buffer.from(r.Bytes32Descriptor.fromValue(aligned.value)).toString('hex');
}
const fixedContractCommitments = [
  'fe85a34810a7e9efb9cb35d636449974706ca01779dd64894cc3ed15938baa8c',
  '713bf4dd29d7636da261aceb128c81664b8a200f4290739ded72e1ff60141df3',
  'cfc17479dfe0edb2b000bd08f6b1e94cd89c36e54594b8cdffa164fd4a7f0a8d',
  '5724ddfbab36cc91c23f5af191c9c6632dd256c3c3db1cd55db70624e44fdf4f',
  '6ee151d0ed3d31e5589e8e20635cb75e9fb7452e09d1b3e878b94b9717572dfb',
];

describe('ShieldedCoinInfo u128 descriptor', () => {
  test('retains the existing u64 descriptor and uses b16 for coin values', () => {
    expect(r.MaxUint8Descriptor.maxValue).toBe(max64);
    expect(r.MaxUint8Descriptor.length).toBe(8);
    expect(r.MaxUint8Descriptor.fromValue(r.MaxUint8Descriptor.toValue(max64))).toBe(max64);
    expect(() => r.MaxUint8Descriptor.fromValue(ocrt.bigIntToValue(1n << 64n))).toThrow();
    expect(r.MaxUint16Descriptor.maxValue).toBe(max128);
    expect(r.MaxUint16Descriptor.length).toBe(16);
    expect(r.ShieldedCoinInfoDescriptor.alignment()).toEqual([
      { tag: 'atom', value: { tag: 'bytes', length: 32 } },
      { tag: 'atom', value: { tag: 'bytes', length: 32 } },
      { tag: 'atom', value: { tag: 'bytes', length: 16 } },
    ]);
  });
  test.each(values)('roundtrips coin value %s without truncation', (value) => {
    const encoded = r.ShieldedCoinInfoDescriptor.toValue(coin(value));
    expect(r.ShieldedCoinInfoDescriptor.fromValue(encoded)).toEqual(coin(value));
    expect(encoded).toHaveLength(0);
  });
  test('rejects a decoded value above u128 while preserving generic encoding behavior', () => {
    const encoded = r.ShieldedCoinInfoDescriptor.toValue(coin(max128 + 1n));
    expect(() => r.ShieldedCoinInfoDescriptor.fromValue(encoded)).toThrow(`expected UnsignedInteger[<=${max128}]`);
  });
  for (const is_left of [false, true]) {
    test.each(values)('upstream commitment and ordered output accept %s for recipient ' + is_left, (value) => {
      const hash = commitment(value, is_left);
      if (value <= max64) expect(hash).toBe(commitment(value, is_left, true));
      else expect(() => commitment(value, is_left, true)).toThrow(/alignment/);
      if (!is_left) expect(hash).toBe(fixedContractCommitments[values.indexOf(value)]);
      const context = r.createCircuitContext(
        ocrt.decodeContractAddress(new Uint8Array(32).fill(8)),
        '00'.repeat(32),
        new ocrt.ContractState(),
        null,
      );
      context.currentZswapLocalState.currentIndex = 13n;
      expect(r.createZswapOutput(context, coin(value), recipient(is_left))).toEqual([]);
      expect(context.currentQueryContext.comIndices.get(hash)).toBe(13n);
      expect(context.currentZswapLocalState.currentIndex).toBe(14n);
      expect(context.currentZswapLocalState.outputs).toEqual([{ coinInfo: coin(value), recipient: recipient(is_left) }]);
      expect(context.currentZswapLocalState.inputs).toEqual([]);
      r.createZswapOutput(context, coin(1n), recipient(is_left));
      expect(context.currentQueryContext.comIndices.get(commitment(1n, is_left))).toBe(14n);
      expect(context.currentZswapLocalState.currentIndex).toBe(15n);
      expect(context.currentZswapLocalState.outputs.map((output) => output.coinInfo.value)).toEqual([value, 1n]);
    });
    test.each([-1n, max128 + 1n])('rejects invalid %s before output mutation for recipient ' + is_left, (value) => {
      const context = r.createCircuitContext(
        ocrt.decodeContractAddress(new Uint8Array(32).fill(8)),
        '00'.repeat(32),
        new ocrt.ContractState(),
        null,
      );
      context.currentZswapLocalState.currentIndex = 13n;
      const query = context.currentQueryContext;
      const plan = context.currentZswapLocalState;
      expect(() => commitment(value, is_left)).toThrow();
      expect(() => r.createZswapOutput(context, coin(value), recipient(is_left))).toThrow();
      expect(context.currentQueryContext).toBe(query);
      expect(context.currentZswapLocalState).toBe(plan);
      expect(context.currentZswapLocalState.currentIndex).toBe(13n);
      expect(context.currentZswapLocalState.outputs).toEqual([]);
      expect(context.currentQueryContext.comIndices.size).toBe(0);
    });
  }
});
