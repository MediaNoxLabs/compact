// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { mkdtemp, readFile, rm, stat } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { isAbsolute, join } from 'node:path';
import test from 'node:test';
import { acquireVerifiedCheckpoint, assertNodeProfile, CANDIDATE_COMPATIBILITY_PROFILE, createIndexerClient, writeCheckpoint } from './checkpoint.mjs';
import { decodeScaleResultBytes, decodeScaleString, decodeScaleVec, scaleBytes, scaleCompact, scaleVec } from './checkpoint-scale.mjs';

const address = '11'.repeat(32);
const blockHash = '22'.repeat(32);
const parentHash = '33'.repeat(32);
const txHash = '44'.repeat(32);
const root = Uint8Array.of(0, ...Array(32).fill(7));
const taggedAddress = Uint8Array.of(...Buffer.from('midnight:contract-address[v2]:', 'utf8'), ...Buffer.from(address, 'hex'));
const walletBytes = Uint8Array.of(5, 1, 2);
const hex = bytes => Buffer.from(bytes).toString('hex');
const ok = bytes => `0x${hex(Uint8Array.of(0, ...scaleVec(bytes)))}`;
const fields = () => ({
  addressHex: address, addressTaggedBytes: taggedAddress, walletSnapshotBytes: walletBytes,
  walletAppliedEventId: '12', networkId: 'undeployed', ledgerVersion: '8.0.3',
  indexerVersion: '4.0.1',
  walletEnvelope: JSON.stringify({ state: hex(walletBytes), offset: 12, networkId: 'undeployed' }),
});
const action = () => ({
  __typename: 'ContractCall', address, state: 'abcd', entryPoint: 'accept',
  transaction: { hash: txHash, block: { hash: blockHash, height: 5 } },
});
const block = () => ({
  hash: blockHash, height: 5, protocolVersion: 22000,
  parent: { hash: parentHash, height: 4 },
  transactions: [{
    __typename: 'RegularTransaction', id: 7, hash: txHash, protocolVersion: 22000,
    startIndex: 4, endIndex: 5, merkleTreeRoot: hex(root),
    zswapLedgerEvents: [{ id: 12, maxId: 400, raw: '00', protocolVersion: 22000 }],
    contractActions: [{ __typename: 'ContractCall', address, state: 'abcd',
      zswapState: '0102', entryPoint: 'accept' }],
  }],
});
function harness() {
  const indexed = block();
  const calls = [];
  const rpc = async (method, args) => {
    calls.push([method, args]);
    if (method === 'system_version') return '0.22.3-6f0ef437';
    if (method === 'chain_getFinalizedHead') return `0x${blockHash}`;
    if (method === 'chain_getHeader') return {
      number: '0x5', parentHash: `0x${parentHash}`,
    };
    if (method === 'chain_getBlockHash') return `0x${blockHash}`;
    if (method === 'state_call') {
      const [name, input, at] = args;
      assert.equal(at, `0x${blockHash}`);
      if (name.endsWith('_get_contract_state') || name.endsWith('_get_zswap_chain_state')) {
        assert.equal(hex(decodeScaleVec(scaleBytes(input))), address);
      } else assert.equal(input, '0x');
      if (name.endsWith('_get_contract_state')) return ok(Uint8Array.of(0xab, 0xcd));
      if (name.endsWith('_get_zswap_chain_state')) return ok(Uint8Array.of(1, 2));
      if (name.endsWith('_get_zswap_state_root')) return ok(root);
      if (name.endsWith('_get_network_id')) return `0x${hex(scaleVec(Buffer.from('undeployed')))}`;
      if (name.endsWith('_get_ledger_version')) return `0x${hex(scaleVec(Buffer.from('=8.0.2')))}`;
    }
    throw new Error(`unexpected node call ${method}`);
  };
  const indexer = { getBlock: async hash => {
    assert.equal(hash, blockHash);
    return indexed;
  } };
  const ledger = {
    ContractState: { deserialize: bytes => { assert.equal(hex(bytes), 'abcd'); return {}; } },
    ZswapChainState: { deserialize: bytes => { assert.equal(hex(bytes), '0102'); return { firstFree: 0n }; } },
    ZswapLocalState: { deserialize: bytes => { assert.equal(hex(bytes), '050102'); return { firstFree: 5n }; } },
  };
  return { indexed, calls, rpc, indexer, ledger, expectedAction: action() };
}
const acquire = (overrides = {}, setup = harness()) =>
  acquireVerifiedCheckpoint({ ...fields(), ...setup, ...overrides });

