// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

// Submit a fresh Rust-exported counter deploy/call pair against a funded
// ledger-8 devnet wallet. The wallet owns fee balancing and submission.
import { mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { Buffer } from 'node:buffer';
import { execFile } from 'node:child_process';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { promisify } from 'node:util';
import * as ledger from '@midnight-ntwrk/ledger-v8';
import { setNetworkId } from '@midnight-ntwrk/midnight-js-network-id';
import { HDWallet, Roles } from '@midnight-ntwrk/wallet-sdk-hd';
import { WalletFacade } from '@midnight-ntwrk/wallet-sdk-facade';
import { ShieldedWallet } from '@midnight-ntwrk/wallet-sdk-shielded';
import { DustWallet } from '@midnight-ntwrk/wallet-sdk-dust-wallet';
import {
  createKeystore,
  InMemoryTransactionHistoryStorage,
  PublicKey,
  UnshieldedWallet,
} from '@midnight-ntwrk/wallet-sdk-unshielded-wallet';
import { firstValueFrom, filter, timeout } from 'rxjs';
import { WebSocket } from 'ws';
import { hasFinalizedCanonicalBlock, isExpectedAction, normalizeHash } from './provenance.mjs';

const [deployPath, callPath] = process.argv.slice(2);
const seedHex = process.env.COMPACT_RUST_WALLET_SEED_HEX;
const networkId = process.env.COMPACT_RUST_HANDOFF_NETWORK_ID;
const indexer = process.env.COMPACT_RUST_INDEXER_URL;
const node = process.env.COMPACT_RUST_NODE_URL;
const proofServer = process.env.COMPACT_RUST_PROOF_SERVER_URL;
const confirmedCallBuilder = process.env.COMPACT_RUST_CONFIRMED_CALL_BUILDER;
const counterArtifacts = process.env.COMPACT_RUST_COUNTER_ARTIFACTS;
if (Boolean(confirmedCallBuilder) !== Boolean(counterArtifacts)) {
  throw new Error('set both COMPACT_RUST_CONFIRMED_CALL_BUILDER and COMPACT_RUST_COUNTER_ARTIFACTS');
}
if (!deployPath || !callPath || !seedHex || !networkId || !indexer || !node || !proofServer) {
  throw new Error('usage: set COMPACT_RUST_WALLET_SEED_HEX, COMPACT_RUST_HANDOFF_NETWORK_ID, COMPACT_RUST_INDEXER_URL, COMPACT_RUST_NODE_URL and COMPACT_RUST_PROOF_SERVER_URL; then run node check.mjs <deploy.bin> <call.bin>');
}
if (!/^[0-9a-f]{64}$/i.test(seedHex)) {
  throw new Error('COMPACT_RUST_WALLET_SEED_HEX must be 32 bytes of hex');
}

setNetworkId(networkId);
globalThis.WebSocket = WebSocket;
const deploy = ledger.Transaction.deserialize('signature', 'proof', 'binding', await readFile(deployPath));
const call = ledger.Transaction.deserialize('signature', 'proof', 'binding', await readFile(callPath));
const deployAction = deploy.intents.get(1)?.actions[0];
const callAction = call.intents.get(1)?.actions[0];
const deployTtl = deploy.intents.get(1)?.ttl;
const callTtl = call.intents.get(1)?.ttl;
if (!(deployAction instanceof ledger.ContractDeploy) || !(callAction instanceof ledger.ContractCall) ||
    deployAction.address !== callAction.address) {
  throw new Error('expected a paired counter deployment and call at the same address');
}
if (!(deployTtl instanceof Date) || !(callTtl instanceof Date) ||
    deployTtl.getTime() !== callTtl.getTime() || deployTtl.getTime() <= Date.now() + 60_000) {
  throw new Error('deploy and call need the same future intent TTL');
}
const address = deployAction.address;
const execFileAsync = promisify(execFile);

async function waitForProofServer() {
  const deadline = Date.now() + 300_000;
  while (Date.now() < deadline) {
    try {
      await fetch(proofServer, { signal: AbortSignal.timeout(3_000) });
      return;
    } catch {
      await new Promise(resolve => setTimeout(resolve, 5_000));
    }
  }
  throw new Error(`proof server did not accept HTTP connections at ${proofServer}`);
}

const hd = HDWallet.fromSeed(Buffer.from(seedHex, 'hex'));
if (hd.type !== 'seedOk') throw new Error('wallet seed was rejected');
const derived = hd.hdWallet.selectAccount(0)
  .selectRoles([Roles.Zswap, Roles.NightExternal, Roles.Dust]).deriveKeysAt(0);
if (derived.type !== 'keysDerived') throw new Error('wallet key derivation failed');
hd.hdWallet.clear();
const shieldedSecretKeys = ledger.ZswapSecretKeys.fromSeed(derived.keys[Roles.Zswap]);
const dustSecretKey = ledger.DustSecretKey.fromSeed(derived.keys[Roles.Dust]);
const unshieldedKeystore = createKeystore(derived.keys[Roles.NightExternal], networkId);
const wallet = await WalletFacade.init({
  configuration: {
    networkId,
    indexerClientConnection: { indexerHttpUrl: indexer, indexerWsUrl: indexer.replace(/^http/, 'ws').replace('/api/v3/graphql', '/api/v3/graphql/ws') },
    provingServerUrl: new URL(proofServer),
    relayURL: new URL(node.replace(/^http/, 'ws')),
    costParameters: { additionalFeeOverhead: 300_000_000_000_000n, feeBlocksMargin: 5 },
    txHistoryStorage: new InMemoryTransactionHistoryStorage(),
  },
  shielded: cfg => ShieldedWallet(cfg).startWithSecretKeys(shieldedSecretKeys),
  unshielded: cfg => UnshieldedWallet(cfg).startWithPublicKey(PublicKey.fromKeyStore(unshieldedKeystore)),
  dust: cfg => DustWallet(cfg).startWithSecretKey(dustSecretKey, ledger.LedgerParameters.initialParameters().dust),
});

async function indexedAction() {
  const response = await fetch(indexer, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({
      query: 'query ($address: HexEncoded!) { contractAction(address: $address) { __typename address state transaction { hash block { height hash } } } }',
      variables: { address },
    }),
  });
  if (!response.ok) throw new Error(`indexer HTTP ${response.status}`);
  const body = await response.json();
  if (body.errors?.length) throw new Error(`indexer: ${JSON.stringify(body.errors)}`);
  return body.data.contractAction;
}

