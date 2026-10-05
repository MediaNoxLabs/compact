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

// SCALE framing for the pinned MidnightRuntimeApi Vec<u8> and Result<Vec<u8>, E>.

export function scaleCompact(value) {
  if (!Number.isSafeInteger(value) || value < 0) throw new Error('invalid SCALE length');
  if (value < 64) return Uint8Array.of(value << 2);
  if (value < 1 << 14) return Uint8Array.of(((value << 2) | 1) & 255, value >> 6);
  if (value < 1 << 30) {
    const encoded = (value * 4) + 2;
    return Uint8Array.of(encoded & 255, (encoded >>> 8) & 255,
      (encoded >>> 16) & 255, (encoded >>> 24) & 255);
  }
  const bytes = [];
  let rest = BigInt(value);
  while (rest) { bytes.push(Number(rest & 255n)); rest >>= 8n; }
  if (bytes.length > 67) throw new Error('SCALE length exceeds compact codec');
  return Uint8Array.of(((bytes.length - 4) << 2) | 3, ...bytes);
}

export function scaleVec(bytes) {
  if (!(bytes instanceof Uint8Array)) throw new Error('SCALE vector input must be bytes');
  return Uint8Array.of(...scaleCompact(bytes.length), ...bytes);
}

function compactAt(bytes, cursor) {
  if (cursor >= bytes.length) throw new Error('truncated SCALE compact length');
  const first = bytes[cursor++];
  const mode = first & 3;
  let value;
  if (mode === 0) value = first >> 2;
  else if (mode === 1) {
    if (cursor >= bytes.length) throw new Error('truncated SCALE compact length');
    value = ((first | (bytes[cursor++] << 8)) >>> 2);
    if (value < 64) throw new Error('noncanonical SCALE compact length');
  } else if (mode === 2) {
    if (cursor + 3 > bytes.length) throw new Error('truncated SCALE compact length');
    value = ((first | (bytes[cursor] << 8) | (bytes[cursor + 1] << 16) |
      (bytes[cursor + 2] << 24)) >>> 2);
    cursor += 3;
    if (value < 1 << 14) throw new Error('noncanonical SCALE compact length');
  } else {
    const count = (first >> 2) + 4;
    if (count > 7 || cursor + count > bytes.length) throw new Error('unsupported SCALE compact length');
    let wide = 0n;
    for (let i = 0; i < count; i++) wide |= BigInt(bytes[cursor + i]) << BigInt(8 * i);
    if (wide < 1n << 30n || wide > BigInt(Number.MAX_SAFE_INTEGER) || !bytes[cursor + count - 1]) {
      throw new Error('noncanonical SCALE compact length');
    }
    value = Number(wide);
    cursor += count;
  }
  return [value, cursor];
}

export function scaleBytes(hex, label = 'SCALE bytes') {
  if (typeof hex !== 'string' || !/^0x(?:[a-f0-9]{2})*$/i.test(hex)) {
    throw new Error(`${label} must be 0x-prefixed bytes`);
  }
  return Buffer.from(hex.slice(2), 'hex');
}

export function decodeScaleVec(bytes) {
  if (!(bytes instanceof Uint8Array)) throw new Error('SCALE vector must be bytes');
  const [length, cursor] = compactAt(bytes, 0);
  if (cursor + length !== bytes.length) throw new Error('SCALE vector has truncated or trailing bytes');
  return bytes.subarray(cursor);
}

export function decodeScaleResultBytes(bytes) {
  if (!(bytes instanceof Uint8Array) || bytes.length === 0) throw new Error('empty SCALE Result');
  if (bytes[0] === 1) throw new Error('node runtime API returned a LedgerApiError');
  if (bytes[0] !== 0) throw new Error('invalid SCALE Result discriminant');
  return decodeScaleVec(bytes.subarray(1));
}

export function decodeScaleString(bytes) {
  return new TextDecoder('utf-8', { fatal: true }).decode(decodeScaleVec(bytes));
}
