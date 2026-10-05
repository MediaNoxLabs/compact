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

// ADR217: the live runner owns transport and wallet transitions, never ledger state.
import assert from "node:assert/strict";
import { randomBytes } from "node:crypto";
import { execFile } from "node:child_process";
import { readFile, mkdir, lstat } from "node:fs/promises";
import { resolve, join, isAbsolute } from "node:path";
import { pathToFileURL } from "node:url";
import { promisify } from "node:util";
import {
  normalizeHash,
  hasFinalizedCanonicalBlock,
  isExpectedAction,
} from "./provenance.mjs";
import {
  sha256,
  writePrivateHandoff,
  shieldedOfferFingerprint,
  assertShieldedOffersUnchanged,
} from "./shielded-handoff.mjs";

const FORMAT = "compact-shielded-live/v1";
const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
const jsonBytes = (value) => Buffer.from(`${JSON.stringify(value, null, 2)}\n`);
export function requireIsolatedEndpoints(node, indexer, proofServer) {
  for (const [value, port] of [
    [node, "49944"],
    [indexer, "48088"],
    [proofServer, "46300"],
  ]) {
    const url = new URL(value);
    assert(
      ["127.0.0.1", "localhost"].includes(url.hostname) &&
        url.protocol === "http:" &&
        url.port === port,
      "ADR217 accepts only its isolated local service ports",
    );
  }
}

export async function privateReference(path, bytes, encoding) {
  return {
    path: resolve(path),
    ...(await writePrivateHandoff(path, bytes)),
    encoding,
  };
}
export async function readReference(ref, encoding) {
  assert(ref && isAbsolute(ref.path), "handoff path must be absolute");
  assert.equal(ref.encoding, encoding, "handoff encoding mismatch");
  const info = await lstat(ref.path);
  assert(info.isFile(), "handoff must be a regular file");
  assert.equal(info.mode & 0o077, 0, "handoff must be private");
  const bytes = await readFile(ref.path);
  assert.equal(bytes.length, ref.bytes, "handoff length mismatch");
  assert.equal(sha256(bytes), ref.sha256, "handoff digest mismatch");
  return bytes;
}
export function coinJson(coin, commitment) {
  return {
    nonceHex: coin.nonce,
    colorHex: coin.type,
    value: String(coin.value),
    mtIndex: String(coin.mt_index),
    ...(commitment ? { commitmentHex: commitment } : {}),
  };
}
export function selectExactCoin(available, expected) {
  const sameType = available.filter(
    (row) => row.coin.type === expected.colorHex,
  );
  assert.equal(
    sameType.length,
    1,
    "expected exactly one available coin of the selected token",
  );
  const selected = sameType[0];
  assert.equal(
    selected.coin.nonce,
    expected.nonceHex,
    "wallet coin nonce mismatch",
  );
  assert.equal(
    String(selected.coin.value),
    expected.value,
    "wallet coin value mismatch",
  );
  assert.equal(
    String(selected.coin.mt_index),
    expected.mtIndex,
    "wallet coin allocation mismatch",
  );
  if (expected.commitmentHex)
    assert.equal(
      selected.commitment,
      expected.commitmentHex,
      "wallet commitment mismatch",
    );
  return selected;
}
export function selectedReservationInput(transaction, selected) {
  assert.equal(
    transaction.fallibleOffer?.size ?? 0,
    0,
    "reservation has fallible offers",
  );
  assert.equal(
    transaction.intents?.size ?? 0,
    0,
    "reservation has contract or fee intents",
  );
  const offer = transaction.guaranteedOffer;
  assert(offer, "reservation has no guaranteed offer");
  assert.equal(offer.inputs.length, 1, "reservation must have one exact input");
  assert.equal(
    offer.outputs.length,
    0,
    "reservation must have no change output",
  );
  assert.equal(
    offer.transients.length,
    0,
    "reservation must have no transient",
  );
  assert.equal(
    offer.inputs[0].contractAddress,
    undefined,
    "reservation input must be user owned",
  );
  assert.equal(
    offer.inputs[0].nullifier,
    selected.nullifier,
    "reservation selected a different coin",
  );
  assert.deepEqual(
    [...offer.deltas],
    [[selected.coin.type, selected.coin.value]],
    "reservation value mismatch",
  );
  return offer.inputs[0];
}
export function verifyResult(
  result,
  action,
  networkId,
  transaction,
  ledger,
  address,
) {
  assert.equal(result.format, FORMAT);
  assert.equal(result.kind, "action-result");
  assert.equal(result.action, action);
  assert.equal(result.networkId, networkId);
  if (address)
    assert.equal(result.addressHex, address, "builder address changed");
  assert.deepEqual(
    shieldedOfferFingerprint(transaction),
    result.offerFingerprint,
    "builder offer fingerprint mismatch",
  );
  assert.equal(
    result.offerFingerprint.fallible.length,
    0,
    "acceptance uses guaranteed offers only",
  );
  const actions = [...transaction.intents.values()].flatMap(
    (intent) => intent.actions,
  );
  assert.equal(actions.length, 1, "builder must produce one contract action");
  const call = actions[0];
  assert.equal(call.address, result.addressHex, "transaction address mismatch");
  if (action === "deploy") assert(call instanceof ledger.ContractDeploy);
  else {
    assert(call instanceof ledger.ContractCall);
    const entryPoint =
      typeof call.entryPoint === "string"
        ? call.entryPoint
        : Buffer.from(call.entryPoint).toString("utf8");
    assert.equal(entryPoint, action, "transaction entry point mismatch");
  }
  return result;
}

