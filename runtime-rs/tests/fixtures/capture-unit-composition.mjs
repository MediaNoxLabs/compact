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

// Explicit generated paths for the reduced crypto and unchanged original DID.
import { pathToFileURL } from "node:url";
import { readFileSync, realpathSync } from "node:fs";
import { resolve, dirname } from "node:path";
import { createHash } from "node:crypto";
import * as r from "../../../runtime/dist/index.js";
const paths = process.argv.slice(2);
if (paths.length !== 2)
  throw Error("expected crypto and DID contract/index.js");
for (const p of paths)
  if (
    realpathSync(
      resolve(dirname(p), "node_modules/@midnight-ntwrk/compact-runtime"),
    ) !== realpathSync("runtime")
  )
    throw Error("wrong runtime");
const mods = await Promise.all(
  paths.map((p) => import(pathToFileURL(resolve(p)).href)),
);
const json = (v) =>
  JSON.parse(
    JSON.stringify(v, (_, x) =>
      typeof x === "bigint"
        ? String(x)
        : x instanceof Uint8Array
          ? Array.from(x)
          : x,
    ),
  );
const hex = (s) => Buffer.from(s.serialize()).toString("hex");
const key = { bytes: new Uint8Array(32) },
  pk = r.ecMulGenerator(1n),
  announcement = r.ecMulGenerator(2n),
  mask = (1n << 248n) - 1n;
let queries = [],
  events = [],
  witnessCalls = [],
  options = {};
const original = r.QueryContext.prototype.query;
r.QueryContext.prototype.query = function (...args) {
  const out = original.call(this, ...args);
  queries.push({ gasCost: json(out.gasCost) });
  events.push({ query: queries.length - 1 });
  return out;
};
const witnesses = {
  getSchnorrReduction: ({ privateState }, hash) => {
    witnessCalls.push({
      name: "reduction",
      hash: hash.toString(16).padStart(64, "0").match(/../g).reverse().join(""),
    });
    events.push({ witness: "reduction" });
    return [
      privateState + 1,
      options.badReduction
        ? [0n, 0n]
        : options.badQuotient
          ? [116n, hash & mask]
          : [hash >> 248n, hash & mask],
    ];
  },
  localControllerPublicKey: ({ privateState }) => [privateState + 1, pk],
  localRecoveryAuthorityPublicKey: ({ privateState }) => [
    privateState + 1,
    r.ecMulGenerator(3n),
  ],
  currentTimestamp: ({ privateState }) => {
    witnessCalls.push({ name: "timestamp" });
    events.push({ witness: "timestamp" });
    if (options.timestampError) throw Error("timestamp failure");
    return [privateState + 1, options.timestamp ?? 200n];
  },
  now: ({ privateState }) => witnesses.currentTimestamp({ privateState }),
};
const rows = [];
const array = (values) =>
  values.reduce((a, v) => a.arrayPush(v), r.StateValue.newArray());
const scalar = (type, value) =>
  r.StateValue.newCell({
    value: type.toValue(value),
    alignment: type.alignment(),
  });
