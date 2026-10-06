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

// Extra independent source observations for the new Point recording domain.
export function captureCases(scenario) {
  const step = (id, name, key, error, o = {}) => ({ id, name, a: { key }, error, o });
  scenario("point-failures", [
    step("stale-before-signature", "rotateControllerKey", "5", "version is stale", {version:"1", badSignature:true}),
    step("wrong-controller", "rotateControllerKey", "5", "Invalid Jubjub Schnorr signature", {signer:"3"}),
    step("reduction-error", "rotateControllerKey", "5", "reduction failure", {fail:"reduction"}),
    step("late-timestamp", "rotateControllerKey", "5", "timestamp failure", {fail:"timestamp"}),
    step("rotate", "rotateControllerKey", "5"),
    step("old-controller", "rotateControllerKey", "7", "Invalid Jubjub Schnorr signature", {signer:"1"}),
    step("stale-recovery", "recoverControllerKey", "7", "version is stale", {version:"0",badSignature:true}),
    step("recover-timestamp", "recoverControllerKey", "7", "timestamp failure", {fail:"timestamp"}),
    step("recover", "recoverControllerKey", "7"),
    {id:"deactivate",name:"deactivate"},
    step("inactive-stale-first", "rotateControllerKey", "5", "version is stale", {version:"0",badSignature:true}),
    step("inactive-valid-signature", "rotateControllerKey", "5", "Contract is not active"),
  ]);
}
