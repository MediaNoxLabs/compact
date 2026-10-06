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

// Run from the repository root with the unchanged DID's generated contract/index.js.
// Synthetic signing keys/nonces are test data, never wallet credentials.
import { pathToFileURL } from "node:url";
import { readFileSync, realpathSync } from "node:fs";
import { resolve, dirname } from "node:path";
import { createHash } from "node:crypto";
import { captureCases } from "./lifecycle-cases.mjs";
import * as r from "../../../runtime/dist/index.js";
const generated = process.argv[2];
if (!generated || process.argv.length !== 3)
  throw Error("expected generated DID contract/index.js");
if (
  realpathSync(
    resolve(dirname(generated), "node_modules/@midnight-ntwrk/compact-runtime"),
  ) !== realpathSync("runtime")
)
  throw Error("wrong runtime");
const m = await import(pathToFileURL(resolve(generated)).href);
const json = (x) =>
  JSON.parse(
    JSON.stringify(x, (_, v) =>
      typeof v === "bigint"
        ? String(v)
        : v instanceof Uint8Array
          ? Array.from(v)
          : v,
    ),
  );
const hash = (p) => createHash("sha256").update(readFileSync(p)).digest("hex");
const le = (n) =>
  n.toString(16).padStart(64, "0").match(/../g).reverse().join("");
const stateHex = (s) => Buffer.from(s.serialize()).toString("hex");
const key = { bytes: new Uint8Array(32) },
  mask = (1n << 248n) - 1n;
const order =
  6554484396890773809930967563523245729705921265872317281365359162392183254199n;
let options = {},
  calls = [],
  queries = [],
  events = [];