test('pinned SCALE Vec and Result golden bytes reject malformed frames', () => {
  assert.equal(hex(scaleCompact(0)), '00');
  assert.equal(hex(scaleCompact(63)), 'fc');
  assert.equal(hex(scaleCompact(64)), '0101');
  assert.equal(hex(scaleCompact(16384)), '02000100');
  assert.equal(hex(scaleVec(Uint8Array.of(0xaa, 0xbb))), '08aabb');
  const realTagged = Buffer.from('6d69646e696768743a636f6e74726163742d616464726573735b76325d3ad88251c3fc3b7f378b75a0973843ba43ed835616270cca64a6b4434837b1166b', 'hex');
  assert.equal(hex(scaleVec(realTagged)), `f8${hex(realTagged)}`);
  assert.equal(hex(scaleVec(Buffer.from('d88251c3fc3b7f378b75a0973843ba43ed835616270cca64a6b4434837b1166b', 'hex'))),
    '80d88251c3fc3b7f378b75a0973843ba43ed835616270cca64a6b4434837b1166b');
  assert.equal(decodeScaleString(scaleBytes('0x28756e6465706c6f796564')), 'undeployed');
  assert.equal(decodeScaleString(scaleBytes('0x183d382e302e32')), '=8.0.2');
  const liveRoot = scaleBytes('0x008473b35bda8df702a240f2b7605bca3ea4f7bdb4110f5c6d35c58ed512faf7697303');
  assert.equal(decodeScaleResultBytes(liveRoot).length, 33);
  assert.throws(() => decodeScaleResultBytes(Uint8Array.of(1, 0)), /LedgerApiError/);
  assert.throws(() => decodeScaleResultBytes(Uint8Array.of(0, 8, 1)), /truncated/);
  assert.throws(() => decodeScaleVec(Uint8Array.of(4, 1, 2)), /trailing/);
  assert.throws(() => decodeScaleVec(Uint8Array.of(0xfd, 0x00)), /noncanonical/);
});

test('pinned indexer query requests all transactions and event IDs for exact block hash', async () => {
  let request;
  const client = createIndexerClient('http://example.invalid/api/v3/graphql', async (_, init) => {
    request = JSON.parse(init.body);
    return { ok: true, json: async () => ({ data: { block: block() } }) };
  });
  assert.equal((await client.getBlock(blockHash)).hash, blockHash);
  assert.deepEqual(request.variables, { offset: { hash: blockHash } });
  for (const field of ['transactions', 'zswapLedgerEvents', 'endIndex', 'merkleTreeRoot', 'contractActions']) {
    assert.match(request.query, new RegExp(field));
  }
});

test('checkpoint binds full block frontier, actual last event and exact wallet bytes', async () => {
  const h = harness();
  const acquired = await acquire({}, h);
  assert.equal(acquired.manifest.metadata.firstFree, '5');
  assert.equal(acquired.manifest.metadata.finalZswapEventId, '12');
  assert.equal(acquired.manifest.metadata.nodeZswapRootHex, hex(root));
  assert.equal(acquired.manifest.metadata.nodeLedgerConstraint, '=8.0.2');
  assert.equal(acquired.manifest.metadata.compatibilityProfile, CANDIDATE_COMPATIBILITY_PROFILE.compatibilityProfile);
  assert.equal(acquired.manifest.metadata.compatibilityStatus, 'candidate-live-validation-pending');
  assert.equal(acquired.manifest.metadata.nodeSoftwareVersion, '0.22.3-6f0ef437');
  assert.equal(acquired.manifest.metadata.indexerVersion, '4.0.1');
  assert.equal(acquired.manifest.wallet.walletStateSha256.length, 64);
  assert.deepEqual(acquired.binaries.walletState, walletBytes);
  assert.equal(h.calls.filter(([method]) => method === 'state_call').length, 5);
  const directory = await mkdtemp(join(tmpdir(), 'compact-checkpoint-parent-'));
  try {
    const { manifestPath, manifest } = await writeCheckpoint(join(directory, 'checkpoint'), acquired);
    assert.equal((await stat(join(directory, 'checkpoint'))).mode & 0o777, 0o700);
    assert.equal((await stat(manifestPath)).mode & 0o777, 0o600);
    assert.equal(isAbsolute(manifest.files.walletState.path), true);
    assert.equal(hex(await readFile(manifest.files.walletState.path)), hex(walletBytes));
    assert.equal(manifest.files.contractTree.encoding, 'ZswapChainState');
    for (const reference of Object.values(manifest.files)) {
      assert.equal(isAbsolute(reference.path), true);
      assert.equal((await stat(reference.path)).size, reference.bytes);
      assert.equal(createHash('sha256').update(await readFile(reference.path)).digest('hex'), reference.sha256);
    }
    assert.deepEqual(JSON.parse(await readFile(manifestPath, 'utf8')), manifest);
  } finally { await rm(directory, { recursive: true, force: true }); }
});

