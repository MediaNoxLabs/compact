// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
import assert from 'node:assert/strict';
import test from 'node:test';
import { requireOfferReconciliation } from './shielded-handoff.mjs';

// An explicit module path permits reuse of an installed, pinned acceptance
// dependency directory without writing through a shared node_modules symlink.
const ledger = await import(process.env.COMPACT_LEDGER_V8_MODULE ?? '@midnight-ntwrk/ledger-v8');

test('a real filtered coin witness does not supply full-ledger allocation/history authority', () => {
  const address = '03'.repeat(32);
  const coin = { nonce: '01'.repeat(32), type: '02'.repeat(32), value: 1n };
  const output = ledger.ZswapOutput.newContractOwned(coin, undefined, address);
  const [created, indices] = new ledger.ZswapChainState().tryApply(ledger.ZswapOffer.fromOutput(output));
  const full = created.postBlockUpdate(new Date('2026-10-06T00:00:00Z'));
  const filtered = full.filter(address);
  assert.equal(full.firstFree, 1n);
  assert.equal(filtered.firstFree, 0n);
  const input = ledger.ZswapInput.newContractOwned(
    { ...coin, mt_index: indices.get(output.commitment) }, undefined, address, filtered,
  );
  const spend = ledger.ZswapOffer.fromInput(input);
  // The same actual input succeeds against the complete state and fails against
  // its projection. No missing keys, proofs or services cause this refusal.
  assert.equal(requireOfferReconciliation(full, spend).after.firstFree, 1n);
  assert.throws(() => requireOfferReconciliation(filtered, spend), error => {
    assert.match(error.message, /cannot reconcile the exact offer/);
    assert.match(String(error.cause), /unknown coin tree root/);
    return true;
  });
});
