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

// Independent direct calls to every exported original pure API.
import { pathToFileURL } from "node:url";
import { readFileSync, realpathSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { createHash } from "node:crypto";
import * as r from "../../../runtime/dist/index.js";
const path = process.argv[2];
if (!path || process.argv.length !== 3)
  throw Error("expected original generated DID module");
if (
  realpathSync(
    resolve(dirname(path), "node_modules/@midnight-ntwrk/compact-runtime"),
  ) !== realpathSync("runtime")
)
  throw Error("wrong runtime");
const m = await import(pathToFileURL(resolve(path)).href);
const le = (n) =>
  n.toString(16).padStart(64, "0").match(/../g).reverse().join("");
const method = {
  id: "key",
  typ: 1,
  publicKeyJwk: { kty: 3, crv: 0, x: "x", y: "y" },
};
const service = {
  id: "svc",
  typ: "LinkedDomains",
  serviceEndpoint: "https://example.invalid/δ",
};
const specs = {
  controllerAuthorizationDigest: { operationHash: "1", argsHash: "2" },
  rotateControllerKeyAuthorizationDigest: { key: "1" },
  recoverControllerKeyAuthorizationDigest: { key: "1" },
  setAlsoKnownAsAuthorizationDigest: { value: "alias", mutation: 1 },
  setVerificationMethodAuthorizationDigest: { method, mutation: 1 },
  removeVerificationMethodAuthorizationDigest: { id: "key" },
  setSchnorrJubjubVerificationMethodAuthorizationDigest: {
    method: { id: "key", key: "1" },
    mutation: 1,
  },
  removeSchnorrJubjubVerificationMethodAuthorizationDigest: { id: "key" },
  setVerificationMethodRelationAuthorizationDigest: {
    relation: 1,
    id: "key",
    mutation: 1,
  },
  setServiceAuthorizationDigest: { service, mutation: 1 },
  removeServiceAuthorizationDigest: { id: "svc" },
  deactivateAuthorizationDigest: {},
};
function args(name, a) {
  if (name === "controllerAuthorizationDigest")
    return [BigInt(a.operationHash), BigInt(a.argsHash)];
  if (name.startsWith("rotate") || name.startsWith("recover"))
    return [r.ecMulGenerator(BigInt(a.key))];
  if (name.startsWith("setAlso")) return [a.value, a.mutation];
  if (name === "setVerificationMethodAuthorizationDigest")
    return [a.method, a.mutation];
  if (name.startsWith("setSchnorr"))
    return [
      { id: a.method.id, publicKey: r.ecMulGenerator(BigInt(a.method.key)) },
      a.mutation,
    ];
  if (name.startsWith("setVerificationMethodRelation"))
    return [a.relation, a.id, a.mutation];
  if (name.startsWith("setService")) return [a.service, a.mutation];
  if (name.startsWith("remove")) return [a.id];
  return [];
}
const cases = [];
function add(name, id, a, version = "0", addressByte = 0) {
  const contractId = { bytes: new Uint8Array(32).fill(addressByte) };
  const value = m.pureCircuits[name](
    contractId,
    BigInt(version),
    ...args(name, a),
  );
  cases.push({
    name,
    id,
    args: structuredClone(a),
    version,
    addressByte,
    resultHex: value.map(le),
  });
}
for (const [name, base] of Object.entries(specs)) {
  add(name, "base", base);
  add(name, "version-one", base, "1");
  add(name, "version-max", base, "18446744073709551615");
  add(name, "other-address", base, "0", 17);
  const mutate = (id, path, value) => {
    const a = structuredClone(base);
    let o = a;
    for (const p of path.slice(0, -1)) o = o[p];
    o[path.at(-1)] = value;
    add(name, id, a);
  };
  if ("mutation" in base)
    for (const v of [0, 2]) mutate(`mutation-${v}`, ["mutation"], v);
  if ("key" in base) mutate("other-key", ["key"], "5");
  for (const key of ["id", "value"])
    if (key in base)
      for (const value of ["", "日本語🗝️"])
        mutate(`${key}-${value ? "unicode" : "empty"}`, [key], value);
  if ("operationHash" in base) {
    mutate("operation-changed", ["operationHash"], "7");
    mutate("args-changed", ["argsHash"], "7");
  }
  if ("relation" in base)
    for (const v of [0, 2, 3, 4, 5]) mutate(`relation-${v}`, ["relation"], v);
  if ("service" in base)
    for (const key of ["id", "typ", "serviceEndpoint"])
      for (const value of ["", "日本語🗝️"])
        mutate(
          `service-${key}-${value ? "unicode" : "empty"}`,
          ["service", key],
          value,
        );
  if ("method" in base) {
    for (const value of ["", "日本語🗝️"])
      mutate(
        `method-id-${value ? "unicode" : "empty"}`,
        ["method", "id"],
        value,
      );
    if ("key" in base.method)
      mutate("method-other-key", ["method", "key"], "5");
    else {
      mutate("method-type-undefined", ["method", "typ"], 0);
      for (const [key, values] of Object.entries({
        kty: [0, 1, 2],
        crv: [1, 2, 3, 4, 5, 6],
        x: ["", "日本語🗝️"],
        y: ["", "日本語🗝️"],
      }))
        for (const v of values)
          mutate(`jwk-${key}-${v}`, ["method", "publicKeyJwk", key], v);
    }
  }
}
const sha = (p) => createHash("sha256").update(readFileSync(p)).digest("hex");
const provenance = [
  "examples/rust_backend/did_adoption/packages/contract/src/did.compact",
  "examples/rust_backend/did_adoption/packages/jubjub-schnorr/src/schnorr.compact",
  "tests-rust-backend/did-adoption/lib.rs",
  "runtime/src/built-ins.ts",
  "runtime/src/compact-types.ts",
  "examples/rust_backend/did_adoption/source-manifest.json",
  "tests-rust-backend/did-adoption/oracle/capture-pure.mjs",
].map((path) => ({ path, sha256: sha(path) }));
process.stdout.write(
  JSON.stringify(
    {
      scope:
        "12 direct pure exports; finite argument/domain sensitivity, not a collision-resistance proof",
      provenance,
      generatedJavaScriptSha256: sha(path),
      runtimeJavaScriptSha256: sha("runtime/dist/built-ins.js"),
      cases,
    },
    null,
    2,
  ) + "\n",
);