test('preflight pins raw node, indexer and decoder identities before wallet startup', async () => {
  const h = harness();
  const profile = await assertNodeProfile(h.rpc, { indexerVersion: '4.0.1' });
  assert.equal(profile.blockHash, blockHash);
  assert.equal(profile.nodeLedgerConstraint, '=8.0.2');
  assert.deepEqual(h.calls.map(([method]) => method), [
    'chain_getFinalizedHead', 'system_version', 'state_call',
  ]);
  await assert.rejects(assertNodeProfile(h.rpc, { indexerVersion: '4.0.2' }), /pairing is not approved/);
  await assert.rejects(assertNodeProfile(h.rpc, { indexerVersion: '4.0.1', ledgerVersion: '8.0.2' }), /pairing is not approved/);
  const changedNode = async (method, args) => method === 'system_version' ? '0.22.5' : h.rpc(method, args);
  await assert.rejects(assertNodeProfile(changedNode, { indexerVersion: '4.0.1' }), /pairing is not approved/);
});

test('later same-block transaction must determine final frontier/root/event', async () => {
  const h = harness();
  const nextRoot = Uint8Array.of(0, ...Array(32).fill(8));
  h.indexed.transactions.push({
    __typename: 'RegularTransaction', id: 8, hash: '55'.repeat(32), protocolVersion: 22000,
    startIndex: 5, endIndex: 6, merkleTreeRoot: hex(nextRoot),
    zswapLedgerEvents: [{ id: 13, maxId: 400, raw: '01', protocolVersion: 22000 }],
    contractActions: [],
  });
  await assert.rejects(acquire({}, h), /root mismatch/);
  const nodeWithFinalRoot = async (method, args) => {
    if (method === 'state_call' && args[0].endsWith('_get_zswap_state_root')) return ok(nextRoot);
    return h.rpc(method, args);
  };
  h.ledger.ZswapLocalState.deserialize = bytes => {
    assert.equal(hex(bytes), hex(walletBytes));
    return { firstFree: 6n };
  };
  const acquired = await acquire({
    rpc: nodeWithFinalRoot, walletAppliedEventId: '13',
    walletEnvelope: JSON.stringify({ state: hex(walletBytes), offset: 13, networkId: 'undeployed' }),
  }, h);
  assert.equal(acquired.manifest.metadata.firstFree, '6');
  assert.equal(acquired.manifest.metadata.finalZswapEventId, '13');
  assert.equal(acquired.manifest.metadata.nodeZswapRootHex, hex(nextRoot));
});

test('mismatched action, event, frontier, snapshot and version fail closed', async () => {
  const h = harness();
  await assert.rejects(acquire({ walletAppliedEventId: '13',
    walletEnvelope: JSON.stringify({ state: hex(walletBytes), offset: 13, networkId: 'undeployed' }) }, h), /ahead or behind/);
  await assert.rejects(acquire({ walletSnapshotBytes: Uint8Array.of(4, 1, 2) }, h), /wallet snapshot bytes differ/);
  await assert.rejects(acquire({ addressTaggedBytes: Buffer.from(address, 'hex') }, h), /tagged ContractAddress/);
  await assert.rejects(acquire({ addressTaggedBytes: Uint8Array.of(...taggedAddress.slice(0, -1), 0) }, h), /matching upstream tagged/);
  await assert.rejects(acquire({ indexerVersion: '4.0.2' }, h), /pairing is not approved/);
  h.indexed.transactions[0].endIndex = 6;
  await assert.rejects(acquire({}, h), /wallet firstFree differs/);
  h.indexed.transactions[0].endIndex = 5;
  h.indexed.transactions[0].contractActions[0].state = 'eeee';
  await assert.rejects(acquire({}, h), /expected action is absent/);
});

test('empty-event block searches checked ancestry, without using dynamic maxId as checkpoint', async () => {
  const h = harness();
  h.indexed.transactions[0].zswapLedgerEvents = [];
  const ancestor = {
    hash: parentHash, height: 4, protocolVersion: 22000,
    parent: { hash: '66'.repeat(32), height: 3 },
    transactions: [{ __typename: 'RegularTransaction', id: 6, hash: '77'.repeat(32),
      startIndex: 4, endIndex: 4, merkleTreeRoot: hex(root), contractActions: [],
      zswapLedgerEvents: [{ id: 12, maxId: 999, raw: '00' }] }],
  };
  h.indexer.getBlock = async hash => hash === blockHash ? h.indexed : ancestor;
  const acquired = await acquire({}, h);
  assert.equal(acquired.manifest.metadata.finalZswapEventId, '12');
});
