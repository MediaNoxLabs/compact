// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

// Unchanged original DID constructor and source calls; strict Rust proof is a separate gate.
export const captureScope = "Original nested JWK VerificationMethod CRUD with source-realistic relation prestates";
export function captureCases(scenario) {
  const id = "did:jwk:日本語🔑";
  const method = (curve = 0, key = 3, overrides = {}) => ({
    id,
    typ: 1,
    publicKeyJwk: { kty: key, crv: curve, x: "x:α", y: "y:β", ...overrides.publicKeyJwk },
    ...Object.fromEntries(Object.entries(overrides).filter(([name]) => name !== "publicKeyJwk")),
  });
  const set = (caseId, value, mutation, error, o = {}) =>
    ({ id: caseId, name: "setVerificationMethod", a: { method: value, mutation }, error, o });
  const remove = (caseId, methodId, error, o = {}) =>
    ({ id: caseId, name: "removeVerificationMethod", a: { id: methodId }, error, o });
  const relation = (caseId, rel, mutation, error) => ({
    id: caseId, name: "setVerificationMethodRelation",
    a: { relation: rel, id, mutation }, error,
  });

  scenario("jwk-method-recording", [
    set("stale-before-signature", method(), 1, "version is stale", { version: "1", badSignature: true }),
    set("wrong-controller", method(), 1, "Invalid Jubjub Schnorr signature", { signer: "3" }),
    set("reduction-error", method(), 1, "reduction failure", { fail: "reduction" }),
    set("undefined-mutation", method(), 0, "mutation"),
    set("invalid-typ", method(0, 3, { typ: 0 }), 1, "JsonWebKey"),
    set("invalid-key-type", method(0, 1), 1, "Only OKP"),
    set("invalid-okp-curve", method(3, 3), 1, "OKP keys"),
    set("invalid-ec-curve", method(1, 0), 1, "EC keys"),
    set("missing-update", method(), 2, "does not exist"),
    set("insert-unicode", method(), 1),
    set("duplicate", method(), 1, "already exists"),
    set("update-timestamp-rollback", method(1, 3), 2, "timestamp failure", { fail: "timestamp" }),
    set("update-jwk", method(3, 0, { publicKeyJwk: { x: "updated:Δ", y: "updated:🚀" } }), 2),
    remove("missing-remove", "missing", "does not exist"),
    remove("remove-unicode", id),
    remove("remove-again", id, "does not exist"),
    { id: "deactivate", name: "deactivate" },
    set("inactive", method(), 1, "Contract is not active"),
  ]);

  scenario("jwk-agreement-relation-prestate", [
    set("insert-x25519", method(1), 1),
    relation("bind-key-agreement", 3, 1),
    set("reject-non-x25519-update", method(0), 2, "Existing KeyAgreement relation requires an X25519"),
    remove("reject-referenced-remove", id, "still referenced"),
    relation("unbind-key-agreement", 3, 2),
    remove("remove-after-unbind", id),
  ]);
  scenario("jwk-signing-relation-prestate", [
    set("insert-ed25519", method(0), 1),
    relation("bind-authentication", 1, 1),
    set("reject-x25519-update", method(1), 2, "Existing signing verification relations cannot use X25519"),
  ]);
}