function assertCounterValue(action, expected) {
  const state = ledger.ContractState.deserialize(Buffer.from(action.state, 'hex'));
  const fields = state.data.state.asArray();
  if (fields?.length !== 1 || fields[0].type() !== 'cell') {
    throw new Error('indexed call has no single Counter field');
  }
  const cell = fields[0].asCell();
  const alignment = cell.alignment[0];
  const bytes = cell.value[0];
  if (cell.alignment.length !== 1 || alignment?.tag !== 'atom' ||
      alignment.value.tag !== 'bytes' || alignment.value.length !== 8 ||
      cell.value.length !== 1 || !(bytes instanceof Uint8Array) ||
      bytes.length === 0 || bytes.length > 8) {
    throw new Error('indexed call has an invalid Counter cell');
  }
  const actual = bytes.reduce(
    (value, byte, index) => value + (BigInt(byte) << BigInt(index * 8)), 0n,
  );
  if (actual !== BigInt(expected)) {
    throw new Error(`indexed Counter is ${actual}, expected ${expected}`);
  }
}

async function nodeRpc(method, params) {
  const response = await fetch(node, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({ jsonrpc: '2.0', id: 1, method, params }),
  });
  if (!response.ok) throw new Error(`node RPC HTTP ${response.status}`);
  const body = await response.json();
  if (body.error) throw new Error(`node RPC ${method}: ${JSON.stringify(body.error)}`);
  return body.result;
}

async function waitForAction(type, submittedHash) {
  const deadline = Date.now() + 180_000;
  while (Date.now() < deadline) {
    const action = await indexedAction();
    if (isExpectedAction(action, type, address, submittedHash) &&
        await hasFinalizedCanonicalBlock(action.transaction.block, nodeRpc)) return action;
    await new Promise(resolve => setTimeout(resolve, 3_000));
  }
  throw new Error(`timed out waiting for finalized indexed ${type} ${submittedHash} at ${address}`);
}