// Dependencies are explicit so negative guards run without a wallet or service.
export async function finalizePreservingOffers(wallet, transaction, keys, ttl) {
  const before = shieldedOfferFingerprint(transaction);
  const recipe = await wallet.balanceFinalizedTransaction(transaction, keys, {
    ttl,
    tokenKindsToBalance: ["dust"],
  });
  try {
    assert.deepEqual(
      shieldedOfferFingerprint(recipe.originalTransaction),
      before,
      "balancing changed original offer",
    );
    assert.deepEqual(
      shieldedOfferFingerprint(recipe.balancingTransaction),
      { guaranteed: null, fallible: [] },
      "fee transaction contains a shielded offer",
    );
    const finalized = await wallet.finalizeRecipe(recipe);
    assertShieldedOffersUnchanged(transaction, finalized);
    assert.deepEqual(
      shieldedOfferFingerprint(finalized),
      before,
      "mutable offer changed during finalization",
    );
    return finalized;
  } catch (error) {
    await wallet.revert(recipe);
    throw error;
  }
}

async function main() {
  const env = process.env;
  const required = (name) => {
    assert(env[name], `missing ${name}`);
    return env[name];
  };
  const seedHex = required("COMPACT_RUST_WALLET_SEED_HEX");
  assert(/^[a-f0-9]{64}$/i.test(seedHex), "wallet seed must be 32 bytes");
  const networkId = required("COMPACT_RUST_HANDOFF_NETWORK_ID");
  const node = required("COMPACT_RUST_NODE_URL");
  const indexerUrl = required("COMPACT_RUST_INDEXER_URL");
  const proofServer = required("COMPACT_RUST_PROOF_SERVER_URL");
  requireIsolatedEndpoints(node, indexerUrl, proofServer);
  const builder = resolve(required("COMPACT_RUST_SHIELDED_BUILDER"));
  const artifactsRoot = resolve(required("COMPACT_RUST_SHIELDED_ARTIFACTS"));
  const directory = resolve(required("COMPACT_RUST_SHIELDED_RUN_DIR"));
  await mkdir(directory, { mode: 0o700 }); // A run never reuses a previous directory.
  const ledger = await import("@midnight-ntwrk/ledger-v8");
  const { setNetworkId } =
    await import("@midnight-ntwrk/midnight-js-network-id");
  const { HDWallet, Roles } = await import("@midnight-ntwrk/wallet-sdk-hd");
  const { WalletFacade } = await import("@midnight-ntwrk/wallet-sdk-facade");
  const { ShieldedWallet } =
    await import("@midnight-ntwrk/wallet-sdk-shielded");
  const { DustWallet } = await import("@midnight-ntwrk/wallet-sdk-dust-wallet");
  const {
    createKeystore,
    InMemoryTransactionHistoryStorage,
    PublicKey,
    UnshieldedWallet,
  } = await import("@midnight-ntwrk/wallet-sdk-unshielded-wallet");
  const { firstValueFrom, filter, timeout } = await import("rxjs");
  const { WebSocket } = await import("ws");
  const { acquireVerifiedCheckpoint, createIndexerClient, writeCheckpoint } =
    await import("./checkpoint.mjs");
  const { scaleBytes, decodeScaleString } =
    await import("./checkpoint-scale.mjs");
  setNetworkId(networkId);
  globalThis.WebSocket = WebSocket;
  const hd = HDWallet.fromSeed(Buffer.from(seedHex, "hex"));
  assert.equal(hd.type, "seedOk");
  const derived = hd.hdWallet
    .selectAccount(0)
    .selectRoles([Roles.Zswap, Roles.NightExternal, Roles.Dust])
    .deriveKeysAt(0);
  assert.equal(derived.type, "keysDerived");
  hd.hdWallet.clear();
  const shieldedSecretKeys = ledger.ZswapSecretKeys.fromSeed(
    derived.keys[Roles.Zswap],
  );
  const dustSecretKey = ledger.DustSecretKey.fromSeed(derived.keys[Roles.Dust]);
  const unshieldedKeystore = createKeystore(
    derived.keys[Roles.NightExternal],
    networkId,
  );
  const wallet = await WalletFacade.init({
    configuration: {
      networkId,
      indexerClientConnection: {
        indexerHttpUrl: indexerUrl,
        indexerWsUrl: indexerUrl
          .replace(/^http/, "ws")
          .replace("/api/v3/graphql", "/api/v3/graphql/ws"),
      },
      provingServerUrl: new URL(proofServer),
      relayURL: new URL(node.replace(/^http/, "ws")),
      costParameters: {
        additionalFeeOverhead: 300_000_000_000_000n,
        feeBlocksMargin: 5,
      },
      txHistoryStorage: new InMemoryTransactionHistoryStorage(),
    },
    shielded: (cfg) =>
      ShieldedWallet(cfg).startWithSecretKeys(shieldedSecretKeys),
    unshielded: (cfg) =>
      UnshieldedWallet(cfg).startWithPublicKey(
        PublicKey.fromKeyStore(unshieldedKeystore),
      ),
    dust: (cfg) =>
      DustWallet(cfg).startWithSecretKey(
        dustSecretKey,
        ledger.LedgerParameters.initialParameters().dust,
      ),
  });
  const keys = { shieldedSecretKeys, dustSecretKey };
  const waitState = (predicate) =>
    firstValueFrom(
      wallet.state().pipe(
        filter((state) => state.isSynced && predicate(state)),
        timeout({ first: 300_000 }),
      ),
    );
  const rpc = async (method, params) => {
    const response = await fetch(node, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ jsonrpc: "2.0", id: 1, method, params }),
      signal: AbortSignal.timeout(30_000),
    });
    assert(response.ok, `node HTTP ${response.status}`);
    const body = await response.json();
    if (body.error)
      throw new Error(`node RPC ${method}: ${JSON.stringify(body.error)}`);
    return body.result;
  };
  let address;
  let taggedAddress;
  let lastCheckpoint;
  let reserved;
  let submitted = false;
  const receipt = {
    format: FORMAT,
    kind: "lifecycle-receipt",
    networkId,
    status: "running",
    phases: [],
    provenance: {
      builderSha256: sha256(await readFile(builder)),
      runnerSha256: sha256(await readFile(new URL(import.meta.url))),
      packageLockSha256: sha256(
        await readFile(new URL("./package-lock.json", import.meta.url)),
      ),
      sourceSha256: sha256(
        await readFile(new URL("./shielded.compact", import.meta.url)),
      ),
      endpoints: { node, indexer: indexerUrl, proofServer },
    },
  };
  const execute = promisify(execFile);
  let checkpointAttempt = 0;
  async function actionRequest(action, fields = {}) {
    const request = {
      format: FORMAT,
      kind: "action-request",
      networkId,
      ttlEpochSeconds: String(Math.floor(Date.now() / 1000) + 2700),
      artifactsRoot,
      ...(lastCheckpoint ? { checkpoint: lastCheckpoint } : {}),
      ...fields,
    };
    const path = join(directory, `${action}-request.json`);
    await privateReference(path, jsonBytes(request), "JSON");
    const out = join(directory, `${action}-builder`);
    try {
      await execute(builder, [action, path, out], {
        timeout: 1_200_000,
        maxBuffer: 1024 * 1024,
      });
    } catch (cause) {
      await privateReference(
        join(directory, `${action}-builder-error.json`),
        jsonBytes({
          code: cause.code,
          signal: cause.signal,
          stdout: cause.stdout,
          stderr: cause.stderr,
        }),
        "JSON",
      );
      throw new Error(
        `Rust ${action} builder failed; private request retained`,
        { cause },
      );
    }
    const result = JSON.parse(await readFile(join(out, "result.json"), "utf8"));
    const bytes = await readReference(
      result.transaction,
      "Transaction<Signature,Proof,Binding>",
    );
    const tx = ledger.Transaction.deserialize(
      "signature",
      "proof",
      "binding",
      bytes,
    );
    verifyResult(result, action, networkId, tx, ledger, address);
    if (result.guaranteedOffer) {
      const offerBytes = await readReference(
        result.guaranteedOffer,
        "Offer<Proof>",
      );
      assert.equal(sha256(offerBytes), result.offerFingerprint.guaranteed);
    } else assert.equal(result.offerFingerprint.guaranteed, null);
    if (action === "deploy") {
      address = result.addressHex;
      taggedAddress = await readReference(
        result.contractAddress,
        "ContractAddress",
      );
    } else assert.equal(result.checkpointSha256, lastCheckpoint.sha256);
    return { result, tx };
  }
  async function indexedAction() {
    const response = await fetch(indexerUrl, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({
        query:
          "query ($address: HexEncoded!) { contractAction(address: $address) { __typename address state zswapState ... on ContractCall { entryPoint } transaction { hash block { height hash } } } }",
        variables: { address },
      }),
      signal: AbortSignal.timeout(30_000),
    });
    assert(response.ok, `indexer HTTP ${response.status}`);
    const body = await response.json();
    assert(
      !body.errors?.length,
      `indexer query failed: ${JSON.stringify(body.errors)}`,
    );
    return body.data.contractAction;
  }
  async function confirmed(action, transactionHash) {
    const type = action === "deploy" ? "ContractDeploy" : "ContractCall";
    const deadline = Date.now() + 300_000;
    while (Date.now() < deadline) {
      const observed = await indexedAction();
      if (
        isExpectedAction(observed, type, address, transactionHash) &&
        (action === "deploy" || observed.entryPoint === action) &&
        (await hasFinalizedCanonicalBlock(observed.transaction.block, rpc))
      )
        return observed;
      await sleep(2000);
    }
    throw new Error(`finalized ${action} was not indexed`);
  }
  async function checkpoint(expectedAction) {
    const deadline = Date.now() + 180_000;
    let lastError;
    do {
      const state = await waitState((state) => Boolean(state.shielded));
      const snapshot = Buffer.from(state.shielded.serialize());
      const envelope = JSON.parse(snapshot.toString());
      try {
        const acquired = await acquireVerifiedCheckpoint({
          rpc,
          indexer: createIndexerClient(indexerUrl),
          ledger,
          expectedAction,
          addressHex: address,
          addressTaggedBytes: taggedAddress,
          walletEnvelope: envelope,
          walletAppliedEventId: envelope.offset,
          expectedNodeLedgerConstraint: "=8.0.3",
          walletSnapshotBytes: Buffer.from(envelope.state, "hex"),
          networkId,
          ledgerVersion: "8.0.3",
        });
        const out = join(directory, `checkpoint-${++checkpointAttempt}`);
        const { manifestPath } = await writeCheckpoint(out, acquired);
        await privateReference(
          join(out, "wallet-sdk-envelope.json"),
          snapshot,
          "ShieldedWalletState JSON",
        );
        lastCheckpoint = await privateReference(
          join(directory, `checkpoint-${checkpointAttempt}.json`),
          await readFile(manifestPath),
          "checkpoint-json",
        );
        return acquired.manifest;
      } catch (error) {
        lastError = error;
        await sleep(2000);
      }
    } while (Date.now() < deadline);
    throw new Error("wallet/node checkpoint agreement timed out", {
      cause: lastError,
    });
  }
  async function submit(action, built) {
    const deadline = Date.now() + 180_000;
    let finalized;
    while (!finalized) {
      try {
        finalized = await finalizePreservingOffers(
          wallet,
          built.tx,
          keys,
          new Date(Date.now() + 20 * 60_000),
        );
      } catch (error) {
        if (
          !String(error).includes(
            "Insufficient Funds: could not balance dust",
          ) ||
          Date.now() >= deadline
        )
          throw error;
        await sleep(5000);
      }
    }
    const transactionHash = normalizeHash(finalized.transactionHash());
    const bytes = finalized.serialize();
    const finalRef = await privateReference(
      join(directory, `${action}-finalized.bin`),
      bytes,
      "Transaction<Signature,Proof,Binding>",
    );
    submitted = true; // Once transport begins, never release the reservation on an ambiguous response.
    await wallet.submitTransaction(finalized);
    const observed = await confirmed(action, transactionHash);
    const view = await checkpoint(observed);
    receipt.phases.push({
      action,
      transactionHash,
      finalized: finalRef,
      offerFingerprint: shieldedOfferFingerprint(finalized),
      checkpoint: lastCheckpoint,
      metadata: view.metadata,
      artifacts: built.result.artifacts,
    });
    await privateReference(
      join(directory, `${action}-receipt.json`),
      jsonBytes(receipt),
      "JSON",
    );
    return { finalized, observed };
  }
  const waitRecovered = async (expected) => {
    const state = await waitState((current) => {
      try {
        selectExactCoin(current.shielded?.availableCoins ?? [], expected);
        return true;
      } catch {
        return false;
      }
    });
    return selectExactCoin(state.shielded.availableCoins, expected);
  };
  try {
    const preflightBlock = await rpc("chain_getFinalizedHead", []);
    const version = decodeScaleString(
      scaleBytes(
        await rpc("state_call", [
          "MidnightRuntimeApi_get_ledger_version",
          "0x",
          preflightBlock,
        ]),
      ),
    );
    assert.equal(
      version,
      "=8.0.3",
      "node ledger constraint differs from pinned acceptance runtime",
    );
    receipt.provenance.nodeLedgerConstraint = version;
    const proofDeadline = Date.now() + 300_000;
    while (true) {
      try {
        await fetch(proofServer, { signal: AbortSignal.timeout(3000) });
        break;
      } catch (error) {
        if (Date.now() >= proofDeadline)
          throw new Error("proof server is not HTTP ready", { cause: error });
        await sleep(5000);
      }
    }
    await wallet.start(shieldedSecretKeys, dustSecretKey);
    const funded = await waitState(
      (state) =>
        (state.unshielded?.balances[ledger.nativeToken().raw] ?? 0n) > 0n ||
        (state.dust?.balance(new Date()) ?? 0n) > 0n,
    );
    if ((funded.dust?.balance(new Date()) ?? 0n) <= 0n) {
      const coins =
        funded.unshielded?.availableCoins.filter(
          (row) => row.meta.registeredForDustGeneration === false,
        ) ?? [];
      assert(coins.length > 0, "wallet requires NIGHT for Dust registration");
      const recipe = await wallet.registerNightUtxosForDustGeneration(
        coins,
        unshieldedKeystore.getPublicKey(),
        (payload) => unshieldedKeystore.signData(payload),
      );
      await wallet.submitTransaction(await wallet.finalizeRecipe(recipe));
      await waitState((state) => (state.dust?.balance(new Date()) ?? 0n) > 0n);
    }
    const deploy = await actionRequest("deploy");
    assert.equal(
      await indexedAction(),
      null,
      "contract already deployed; use a fresh isolated chain",
    );
    await submit("deploy", deploy);
    const walletKeys = {
      walletCoinPublicKeyHex: shieldedSecretKeys.coinPublicKey,
      walletEncryptionPublicKeyHex: shieldedSecretKeys.encryptionPublicKey,
    };
    const bootstrap = await actionRequest("bootstrap", {
      domainHex: randomBytes(32).toString("hex"),
      nonceHex: randomBytes(32).toString("hex"),
      amount: "42",
      ...walletKeys,
    });
    await submit("bootstrap", bootstrap);
    const selected = await waitRecovered(bootstrap.result.coin);
    // The checkpoint is acquired before reservation; initSwap changes pending state only.
    reserved = await wallet.shielded.initSwap(
      shieldedSecretKeys,
      { [selected.coin.type]: selected.coin.value },
      [],
    );
    submitted = false;
    const input = selectedReservationInput(reserved, selected);
    const inputRef = await privateReference(
      join(directory, "selected-input.bin"),
      input.serialize(),
      "Input<ProofPreimage>",
    );
    const accept = await actionRequest("accept", {
      input: inputRef,
      coin: coinJson(selected.coin, selected.commitment),
    });
    await submit("accept", accept);
    reserved = undefined;
    await waitState(
      (state) =>
        !(state.shielded?.availableCoins ?? []).some(
          (row) => row.nullifier === selected.nullifier,
        ),
    );
    const release = await actionRequest("release", {
      coin: accept.result.coin,
      ...walletKeys,
    });
    const released = await submit("release", release);
    const recovered = await waitRecovered(release.result.coin);
    receipt.recovered = coinJson(recovered.coin, recovered.commitment);
    // Bypass wallet pending-cache duplicate handling: request actual node submission.
    try {
      await wallet.submissionService.submitTransaction(
        released.finalized,
        "Finalized",
      );
      throw new Error("node unexpectedly accepted replay");
    } catch (error) {
      if (error.message === "node unexpectedly accepted replay") throw error;
      // An arbitrary transport failure is not evidence of ledger replay rejection.
      assert(
        /duplicate|already imported|already in|stale|nullifier|spent|invalid transaction/i.test(
          String(error),
        ),
        "replay failed without a recognized node rejection",
      );
      receipt.replay = {
        transactionHash: normalizeHash(released.finalized.transactionHash()),
        rejection: String(error),
      };
    }
    receipt.status = "passed";
    await privateReference(
      join(directory, "receipt.json"),
      jsonBytes(receipt),
      "JSON",
    );
    console.log(
      `shielded lifecycle passed: ${join(directory, "receipt.json")}`,
    );
  } catch (error) {
    if (reserved && !submitted)
      await wallet.shielded.revertTransaction(reserved);
    receipt.status = "failed";
    receipt.failure = { name: error.name, message: error.message };
    await privateReference(
      join(directory, "failure.json"),
      jsonBytes(receipt),
      "JSON",
    );
    throw error;
  } finally {
    await wallet.stop();
  }
}

if (
  process.argv[1] &&
  import.meta.url === pathToFileURL(resolve(process.argv[1])).href
) {
  main().catch((error) => {
    console.error(`shielded lifecycle failed: ${error.message}`);
    process.exitCode = 1;
  });
}
