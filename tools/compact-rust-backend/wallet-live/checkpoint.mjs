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

// A trusted-endpoint checkpoint adapter, not a consensus proof or ledger replay.
import { createHash } from 'node:crypto';
import { lstat, mkdir, writeFile } from 'node:fs/promises';
import { join, resolve } from 'node:path';
import { hasFinalizedCanonicalBlock, normalizeHash } from './provenance.mjs';
import { decodeScaleResultBytes, decodeScaleString, scaleBytes, scaleVec } from './checkpoint-scale.mjs';

const BLOCK_QUERY = `query ($offset: BlockOffset) {
  block(offset: $offset) {
    hash height protocolVersion parent { hash height }
    transactions {
      __typename id hash protocolVersion
      zswapLedgerEvents { id maxId raw protocolVersion }
      contractActions { __typename address state zswapState ... on ContractCall { entryPoint } }
      ... on RegularTransaction { startIndex endIndex merkleTreeRoot }
    }
  }
}`;
const CONTRACT_ADDRESS_TAG = Buffer.from('midnight:contract-address[v2]:', 'utf8');
export const CANDIDATE_COMPATIBILITY_PROFILE = Object.freeze({
  compatibilityProfile: 'node-0.22.3/indexer-4.0.1/ledger-8.0.2/codec-8.0.3',
  compatibilityStatus: 'candidate-live-validation-pending',
  nodeSoftwareVersion: '0.22.3-6f0ef437',
  indexerVersion: '4.0.1',
  nodeLedgerConstraint: '=8.0.2',
  ledgerVersion: '8.0.3',
});

const hash = bytes => createHash('sha256').update(bytes).digest('hex');
const decimal = (value, label) => {
  if ((typeof value !== 'string' && typeof value !== 'number' && typeof value !== 'bigint') ||
      !/^(0|[1-9][0-9]*)$/.test(String(value))) throw new Error(`${label} must be a decimal integer`);
  const n = BigInt(value);
  if (n > BigInt(Number.MAX_SAFE_INTEGER)) throw new Error(`${label} exceeds safe GraphQL range`);
  return n;
};
const requiredBytes = (value, label) => {
  if (!(value instanceof Uint8Array) || value.length === 0) throw new Error(`${label} must be nonempty bytes`);
  return Uint8Array.from(value);
};
const indexedHex = (value, label) => {
  if (typeof value !== 'string' || !/^(?:0x)?(?:[0-9a-f]{2})+$/i.test(value)) {
    throw new Error(`${label} must be nonempty hex`);
  }
  return Buffer.from(value.replace(/^0x/i, ''), 'hex');
};
const sameBytes = (left, right) => Buffer.from(left).equals(Buffer.from(right));