async function submit(tx, label) {
  const deadline = Date.now() + 120_000;
  let recipe;
  while (!recipe) {
    try {
      recipe = await wallet.balanceFinalizedTransaction(
        tx, { shieldedSecretKeys, dustSecretKey }, { ttl: new Date(Date.now() + 20 * 60_000) },
      );
    } catch (error) {
      if (!String(error).includes('Insufficient Funds: could not balance dust') || Date.now() >= deadline) {
        throw error;
      }
      console.log(`${label}: waiting for DUST to become spendable`);
      await new Promise(resolve => setTimeout(resolve, 5_000));
    }
  }
  const finalized = await wallet.finalizeRecipe(recipe);
  const hash = normalizeHash(finalized.transactionHash());
  const identifier = await wallet.submitTransaction(finalized);
  console.log(`${label} submitted: ${identifier}, final transaction hash ${hash}`);
  return hash;
}

await waitForProofServer();
try {
  await wallet.start(shieldedSecretKeys, dustSecretKey);
  let state = await firstValueFrom(wallet.state().pipe(
    filter(current => current.isSynced &&
      ((current.unshielded?.balances[ledger.nativeToken().raw] ?? 0n) +
       (current.shielded?.balances[ledger.nativeToken().raw] ?? 0n)) > 0n),
    timeout({ first: 180_000 }),
  ));
  console.log(`wallet synced: NIGHT ${state.unshielded?.balances[ledger.nativeToken().raw] ?? 0n}, DUST ${state.dust?.balance(new Date()) ?? 0n}`);
  if ((state.dust?.balance(new Date()) ?? 0n) <= 0n) {
    const coins = state.unshielded?.availableCoins.filter(
      coin => coin.meta.registeredForDustGeneration === false,
    ) ?? [];
    if (coins.length === 0) throw new Error('wallet has no DUST or unregistered NIGHT for fees');
    const recipe = await wallet.registerNightUtxosForDustGeneration(
      coins,
      unshieldedKeystore.getPublicKey(),
      payload => unshieldedKeystore.signData(payload),
    );
    const registration = await wallet.finalizeRecipe(recipe);
    console.log(`DUST registration submitted: ${await wallet.submitTransaction(registration)}`);
    state = await firstValueFrom(wallet.state().pipe(
      filter(current => current.isSynced && (current.dust?.balance(new Date()) ?? 0n) > 0n),
      timeout({ first: 180_000 }),
    ));
  }
  const existing = await indexedAction();
  if (existing) throw new Error(`contract ${address} already exists; export a fresh pair on a new devnet`);
  const deployedHash = await submit(deploy, 'deployment');
  const deployed = await waitForAction('ContractDeploy', deployedHash);
  console.log(`deployment indexed: block ${deployed.transaction.block.height}, address ${address}`);
  const calledHash = await submit(call, 'call');
  const called = await waitForAction('ContractCall', calledHash);
  assertCounterValue(called, 1);
  console.log(`call indexed: block ${called.transaction.block.height}, address ${address}`);
  console.log('indexed contract state: round = 1');
  if (confirmedCallBuilder) {
    const workspace = await mkdtemp(join(tmpdir(), 'compact-rust-confirmed-call-'));
    try {
      const statePath = join(workspace, 'state.bin');
      const outputPath = join(workspace, 'second-call.bin');
      await writeFile(statePath, Buffer.from(called.state, 'hex'));
      const ttl = Math.floor(Date.now() / 1000) + 45 * 60;
      const result = await execFileAsync(confirmedCallBuilder, [
        statePath, deployPath, counterArtifacts, outputPath, networkId, String(ttl), address,
      ], { timeout: 300_000 });
      console.log(result.stdout.trim());
      const secondCall = ledger.Transaction.deserialize(
        'signature', 'proof', 'binding', await readFile(outputPath),
      );
      const secondAction = secondCall.intents.get(1)?.actions[0];
      if (!(secondAction instanceof ledger.ContractCall) || secondAction.address !== address) {
        throw new Error('confirmed-state builder produced a call for the wrong contract');
      }
      const secondHash = await submit(secondCall, 'confirmed-state call');
      const second = await waitForAction('ContractCall', secondHash);
      assertCounterValue(second, 2);
      console.log(`confirmed-state call indexed: block ${second.transaction.block.height}, address ${address}`);
      console.log('indexed contract state: round = 2');
    } finally {
      await rm(workspace, { recursive: true, force: true });
    }
  }
} finally {
  await wallet.stop();
}
