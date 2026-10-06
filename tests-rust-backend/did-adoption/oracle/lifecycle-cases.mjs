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

// Reviewed source-call cases. Every scenario uses the original constructor.
export function captureCases(scenario) {
  const step = (id, name, a = {}, error, o = {}) => ({ id, name, a, error, o });
  const vm = (id, crv = 0, kty = 3, typ = 1) => ({
    id,
    typ,
    publicKeyJwk: { kty, crv, x: "x", y: "y" },
  });
  const service = (
    id,
    typ = "LinkedDomains",
    serviceEndpoint = "https://example.invalid/δ",
  ) => ({ id, typ, serviceEndpoint });
  scenario("constructor-only", []);
  for (const fail of ["controller", "recovery", "timestamp"])
    scenario(`constructor-${fail}-failure`, [], { fail });
  scenario("constructor-equal-keys", [], { sameKeys: true });
  scenario("constructor-zero-time", [], { timestamp: "0" });
  scenario("constructor-max-time", [], { timestamp: "18446744073709551615" });
  scenario("aliases-services", [
    step("alias-insert", "setAlsoKnownAs", {
      value: "did:example:δ",
      mutation: 1,
    }),
    step(
      "alias-duplicate",
      "setAlsoKnownAs",
      { value: "did:example:δ", mutation: 1 },
      "already exists",
    ),
    step(
      "alias-undefined",
      "setAlsoKnownAs",
      { value: "", mutation: 0 },
      "mutation",
    ),
    step(
      "alias-missing-remove",
      "setAlsoKnownAs",
      { value: "missing", mutation: 2 },
      "does not exist",
    ),
    step("alias-remove", "setAlsoKnownAs", {
      value: "did:example:δ",
      mutation: 2,
    }),
    step("alias-empty", "setAlsoKnownAs", { value: "", mutation: 1 }),
    step(
      "service-missing-update",
      "setService",
      { service: service("svc"), mutation: 2 },
      "does not exist",
    ),
    step("service-insert", "setService", {
      service: service("svc"),
      mutation: 1,
    }),
    step(
      "service-duplicate",
      "setService",
      { service: service("svc"), mutation: 1 },
      "already exists",
    ),
    step(
      "service-undefined",
      "setService",
      { service: service("svc"), mutation: 0 },
      "mutation",
    ),
    step("service-update", "setService", {
      service: service("svc", "", ""),
      mutation: 2,
    }),
    step(
      "service-missing-remove",
      "removeService",
      { id: "missing" },
      "does not exist",
    ),
    step("service-remove", "removeService", { id: "svc" }),
    step(
      "late-timestamp-failure",
      "setAlsoKnownAs",
      { value: "must-rollback", mutation: 1 },
      "timestamp failure",
      { fail: "timestamp" },
    ),
    step("after-rollback-insert", "setAlsoKnownAs", {
      value: "must-rollback",
      mutation: 1,
    }),
  ]);
  const methods = [];
  for (const [label, crv, kty] of [
    ["ed", 0, 3],
    ["x", 1, 3],
    ["bls1", 5, 3],
    ["bls2", 6, 3],
    ["p256", 3, 0],
    ["secp", 4, 0],
  ])
    methods.push(
      step(`accept-${label}`, "setVerificationMethod", {
        method: vm(label, crv, kty),
        mutation: 1,
      }),
    );
  for (const [label, crv, kty, typ] of [
    ["type", 0, 3, 0],
    ["rsa", 0, 1, 1],
    ["oct", 0, 2, 1],
    ["jubjub", 2, 0, 1],
    ["ec-ed", 0, 0, 1],
    ["okp-p256", 3, 3, 1],
  ])
    methods.push(
      step(
        `reject-${label}`,
        "setVerificationMethod",
        { method: vm(label, crv, kty, typ), mutation: 1 },
        "",
      ),
    );
  methods.push(
    step(
      "missing-update",
      "setVerificationMethod",
      { method: vm("missing"), mutation: 2 },
      "does not exist",
    ),
    step(
      "undefined",
      "setVerificationMethod",
      { method: vm("u"), mutation: 0 },
      "mutation",
    ),
    step(
      "duplicate",
      "setVerificationMethod",
      { method: vm("ed"), mutation: 1 },
      "already exists",
    ),
    step(
      "missing-remove",
      "removeVerificationMethod",
      { id: "missing" },
      "does not exist",
    ),
  );
  for (const relation of [1, 2, 3, 4, 5]) {
    const id = relation === 3 ? "x" : "ed";
    methods.push(
      step(`relation-${relation}-insert`, "setVerificationMethodRelation", {
        relation,
        id,
        mutation: 1,
      }),
      step(
        `relation-${relation}-duplicate`,
        "setVerificationMethodRelation",
        { relation, id, mutation: 1 },
        "already exists",
      ),
      step(
        `relation-${relation}-blocks-removal`,
        "removeVerificationMethod",
        { id },
        "",
      ),
      step(
        `relation-${relation}-blocks-update`,
        "setVerificationMethod",
        { method: vm(id, relation === 3 ? 0 : 1), mutation: 2 },
        "",
      ),
      step(`relation-${relation}-remove`, "setVerificationMethodRelation", {
        relation,
        id,
        mutation: 2,
      }),
      step(
        `relation-${relation}-missing-remove`,
        "setVerificationMethodRelation",
        { relation, id, mutation: 2 },
        "does not exist",
      ),
    );
    if (relation !== 3)
      methods.push(
        step(
          `relation-${relation}-x25519-rejected`,
          "setVerificationMethodRelation",
          { relation, id: "x", mutation: 1 },
          "",
        ),
      );
  }
  methods.push(
    step(
      "key-agreement-ed-rejected",
      "setVerificationMethodRelation",
      { relation: 3, id: "ed", mutation: 1 },
      "",
    ),
    step(
      "undefined-relation",
      "setVerificationMethodRelation",
      { relation: 0, id: "ed", mutation: 1 },
      "defined",
    ),
    step(
      "undefined-relation-missing-target-first",
      "setVerificationMethodRelation",
      { relation: 0, id: "absent", mutation: 1 },
      "does not exist",
    ),
    step(
      "undefined-relation-mutation-first",
      "setVerificationMethodRelation",
      { relation: 0, id: "absent", mutation: 0 },
      "mutation",
    ),
    step("generic-update", "setVerificationMethod", {
      method: {
        ...vm("ed"),
        publicKeyJwk: { ...vm("ed").publicKeyJwk, x: "changed" },
      },
      mutation: 2,
    }),
    step("generic-remove", "removeVerificationMethod", { id: "ed" }),
  );
  methods.push(
    step("ordered-guards-method", "setVerificationMethod", {
      method: vm("ordered"),
      mutation: 1,
    }),
  );
  for (const relation of [1, 2, 4, 5])
    methods.push(
      step(`ordered-${relation}-insert`, "setVerificationMethodRelation", {
        relation,
        id: "ordered",
        mutation: 1,
      }),
    );
  const relationNames = {
    1: "authenticationRelation",
    2: "assertionMethodRelation",
    4: "capabilityInvocationRelation",
    5: "capabilityDelegationRelation",
  };
  for (const relation of [1, 2, 4, 5]) {
    methods.push(
      step(
        `ordered-${relation}-refusal`,
        "removeVerificationMethod",
        { id: "ordered" },
        relationNames[relation],
      ),
    );
    methods.push(
      step(`ordered-${relation}-remove`, "setVerificationMethodRelation", {
        relation,
        id: "ordered",
        mutation: 2,
      }),
    );
  }
  methods.push(
    step("ordered-final-remove", "removeVerificationMethod", { id: "ordered" }),
  );
  methods.push(
    step(
      "mutation-before-invalid-type",
      "setVerificationMethod",
      { method: vm("absent", 0, 3, 0), mutation: 0 },
      "Map mutation",
    ),
  );
  methods.push(
    step(
      "invalid-type-before-missing-update",
      "setVerificationMethod",
      { method: vm("absent", 0, 3, 0), mutation: 2 },
      "Only JsonWebKey",
    ),
  );
  scenario("methods-relations", methods);
  scenario("schnorr", [
    step(
      "missing-read",
      "verifySchnorrJubjubDigestSignature",
      { id: "s" },
      "does not exist",
    ),
    step(
      "missing-update",
      "setSchnorrJubjubVerificationMethod",
      { method: { id: "s", key: "5" }, mutation: 2 },
      "does not exist",
    ),
    step(
      "undefined",
      "setSchnorrJubjubVerificationMethod",
      { method: { id: "s", key: "5" }, mutation: 0 },
      "mutation",
    ),
    step("insert", "setSchnorrJubjubVerificationMethod", {
      method: { id: "s", key: "5" },
      mutation: 1,
    }),
    step(
      "duplicate",
      "setSchnorrJubjubVerificationMethod",
      { method: { id: "s", key: "5" }, mutation: 1 },
      "already exists",
    ),
    step(
      "cross-map-duplicate",
      "setVerificationMethod",
      { method: vm("s"), mutation: 1 },
      "already exists",
    ),
    step(
      "read-valid",
      "verifySchnorrJubjubDigestSignature",
      { id: "s" },
      undefined,
      { signer: "5" },
    ),
    step(
      "read-wrong-key",
      "verifySchnorrJubjubDigestSignature",
      { id: "s" },
      "",
      { signer: "1" },
    ),
    step("relation-insert", "setVerificationMethodRelation", {
      relation: 1,
      id: "s",
      mutation: 1,
    }),
    step(
      "referenced-removal",
      "removeSchnorrJubjubVerificationMethod",
      { id: "s" },
      "",
    ),
    step(
      "key-agreement-needs-generic",
      "setVerificationMethodRelation",
      { relation: 3, id: "s", mutation: 1 },
      "",
    ),
    step("update", "setSchnorrJubjubVerificationMethod", {
      method: { id: "s", key: "7" },
      mutation: 2,
    }),
    step(
      "read-updated",
      "verifySchnorrJubjubDigestSignature",
      { id: "s" },
      undefined,
      { signer: "7" },
    ),
    step("relation-remove", "setVerificationMethodRelation", {
      relation: 1,
      id: "s",
      mutation: 2,
    }),
    step("remove", "removeSchnorrJubjubVerificationMethod", { id: "s" }),
    step(
      "missing-remove",
      "removeSchnorrJubjubVerificationMethod",
      { id: "s" },
      "does not exist",
    ),
    step("generic-insert", "setVerificationMethod", {
      method: vm("s"),
      mutation: 1,
    }),
    step(
      "reverse-cross-map-duplicate",
      "setSchnorrJubjubVerificationMethod",
      { method: { id: "s", key: "5" }, mutation: 1 },
      "already exists",
    ),
  ]);
  scenario("authorization-lifecycle", [
    step(
      "stale-before-signature",
      "setAlsoKnownAs",
      { value: "x", mutation: 1 },
      "",
      { version: "1", badSignature: true },
    ),
    step("bad-signature", "setAlsoKnownAs", { value: "x", mutation: 1 }, "", {
      badSignature: true,
    }),
    step(
      "reduction-error",
      "setAlsoKnownAs",
      { value: "x", mutation: 1 },
      "reduction failure",
      { fail: "reduction" },
    ),
    step("same-controller", "rotateControllerKey", { key: "1" }, ""),
    step("controller-equals-recovery", "rotateControllerKey", { key: "3" }, ""),
    step("rotate", "rotateControllerKey", { key: "5" }),
    step(
      "old-controller-rejected",
      "setAlsoKnownAs",
      { value: "x", mutation: 1 },
      "",
      { signer: "1" },
    ),
    step("wrong-recovery-authority", "recoverControllerKey", { key: "7" }, "", {
      signer: "5",
    }),
    step("recovery-same-current", "recoverControllerKey", { key: "5" }, ""),
    step("recovery-equals-authority", "recoverControllerKey", { key: "3" }, ""),
    step("recover", "recoverControllerKey", { key: "7" }),
    step("deactivate", "deactivate"),
    step("deactivate-again", "deactivate", {}, "already inactive"),
    step(
      "inactive-write",
      "setAlsoKnownAs",
      { value: "x", mutation: 1 },
      "not active",
    ),
    step(
      "inactive-stale-first",
      "setAlsoKnownAs",
      { value: "x", mutation: 1 },
      "",
      { version: "0" },
    ),
    step(
      "inactive-invalid-signature-first",
      "setAlsoKnownAs",
      { value: "x", mutation: 1 },
      "",
      { badSignature: true },
    ),
    step(
      "inactive-read-before-missing",
      "verifySchnorrJubjubDigestSignature",
      { id: "absent" },
      "not active",
    ),
  ]);
}
