// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

import assert from 'node:assert/strict';
import test from 'node:test';
import { hasFinalizedCanonicalBlock, isExpectedAction, normalizeHash } from './provenance.mjs';

const address = 'a'.repeat(64);
const submitted = 'b'.repeat(64);
const blockHash = 'c'.repeat(64);
const finalizedHash = 'd'.repeat(64);

function action(hash = submitted) {
  return {
    __typename: 'ContractCall',
    address,
    transaction: { hash, block: { height: 12, hash: blockHash } },
  };
}

function rpcAt(finalizedHeight, canonicalHash = blockHash) {
  const requests = [];
  const rpc = async (method, params) => {
    requests.push([method, params]);
    if (method === 'chain_getFinalizedHead') return `0x${finalizedHash}`;
    if (method === 'chain_getHeader') return { number: `0x${finalizedHeight.toString(16)}` };
    if (method === 'chain_getBlockHash') return `0x${canonicalHash}`;
    throw new Error(`unexpected RPC ${method}`);
  };
  return { rpc, requests };
}

test('only the exact final transaction action qualifies', () => {
  assert.equal(isExpectedAction(action(), 'ContractCall', address, `0x${submitted}`), true);
  assert.equal(isExpectedAction(action('e'.repeat(64)), 'ContractCall', address, submitted), false);
  assert.equal(isExpectedAction(action(), 'ContractDeploy', address, submitted), false);
  assert.equal(isExpectedAction(action(), 'ContractCall', 'f'.repeat(64), submitted), false);
  assert.throws(() => normalizeHash('not-a-hash'), /32-byte hex hash/);
  assert.throws(() => isExpectedAction(action('bad'), 'ContractCall', address, submitted), /32-byte hex hash/);
});

test('node finalized head contains the indexed canonical block', async () => {
  const { rpc, requests } = rpcAt(15);
  assert.equal(await hasFinalizedCanonicalBlock(action().transaction.block, rpc), true);
  assert.deepEqual(requests.map(([method]) => method), [
    'chain_getFinalizedHead', 'chain_getHeader', 'chain_getBlockHash',
  ]);
  assert.deepEqual(requests[2][1], [12]);
});

test('an action above the node finalized height waits', async () => {
  const { rpc, requests } = rpcAt(11);
  assert.equal(await hasFinalizedCanonicalBlock(action().transaction.block, rpc), false);
  assert.equal(requests.length, 2);
});

test('mismatched canonical block and malformed metadata fail', async () => {
  const { rpc } = rpcAt(15, 'e'.repeat(64));
  await assert.rejects(hasFinalizedCanonicalBlock(action().transaction.block, rpc),
    /differs from node finalized chain/);
  await assert.rejects(hasFinalizedCanonicalBlock({ height: -1, hash: blockHash }, rpc),
    /invalid block height/);
  await assert.rejects(hasFinalizedCanonicalBlock({ height: 12, hash: 'bad' }, rpc),
    /32-byte hex hash/);
});