function seed(state, path, type, value) {
  const f = state.data.state.asArray();
  if (path.length === 2) {
    const b = f[path[0]].asArray();
    b[path[1]] = scalar(type, value);
    f[path[0]] = array(b);
  } else f[path[0]] = scalar(type, value);
  state.data = new r.ChargedState(array(f));
}
function run(which, label, o = {}) {
  options = { timestamp: 100n };
  const m = mods[which],
    c = new m.Contract(witnesses),
    did = which === 1;
  const initial = c.initialState(
    {
      initialPrivateState: 0,
      initialZswapLocalState: r.emptyZswapLocalState(key),
    },
    ...(did ? [] : [pk]),
  );
  const state = initial.currentContractState;
  if (o.inactive) seed(state, did ? [1, 5] : [1], r.CompactTypeBoolean, false);
  const u64 = new r.CompactTypeUnsignedInteger((1n << 64n) - 1n, 8);
  if (o.version !== undefined) seed(state, did ? [1, 1] : [2], u64, o.version);
  if (o.operations !== undefined) seed(state, [1, 6], u64, o.operations);
  const expected = o.expected ?? o.version ?? 0n;
  const digest = did
    ? m.pureCircuits.deactivateAuthorizationDigest(
        m.ledger(state.data.state).id,
        expected,
      )
    : [r.transientHash(u64, expected), 1n, 2n, 3n];
  const challenge = r.transientHash(
    new r.CompactTypeVector(8, r.CompactTypeField),
    [announcement.x, announcement.y, pk.x, pk.y, ...digest],
  );
  const signature = {
    announcement,
    response: 2n + (challenge & mask) + (o.badSignature ? 1n : 0n),
  };
  const name = did ? "deactivate" : o.verifyOnly ? "verifyOnly" : "close";
  options = o;
  queries = [];
  events = [];
  witnessCalls = [];
  const context = r.createCircuitContext(
    r.dummyContractAddress(),
    key,
    state.data,
    7,
  );
  const row = {
    id: (did ? "did" : "crypto") + "/" + label,
    source: did ? "did" : "crypto",
    export: name,
    options: o,
    expected,
    signature,
    responseHex: signature.response
      .toString(16)
      .padStart(64, "0")
      .match(/../g)
      .reverse()
      .join(""),
    challenge,
    before: hex(state),
    privateBefore: 7,
  };
  try {
    const out = c.circuits[name](context, signature, expected);
    const savedQueries = [...queries],
      savedEvents = [...events];
    state.data = new r.ChargedState(
      out.context.currentQueryContext.state.state,
    );
    Object.assign(row, {
      result: out.result,
      after: hex(state),
      privateAfter: out.context.currentPrivateState,
      publicTranscript: out.proofData.publicTranscript,
      privateTranscript: out.proofData.privateTranscriptOutputs.map((v) => ({
        valueAtoms: v.value.map((a) => Array.from(a)),
        alignment: v.alignment,
      })),
      queries: savedQueries,
      reportedGas: out.gasCost,
      events: savedEvents,
      witnessCalls: [...witnessCalls],
    });
  } catch (e) {
    Object.assign(row, {
      error: e.message,
      queries: [...queries],
      events: [...events],
      witnessCalls: [...witnessCalls],
    });
  }
  rows.push(json(row));
}
for (const which of [0, 1]) {
  run(which, "valid");
  run(which, "stale", { expected: 1n });
  run(which, "bad-signature", { badSignature: true });
  run(which, "bad-reduction", { badReduction: true });
  run(which, "bad-quotient", { badQuotient: true });
  run(which, "inactive", { inactive: true });
  run(which, "stale-before-signature", { expected: 1n, badSignature: true });
  run(which, "signature-before-inactive", {
    inactive: true,
    badSignature: true,
  });
  run(which, "timestamp-failure", { timestampError: true });
  run(which, "version-max", { version: (1n << 64n) - 1n });
  run(which, "timestamp-zero", { timestamp: 0n });
}
run(0, "unguarded-inactive", { inactive: true, verifyOnly: true });
run(1, "operations-max", { operations: (1n << 64n) - 1n });
const provenance = [
  "examples/rust_backend/unit_composition_crypto.compact",
  "examples/rust_backend/did_adoption/packages/contract/src/did.compact",
  "examples/rust_backend/did_adoption/packages/jubjub-schnorr/src/schnorr.compact",
  "runtime/src/built-ins.ts",
  "runtime/src/compact-types.ts",
].map((path) => ({
  path,
  sha256: createHash("sha256").update(readFileSync(path)).digest("hex"),
}));
process.stdout.write(
  JSON.stringify(
    {
      provenance,
      sources: paths.map((p) => ({
        sha256: createHash("sha256").update(readFileSync(p)).digest("hex"),
      })),
      runtimeSha256: createHash("sha256")
        .update(readFileSync("runtime/dist/built-ins.js"))
        .digest("hex"),
      cases: rows,
    },
    null,
    2,
  ) + "\n",
);
