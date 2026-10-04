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

export function normalizeHash(value) {
  if (typeof value !== 'string' || !/^(?:0x)?[0-9a-f]{64}$/i.test(value)) {
    throw new Error('expected a 32-byte hex hash');
  }
  return value.replace(/^0x/i, '').toLowerCase();
}

export function isExpectedAction(action, type, address, transactionHash) {
  if (!action || action.__typename !== type) return false;
  if (normalizeHash(action.address) !== normalizeHash(address)) return false;
  return normalizeHash(action.transaction?.hash) === normalizeHash(transactionHash);
}

export async function hasFinalizedCanonicalBlock(block, rpc) {
  if (!Number.isSafeInteger(block?.height) || block.height < 0) {
    throw new Error('indexed action has an invalid block height');
  }
  const observedHash = normalizeHash(block.hash);
  const finalizedHash = normalizeHash(await rpc('chain_getFinalizedHead', []));
  const header = await rpc('chain_getHeader', [`0x${finalizedHash}`]);
  if (typeof header?.number !== 'string' || !/^0x[0-9a-f]+$/i.test(header.number)) {
    throw new Error('node returned an invalid finalized header');
  }
  const finalizedHeight = Number(BigInt(header.number));
  if (!Number.isSafeInteger(finalizedHeight)) {
    throw new Error('node returned a finalized height outside JavaScript integer range');
  }
  if (block.height > finalizedHeight) return false;
  const canonicalHash = normalizeHash(await rpc('chain_getBlockHash', [block.height]));
  if (canonicalHash !== observedHash) {
    throw new Error(`indexed block ${block.height} differs from node finalized chain`);
  }
  return true;
}
