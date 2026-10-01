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

// Capture the asset-registry nested provenance subtraction with TypeScript.
import { createRequire } from 'node:module';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

if (process.argv.length !== 3) throw new Error('usage: node asset_registry_pure_capture.mjs <compiled-contract-dir>');
const contractIndex = resolve(process.argv[2], 'index.js');
const requireFromContract = createRequire(contractIndex);
await import(pathToFileURL(requireFromContract.resolve('@midnight-ntwrk/compact-runtime')));
const { pureCircuits } = await import(pathToFileURL(contractIndex));
function record(registeredAt) {
  return {
    code: new Uint8Array(32), note: '',
    provenance: { facility: new Uint8Array(32), registeredAt },
    kind: 1, quantity: 0n,
  };
}
let reverseError;
try { pureCircuits.registrationGap(record(100n), record(120n)); }
catch (error) { reverseError = error.message; }
process.stdout.write(JSON.stringify({
  gap: pureCircuits.registrationGap(record(120n), record(100n)).toString(),
  reverseError,
}, null, 2) + '\n');