export function createIndexerClient(url, fetchImpl = fetch, { requestTimeoutMs = 30_000 } = {}) {
  if (typeof url !== 'string' || !/^https?:\/\//.test(url)) throw new Error('indexer URL is required');
  if (!Number.isSafeInteger(requestTimeoutMs) || requestTimeoutMs <= 0) {
    throw new Error('indexer request timeout must be a positive integer');
  }
  return {
    async getBlock(blockHash) {
      const response = await fetchImpl(url, {
        method: 'POST', headers: { 'content-type': 'application/json' },
        body: JSON.stringify({ query: BLOCK_QUERY, variables: { offset: { hash: normalizeHash(blockHash) } } }),
        signal: AbortSignal.timeout(requestTimeoutMs),
      });
      if (!response.ok) throw new Error(`indexer HTTP ${response.status}`);
      const body = await response.json();
      if (body.errors?.length || !body.data?.block) throw new Error('indexer block query failed');
      return body.data.block;
    },
  };
}

async function nodeCall(rpc, method, input, blockHash) {
  const result = await rpc('state_call', [method, `0x${Buffer.from(input).toString('hex')}`, `0x${blockHash}`]);
  return scaleBytes(result, `${method} response`);
}

/** A pinned candidate pairing, not a claim of tested live compatibility. */
export async function assertNodeProfile(rpc, {
  indexerVersion, ledgerVersion = CANDIDATE_COMPATIBILITY_PROFILE.ledgerVersion,
  blockHash,
} = {}) {
  if (typeof rpc !== 'function') throw new Error('node RPC is required');
  const at = normalizeHash(blockHash ?? await rpc('chain_getFinalizedHead', []));
  const [software, versionRaw] = await Promise.all([
    rpc('system_version', []),
    nodeCall(rpc, 'MidnightRuntimeApi_get_ledger_version', new Uint8Array(), at),
  ]);
  const nodeLedgerConstraint = decodeScaleString(versionRaw);
  const approved = CANDIDATE_COMPATIBILITY_PROFILE;
  if (software !== approved.nodeSoftwareVersion ||
      indexerVersion !== approved.indexerVersion ||
      nodeLedgerConstraint !== approved.nodeLedgerConstraint ||
      ledgerVersion !== approved.ledgerVersion) {
    throw new Error(`node/indexer/codec pairing is not approved: ${software}, ${indexerVersion}, ${nodeLedgerConstraint}, ${ledgerVersion}`);
  }
  return { ...approved, blockHash: at };
}

async function previousFact(indexer, firstBlock, select, label) {
  let current = firstBlock;
  const visited = new Set();
  for (let depth = 0; depth < 512; depth++) {
    const blockHash = normalizeHash(current.hash);
    if (visited.has(blockHash)) throw new Error('indexer ancestry cycle');
    visited.add(blockHash);
    const value = select(current);
    if (value !== undefined) return value;
    if (!current.parent) throw new Error(`no historical ${label} before genesis`);
    const parentHash = normalizeHash(current.parent.hash);
    const parent = await indexer.getBlock(parentHash);
    if (normalizeHash(parent.hash) !== parentHash ||
        decimal(parent.height, 'parent height') + 1n !== decimal(current.height, 'block height')) {
      throw new Error('indexer ancestry differs from its parent link');
    }
    current = parent;
  }
  throw new Error(`historical ${label} exceeds bounded ancestry search`);
}

function inspectTransactions(block) {
  if (!Array.isArray(block.transactions)) throw new Error('indexer omitted block transactions');
  let previousTx = -1n;
  let previousEvent = -1n;
  let lastRegular;
  let lastEvent;
  for (const tx of block.transactions) {
    const txId = decimal(tx.id, 'transaction ID');
    if (txId <= previousTx) throw new Error('indexer transaction order is not strict');
    previousTx = txId;
    if (tx.__typename === 'RegularTransaction') {
      const start = decimal(tx.startIndex, 'startIndex');
      const end = decimal(tx.endIndex, 'endIndex');
      if (end < start) throw new Error('indexer endIndex precedes startIndex');
      if (lastRegular && start !== lastRegular.end) throw new Error('indexer transaction frontiers are discontinuous');
      lastRegular = { start, end, root: indexedHex(tx.merkleTreeRoot, 'transaction root'), tx };
    }
    if (!Array.isArray(tx.zswapLedgerEvents)) throw new Error('indexer omitted Zswap events');
    for (const event of tx.zswapLedgerEvents) {
      const id = decimal(event.id, 'Zswap event ID');
      if (id <= previousEvent) throw new Error('indexer Zswap event order is not strict');
      if (decimal(event.maxId, 'Zswap maxId') < id) throw new Error('indexer event maxId below observed ID');
      previousEvent = id;
      lastEvent = { id, txHash: normalizeHash(tx.hash) };
    }
  }
  return { lastRegular, lastEvent };
}

function parseWalletEnvelope(envelope, walletBytes, appliedEventId, networkId) {
  const value = typeof envelope === 'string' ? JSON.parse(envelope) : envelope;
  if (!value || typeof value !== 'object') throw new Error('missing wallet SDK snapshot envelope');
  if (value.networkId !== networkId) throw new Error('wallet snapshot network mismatch');
  if (decimal(value.offset, 'wallet envelope offset') !== appliedEventId) {
    throw new Error('wallet envelope event offset mismatch');
  }
  if (!sameBytes(indexedHex(value.state, 'wallet envelope state'), walletBytes)) {
    throw new Error('wallet snapshot bytes differ from the SDK envelope');
  }
}

/**
 * Acquire one block-bound checkpoint. The installed ledger-v8 JS package does
 * not expose Zswap tree roots; the paired Rust builder MUST decode the exact
 * returned binary refs and rehash both roots before constructing runtime parts.
 */
export async function acquireVerifiedCheckpoint({
  rpc, indexer, ledger, expectedAction, addressHex, addressTaggedBytes,
  walletEnvelope, walletSnapshotBytes, walletAppliedEventId, networkId,
  ledgerVersion = '8.0.3', indexerVersion,
}) {
  if (typeof rpc !== 'function' || typeof indexer?.getBlock !== 'function') {
    throw new Error('node RPC and indexed block reader are required');
  }
  if (!ledger?.ContractState?.deserialize || !ledger?.ZswapChainState?.deserialize ||
      !ledger?.ZswapLocalState?.deserialize) throw new Error('pinned ledger-v8 decoders are required');
  const address = normalizeHash(addressHex);
  const action = expectedAction;
  if (!action || normalizeHash(action.address) !== address || !action.transaction?.block) {
    throw new Error('expected indexed contract action/address/block is required');
  }
  const blockHash = normalizeHash(action.transaction.block.hash);
  const blockHeight = decimal(action.transaction.block.height, 'action block height');
  if (!await hasFinalizedCanonicalBlock(action.transaction.block, rpc)) {
    throw new Error('action block is not finalized');
  }
  const nodeProfile = await assertNodeProfile(rpc, { indexerVersion, ledgerVersion, blockHash });
  const block = await indexer.getBlock(blockHash);
  if (normalizeHash(block.hash) !== blockHash || decimal(block.height, 'indexed block height') !== blockHeight) {
    throw new Error('indexed action and block identity differ');
  }
  const header = await rpc('chain_getHeader', [`0x${blockHash}`]);
  if (normalizeHash(header?.parentHash) !== normalizeHash(block.parent?.hash)) {
    throw new Error('node and indexer parent block differ');
  }
  const taggedAddress = requiredBytes(addressTaggedBytes, 'tagged ContractAddress');
  const expectedTaggedAddress = Buffer.concat([CONTRACT_ADDRESS_TAG, indexedHex(address, 'address')]);
  if (!sameBytes(taggedAddress, expectedTaggedAddress)) {
    throw new Error('node runtime API requires the matching upstream tagged ContractAddress');
  }
  const walletBytes = requiredBytes(walletSnapshotBytes, 'wallet snapshot');
  const appliedEventId = decimal(walletAppliedEventId, 'wallet applied event ID');
  parseWalletEnvelope(walletEnvelope, walletBytes, appliedEventId, networkId);
  // The node Bridge decodes the inner address with untagged Deserializable.
  // The private checkpoint still retains the actual tagged builder reference.
  const callAddress = scaleVec(indexedHex(address, 'address'));
  const [contractRaw, treeRaw, rootRaw, networkRaw] = await Promise.all([
    nodeCall(rpc, 'MidnightRuntimeApi_get_contract_state', callAddress, blockHash),
    nodeCall(rpc, 'MidnightRuntimeApi_get_zswap_chain_state', callAddress, blockHash),
    nodeCall(rpc, 'MidnightRuntimeApi_get_zswap_state_root', new Uint8Array(), blockHash),
    nodeCall(rpc, 'MidnightRuntimeApi_get_network_id', new Uint8Array(), blockHash),
  ]);
  const nodeNetwork = decodeScaleString(networkRaw);
  if (nodeNetwork !== networkId) throw new Error('node network differs from wallet network');
  const contractBytes = decodeScaleResultBytes(contractRaw);
  const treeBytes = decodeScaleResultBytes(treeRaw);
  const rootBytes = decodeScaleResultBytes(rootRaw);
  if (!contractBytes.length || !treeBytes.length || !rootBytes.length) {
    throw new Error('node returned an empty contract/tree/root');
  }
  ledger.ContractState.deserialize(contractBytes);
  const tree = ledger.ZswapChainState.deserialize(treeBytes);
  ledger.ZswapLocalState.deserialize(walletBytes);
  const transactions = inspectTransactions(block);
  const transactionHash = normalizeHash(action.transaction.hash);
  const matchingTx = block.transactions.find(tx => normalizeHash(tx.hash) === transactionHash);
  if (!matchingTx || !matchingTx.contractActions?.some(candidate =>
    candidate.__typename === action.__typename && normalizeHash(candidate.address) === address &&
    candidate.entryPoint === action.entryPoint &&
    sameBytes(indexedHex(candidate.state, 'indexed contract state'), contractBytes))) {
    throw new Error('expected action is absent or differs from node contract state');
  }
  if (!sameBytes(indexedHex(action.state, 'action contract state'), contractBytes)) {
    throw new Error('indexed action contract state is stale at selected block');
  }
  const finalRegular = transactions.lastRegular ?? await previousFact(
    indexer, block, candidate => inspectTransactions(candidate).lastRegular, 'regular frontier');
  if (!sameBytes(finalRegular.root, rootBytes)) throw new Error('node/indexer final Zswap root mismatch');
  const finalEvent = transactions.lastEvent ?? await previousFact(
    indexer, block, candidate => inspectTransactions(candidate).lastEvent, 'Zswap event');
  if (finalEvent.id !== appliedEventId) throw new Error('wallet is ahead or behind the final Zswap event');
  if (BigInt(tree.firstFree) > finalRegular.end) {
    throw new Error('filtered contract tree frontier exceeds global indexer frontier');
  }
  const firstFree = BigInt(ledger.ZswapLocalState.deserialize(walletBytes).firstFree);
  if (firstFree !== finalRegular.end) throw new Error('wallet firstFree differs from indexer endIndex');
  const metadata = {
    networkId, ledgerVersion, addressHex: address, transactionHash,
    blockHash, blockHeight: String(blockHeight), parentBlockHash: normalizeHash(block.parent.hash),
    nodeZswapRootHex: Buffer.from(rootBytes).toString('hex'), firstFree: String(firstFree),
    finalZswapEventId: String(finalEvent.id),
    compatibilityProfile: nodeProfile.compatibilityProfile,
    compatibilityStatus: nodeProfile.compatibilityStatus,
    nodeSoftwareVersion: nodeProfile.nodeSoftwareVersion,
    indexerVersion: nodeProfile.indexerVersion,
    nodeLedgerConstraint: nodeProfile.nodeLedgerConstraint,
    protocolVersion: String(block.protocolVersion),
  };
  const wallet = {
    blockHash, blockHeight: String(blockHeight), networkId, ledgerVersion,
    appliedEventId: String(appliedEventId), walletStateSha256: hash(walletBytes),
  };
  const binaries = {
    contractAddress: taggedAddress, nodeContract: Uint8Array.from(contractBytes),
    contractTree: Uint8Array.from(treeBytes), walletState: walletBytes,
  };
  return { manifest: { format: 'compact-shielded-live/v1', kind: 'checkpoint', metadata, wallet }, binaries };
}

export async function writeCheckpoint(directory, acquired) {
  const checkpointDirectory = resolve(directory);
  await mkdir(checkpointDirectory, { mode: 0o700 });
  const directoryState = await lstat(checkpointDirectory);
  if (!directoryState.isDirectory() || (directoryState.mode & 0o077) !== 0) {
    throw new Error('checkpoint directory must be private (0700)');
  }
  const files = {};
  for (const [name, bytes] of Object.entries(acquired.binaries)) {
    const filename = `${name}.bin`;
    const path = join(checkpointDirectory, filename);
    await writeFile(path, bytes, { flag: 'wx', mode: 0o600 });
    files[name] = { path, sha256: hash(bytes), bytes: bytes.length,
      encoding: name === 'contractAddress' ? 'ContractAddress' :
        name === 'nodeContract' ? 'ContractState' :
          name === 'contractTree' ? 'ZswapChainState' : 'ZswapLocalState' };
  }
  const manifest = { ...acquired.manifest, files };
  const manifestPath = join(checkpointDirectory, 'checkpoint.json');
  await writeFile(manifestPath, JSON.stringify(manifest, null, 2) + '\n', { flag: 'wx', mode: 0o600 });
  return { manifestPath, manifest };
}
