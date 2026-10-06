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

// Unchanged DID source calls with an original-constructor prestate.
export const captureScope = "Original Service Map native TS lifecycle; recording and strict proof are checked separately";
export function captureCases(scenario) {
  const service = (id, typ = "LinkedDomains", serviceEndpoint = "https://example.invalid/δ") =>
    ({ id, typ, serviceEndpoint });
  const set = (id, serviceValue, mutation, error, o = {}) =>
    ({ id, name: "setService", a: { service: serviceValue, mutation }, error, o });
  const remove = (id, serviceId, error, o = {}) =>
    ({ id, name: "removeService", a: { id: serviceId }, error, o });
  const initial = service("svc:日本語🗝️");
  const updated = service(initial.id, "", "");
  scenario("service-recording", [
    set("stale-before-signature", initial, 1, "version is stale", { version: "1", badSignature: true }),
    set("wrong-controller", initial, 1, "Invalid Jubjub Schnorr signature", { signer: "3" }),
    set("reduction-error", initial, 1, "reduction failure", { fail: "reduction" }),
    set("undefined", initial, 0, "mutation"),
    set("missing-update", initial, 2, "does not exist"),
    set("insert-unicode", initial, 1),
    set("duplicate", initial, 1, "already exists"),
    set("update-timestamp-rollback", updated, 2, "timestamp failure", { fail: "timestamp" }),
    set("update-empty-fields", updated, 2),
    remove("missing-remove", "missing", "does not exist"),
    remove("remove-unicode", initial.id),
    remove("remove-again", initial.id, "does not exist"),
    { id: "deactivate", name: "deactivate" },
    set("inactive", initial, 1, "Contract is not active"),
  ]);
}
