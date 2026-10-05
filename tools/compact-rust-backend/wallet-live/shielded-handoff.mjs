// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Offline guards shared by the forthcoming ABI49 live acceptance driver.
import { createHash } from 'node:crypto';
import { writeFile } from 'node:fs/promises';
import { hasFinalizedCanonicalBlock, isExpectedAction, normalizeHash } from './provenance.mjs';

function hexBytes(value, label) {
  if (typeof value !== 'string' || !/^(?:[0-9a-f]{2})+$/i.test(value)) {
    throw new Error(`${label} must be nonempty serialized hex`);
  }
  return Buffer.from(value, 'hex');
}
export function sha256(bytes) {
  return createHash('sha256').update(bytes).digest('hex');
}

// A same-action pair is an observation, not an authenticated full ledger.
export async function confirmedShieldedObservation(action, expected, rpc, ledger) {
  if (!isExpectedAction(action, expected.type, expected.address, expected.transactionHash)) {
    throw new Error('indexed action does not match the submitted transaction/address/type');
  }
  if (expected.type === 'ContractCall' && action.entryPoint !== expected.entryPoint) {
    throw new Error('indexed entry point differs from the submitted call');
  }
  if (!await hasFinalizedCanonicalBlock(action.transaction.block, rpc)) {
    throw new Error('indexed action is not under the trusted node finalized head');
  }
  const stateBytes = hexBytes(action.state, 'contract state');
  const zswapBytes = hexBytes(action.zswapState, 'contract-specific Zswap state');
  ledger.ContractState.deserialize(stateBytes);
  ledger.ZswapChainState.deserialize(zswapBytes);
  return {
    stateBytes, zswapBytes,
    provenance: {
      trust: 'connected-node-and-indexer',
      zswapScope: 'contract-specific-observation-not-full-ledger',
      address: normalizeHash(action.address),
      transactionHash: normalizeHash(action.transaction.hash),
      blockHash: normalizeHash(action.transaction.block.hash),
      blockHeight: action.transaction.block.height,
      entryPoint: action.entryPoint ?? null,
      stateSha256: sha256(stateBytes),
      zswapSha256: sha256(zswapBytes),
    },
  };
}

// Called on the already proven Rust handoff and on its wallet-finalized result.
// Comparing exact serialized offers retains proof identity as well as coin IDs.
export function shieldedOfferFingerprint(transaction) {
  const encode = offer => offer == null ? null : sha256(offer.serialize());
  const fallible = [...(transaction.fallibleOffer ?? new Map()).entries()]
    .map(([segment, offer]) => {
      if (!Number.isInteger(segment) || segment <= 0 || segment > 65535) {
        throw new Error('invalid fallible shielded segment');
      }
      return [segment, encode(offer)];
    }).sort(([a], [b]) => a - b);
  return { guaranteed: encode(transaction.guaranteedOffer), fallible };
}
export function assertShieldedOffersUnchanged(before, after) {
  if (JSON.stringify(shieldedOfferFingerprint(before)) !==
      JSON.stringify(shieldedOfferFingerprint(after))) {
    throw new Error('wallet finalization changed the retained shielded offer or placement');
  }
}

// A projection can produce a Merkle membership witness without supporting
// allocation or spent-nullifier checks. Never repair it by inventing metadata.
export function requireOfferReconciliation(state, offer) {
  try {
    const [after, indices] = state.tryApply(offer);
    return { after, indices };
  } catch (cause) {
    throw new Error('observed Zswap view cannot reconcile the exact offer; require an authoritative snapshot or an explicitly approved observation policy', { cause });
  }
}

// PreProof input witnesses can contain secret material. Keep the private file
// local and exclusive; only its hash/length belongs in a public receipt.
export async function writePrivateHandoff(path, bytes) {
  await writeFile(path, bytes, { flag: 'wx', mode: 0o600 });
  return { bytes: bytes.length, sha256: sha256(bytes) };
}
