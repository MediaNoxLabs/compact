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

// Independent original-source Set cycle, including selected-path failure order.
export const captureScope = "Native constructor-driven opaque-string Set source calls; recording and proof evidence are separate";
export function captureCases(scenario) {
  const step = (id, value, mutation, error, o = {}) => ({
    id, name: "setAlsoKnownAs", a: { value, mutation }, error, o,
  });
  const alias = "did:example:δ/日本語🗝️";
  scenario("alias-recording", [
    step("stale-before-signature", alias, 1, "version is stale", { version: "1", badSignature: true }),
    step("wrong-controller", alias, 1, "Invalid Jubjub Schnorr signature", { signer: "3" }),
    step("reduction-error", alias, 1, "reduction failure", { fail: "reduction" }),
    step("undefined", alias, 0, "mutation"),
    step("missing-remove", alias, 2, "does not exist"),
    step("insert-unicode", alias, 1),
    step("duplicate", alias, 1, "already exists"),
    step("remove-timestamp-rollback", alias, 2, "timestamp failure", { fail: "timestamp" }),
    step("remove-unicode", alias, 2),
    step("insert-empty", "", 1),
    step("remove-empty", "", 2),
    { id: "deactivate", name: "deactivate" },
    step("inactive", alias, 1, "Contract is not active"),
    step("inactive-stale", alias, 1, "version is stale", { version: "0", badSignature: true }),
  ]);
}