const query = r.QueryContext.prototype.query;
r.QueryContext.prototype.query = function (...args) {
  const out = query.apply(this, args);
  queries.push(json(out.gasCost));
  events.push({ query: queries.length - 1, program: json(args[0]) });
  return out;
};
function witness(name, privateState, value) {
  calls.push({ name });
  events.push({ witness: name });
  if (options.fail === name) throw Error(`${name} failure`);
  return [privateState + 1, value];
}
const witnesses = {
  localControllerPublicKey: ({ privateState }) =>
    witness("controller", privateState, r.ecMulGenerator(1n)),
  localRecoveryAuthorityPublicKey: ({ privateState }) =>
    witness(
      "recovery",
      privateState,
      r.ecMulGenerator(options.sameKeys ? 1n : 3n),
    ),
  currentTimestamp: ({ privateState }) =>
    witness("timestamp", privateState, BigInt(options.timestamp ?? 100)),
  getSchnorrReduction: ({ privateState }, h) => {
    calls.push({ name: "reduction", hash: le(h) });
    events.push({ witness: "reduction" });
    if (options.fail === "reduction") throw Error("reduction failure");
    return [privateState + 1, [h >> 248n, h & mask]];
  },
};
function args(name, a) {
  if (name === "rotateControllerKey" || name === "recoverControllerKey")
    return [r.ecMulGenerator(BigInt(a.key))];
  if (name === "setSchnorrJubjubVerificationMethod")
    return [
      { id: a.method.id, publicKey: r.ecMulGenerator(BigInt(a.method.key)) },
      a.mutation,
    ];
  if (name === "setVerificationMethod") return [a.method, a.mutation];
  if (name === "setAlsoKnownAs") return [a.value, a.mutation];
  if (name === "setService") return [a.service, a.mutation];
  if (name === "setVerificationMethodRelation")
    return [a.relation, a.id, a.mutation];
  if (name.startsWith("remove")) return [a.id];
  return [];
}
function signature(digest, secret, bad = false) {
  const announcement = r.ecMulGenerator(2n),
    pk = r.ecMulGenerator(secret);
  const h = r.transientHash(new r.CompactTypeVector(8, r.CompactTypeField), [
    announcement.x,
    announcement.y,
    pk.x,
    pk.y,
    ...digest,
  ]);
  return {
    announcement,
    response: (2n + secret * (h & mask) + (bad ? 1n : 0n)) % order,
  };
}
const scenarios = [];
function scenario(id, steps, ctor = {}) {
  options = ctor;
  calls = [];
  queries = [];
  events = [];
  const c = new m.Contract(witnesses),
    s = { id, constructorOptions: ctor, steps: [] };
  let state,
    priv,
    controller = 1n;
  try {
    const init = c.initialState({
      initialPrivateState: 0,
      initialZswapLocalState: r.emptyZswapLocalState(key),
    });
    state = init.currentContractState;
    priv = init.currentPrivateState;
    s.initial = {
      state: stateHex(state),
      private: priv,
      witnessCalls: json(calls),
      queries: json(queries),
      events: json(events),
    };
  } catch (e) {
    s.constructorError = e.message;
    s.witnessCalls = json(calls);
    s.events = json(events);
    scenarios.push(s);
    return;
  }
  for (const step of steps) {
    const { name, a = {}, o = {} } = step;
    const view = m.ledger(state.data.state);
    const version = BigInt(o.version ?? view.version);
    let input = args(name, a);
    const digest =
      name === "verifySchnorrJubjubDigestSignature"
        ? [1n, 2n, 3n, 4n]
        : m.pureCircuits[name + "AuthorizationDigest"](
            view.id,
            version,
            ...input,
          );
    const secret = BigInt(
      o.signer ?? (name === "recoverControllerKey" ? 3n : controller),
    );
    const sig = signature(digest, secret, o.badSignature);
    if (name === "verifySchnorrJubjubDigestSignature")
      input = [a.id, digest, sig];
    else input.push(sig, version);
    options = { timestamp: String(200 + s.steps.length), ...o };
    calls = [];
    queries = [];
    events = [];
    const row = {
      id: step.id,
      name,
      args: a,
      options,
      version: String(version),
      responseHex: le(sig.response),
      before: stateHex(state),
      privateBefore: priv,
    };
    try {
      const out = c.circuits[name](
        r.createCircuitContext(r.dummyContractAddress(), key, state.data, priv),
        ...input,
      );
      const saved = json(queries);
      state.data = new r.ChargedState(
        out.context.currentQueryContext.state.state,
      );
      priv = out.context.currentPrivateState;
      Object.assign(row, {
        after: stateHex(state),
        privateAfter: priv,
        queries: saved,
        events: json(events),
        witnessCalls: json(calls),
        publicTranscript: json(out.proofData.publicTranscript),
        privateTranscript: out.proofData.privateTranscriptOutputs.map((v) => ({
          valueAtoms: v.value.map((a) => Array.from(a)),
          alignment: json(v.alignment),
        })),
      });
      if (name === "rotateControllerKey" || name === "recoverControllerKey")
        controller = BigInt(a.key);
    } catch (e) {
      Object.assign(row, {
        error: e.message,
        witnessCalls: json(calls),
        queries: json(queries),
        events: json(events),
      });
    }
    if (step.error !== undefined && !row.error)
      throw Error(`${id}/${step.id}: expected failure`);
    if (step.error === undefined && row.error)
      throw Error(`${id}/${step.id}: ${row.error}`);
    if (step.error && !row.error.includes(step.error))
      throw Error(`${id}/${step.id}: wrong error ${row.error}`);
    s.steps.push(row);
  }
  scenarios.push(s);
}
captureCases(scenario);
const provenance = [
  "examples/rust_backend/did_adoption/packages/contract/src/did.compact",
  "examples/rust_backend/did_adoption/packages/jubjub-schnorr/src/schnorr.compact",
  "tests-rust-backend/did-adoption/lib.rs",
  "runtime/src/built-ins.ts",
  "runtime/src/compact-types.ts",
  "examples/rust_backend/did_adoption/source-manifest.json",
  "tests-rust-backend/did-adoption/oracle/capture-lifecycle.mjs",
  "tests-rust-backend/did-adoption/oracle/lifecycle-cases.mjs",
].map((path) => ({ path, sha256: hash(path) }));
process.stdout.write(
  JSON.stringify(
    json({
      scope:
        "Native constructor-driven source calls; only deactivate separately has recorded/proof evidence",
      provenance,
      generatedJavaScriptSha256: hash(generated),
      runtimeJavaScriptSha256: hash("runtime/dist/built-ins.js"),
      scenarios,
    }),
    null,
    2,
  ) + "\n",
);
