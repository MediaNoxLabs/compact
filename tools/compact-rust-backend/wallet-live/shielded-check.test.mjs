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

import assert from "node:assert/strict";
import { test } from "node:test";
import { mkdtemp, chmod, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import {
  requireIsolatedEndpoints,
  privateReference,
  readReference,
  selectExactCoin,
  selectedReservationInput,
  finalizePreservingOffers,
  verifyResult,
} from "./shielded-check.mjs";
import { shieldedOfferFingerprint } from "./shielded-handoff.mjs";

const selected = {
  coin: { type: "aa", nonce: "bb", value: 42n, mt_index: 7n },
  commitment: "cc",
  nullifier: "dd",
};
const expected = {
  colorHex: "aa",
  nonceHex: "bb",
  value: "42",
  mtIndex: "7",
  commitmentHex: "cc",
};
const offer = (value) => ({ serialize: () => Buffer.from(value) });
const transaction = (value) => ({
  guaranteedOffer: value == null ? undefined : offer(value),
  fallibleOffer: new Map(),
});

test("private binary references reject changed bytes, encoding and public files", async () => {
  const directory = await mkdtemp(join(tmpdir(), "compact-shielded-guards-"));
  try {
    const path = join(directory, "input.bin");
    const ref = await privateReference(
      path,
      Buffer.from("secret witness"),
      "Input<ProofPreimage>",
    );
    assert.equal(
      (await readReference(ref, ref.encoding)).toString(),
      "secret witness",
    );
    await assert.rejects(readReference(ref, "Offer<Proof>"), /encoding/);
    await assert.rejects(
      privateReference(path, Buffer.from("replacement"), ref.encoding),
      /EEXIST/,
    );
    await chmod(path, 0o644);
    await assert.rejects(readReference(ref, ref.encoding), /private/);
    await chmod(path, 0o600);
    await writeFile(path, "changed bytes!");
    await assert.rejects(readReference(ref, ref.encoding), /digest/);
  } finally {
    await rm(directory, { recursive: true });
  }
});

test("selection requires exact unique confirmed type/value/nonce/actual index/commitment", () => {
  assert.equal(selectExactCoin([selected], expected), selected);
  for (const [key, value] of [
    ["colorHex", "ab"],
    ["nonceHex", "ba"],
    ["value", "41"],
    ["mtIndex", "8"],
    ["commitmentHex", "cd"],
  ]) {
    assert.throws(() =>
      selectExactCoin([selected], { ...expected, [key]: value }),
    );
  }
  assert.throws(
    () => selectExactCoin([selected, selected], expected),
    /exactly one/,
  );
  assert.throws(() => selectExactCoin([], expected), /exactly one/);
});

test("public reservation must contain precisely the selected input and no change or other intents", () => {
  const input = { contractAddress: undefined, nullifier: "dd" };
  const base = {
    guaranteedOffer: {
      inputs: [input],
      outputs: [],
      transients: [],
      deltas: new Map([["aa", 42n]]),
    },
  };
  assert.equal(selectedReservationInput(base, selected), input);
  for (const mutated of [
    { ...base, intents: new Map([[1, {}]]) },
    { ...base, fallibleOffer: new Map([[1, {}]]) },
    ...[
      { inputs: [input, input] },
      { outputs: [{}] },
      { transients: [{}] },
      { inputs: [{ ...input, nullifier: "other" }] },
      { inputs: [{ ...input, contractAddress: "contract" }] },
      { deltas: new Map([["aa", 41n]]) },
    ].map((changes) => ({
      guaranteedOffer: { ...base.guaranteedOffer, ...changes },
    })),
  ])
    assert.throws(() => selectedReservationInput(mutated, selected));
});

test("Dust-only balancing retains full offer bytes and exact placement", async () => {
  const original = transaction("proof and coin");
  let options;
  const wallet = {
    balanceFinalizedTransaction: async (tx, keys, opts) => {
      options = opts;
      return {
        originalTransaction: tx,
        balancingTransaction: transaction(null),
      };
    },
    finalizeRecipe: async () => transaction("proof and coin"),
    revert: async () => {
      throw new Error("unexpected revert");
    },
  };
  const ttl = new Date();
  const final = await finalizePreservingOffers(wallet, original, {}, ttl);
  assert.deepEqual(options, { ttl, tokenKindsToBalance: ["dust"] });
  assert.deepEqual(
    shieldedOfferFingerprint(original),
    shieldedOfferFingerprint(final),
  );
});

test("offer mutation or fee shielded output aborts and reverts the recipe", async () => {
  for (const mode of ["changed", "moved", "fee", "in-place"]) {
    const original = transaction("proof");
    let reverted = false;
    const wallet = {
      balanceFinalizedTransaction: async () => ({
        originalTransaction: original,
        balancingTransaction: transaction(mode === "fee" ? "extra" : null),
      }),
      finalizeRecipe: async () => {
        if (mode === "in-place") {
          original.guaranteedOffer = offer("mutated");
          return original;
        }
        if (mode === "moved")
          return { fallibleOffer: new Map([[1, offer("proof")]]) };
        return transaction("changed");
      },
      revert: async () => {
        reverted = true;
      },
    };
    await assert.rejects(
      finalizePreservingOffers(wallet, original, {}, new Date()),
    );
    assert(reverted, mode);
  }
});

test("builder result is bound to exact requested action/address/network/offer", () => {
  class ContractCall {
    constructor() {
      this.address = "address";
      this.entryPoint = "accept";
    }
  }
  class ContractDeploy {}
  const tx = {
    ...transaction("retained"),
    intents: new Map([[1, { actions: [new ContractCall()] }]]),
  };
  const result = {
    format: "compact-shielded-live/v1",
    kind: "action-result",
    action: "accept",
    networkId: "undeployed",
    addressHex: "address",
    offerFingerprint: shieldedOfferFingerprint(tx),
  };
  const types = { ContractCall, ContractDeploy };
  verifyResult(result, "accept", "undeployed", tx, types, "address");
  for (const change of [
    { action: "release" },
    { addressHex: "other" },
    { networkId: "preview" },
    { offerFingerprint: { guaranteed: "wrong", fallible: [] } },
  ]) {
    assert.throws(() =>
      verifyResult(
        { ...result, ...change },
        "accept",
        "undeployed",
        tx,
        types,
        "address",
      ),
    );
  }
  tx.intents.get(1).actions[0].entryPoint = "release";
  assert.throws(
    () => verifyResult(result, "accept", "undeployed", tx, types, "address"),
    /entry point/,
  );
});

test("runner cannot target protected oxid or remote service ports", () => {
  const endpoints = [
    "http://127.0.0.1:49944",
    "http://127.0.0.1:48088/api/v3/graphql",
    "http://127.0.0.1:46300",
  ];
  requireIsolatedEndpoints(...endpoints);
  for (const [index, value] of [
    [0, "http://127.0.0.1:9944"],
    [1, "http://127.0.0.1:8088"],
    [2, "http://127.0.0.1:6300"],
    [0, "http://remote:49944"],
  ]) {
    const changed = [...endpoints];
    changed[index] = value;
    assert.throws(() => requireIsolatedEndpoints(...changed), /isolated/);
  }
});
