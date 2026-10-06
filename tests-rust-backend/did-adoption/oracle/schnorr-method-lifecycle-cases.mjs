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

// Original DID constructor followed by real method insert, update, and remove calls.
export const captureScope = "Original SchnorrJubjub method Map lifecycle; recording and strict proof checked separately";
export function captureCases(scenario) {
  const method = (key) => ({ id: "did:method:日本語🗝️", key });
  const set = (id, value, mutation, error, o = {}) =>
    ({ id, name: "setSchnorrJubjubVerificationMethod", a: { method: value, mutation }, error, o });
  const remove = (id, methodId, error, o = {}) =>
    ({ id, name: "removeSchnorrJubjubVerificationMethod", a: { id: methodId }, error, o });
  scenario("schnorr-method-recording", [
    set("stale-before-signature", method("5"), 1, "version is stale", { version: "1", badSignature: true }),
    set("wrong-controller", method("5"), 1, "Invalid Jubjub Schnorr signature", { signer: "3" }),
    set("reduction-error", method("5"), 1, "reduction failure", { fail: "reduction" }),
    set("undefined", method("5"), 0, "mutation"),
    set("missing-update", method("5"), 2, "does not exist"),
    set("insert-unicode", method("5"), 1),
    set("duplicate", method("5"), 1, "already exists"),
    set("update-timestamp-rollback", method("7"), 2, "timestamp failure", { fail: "timestamp" }),
    set("update-point", method("7"), 2),
    remove("missing-remove", "missing", "does not exist"),
    remove("remove-unicode", method("5").id),
    remove("remove-again", method("5").id, "does not exist"),
    { id: "deactivate", name: "deactivate" },
    set("inactive", method("5"), 1, "Contract is not active"),
  ]);
}
