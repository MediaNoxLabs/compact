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

// Capture ledger-8 TypeScript MerkleTreePathRoot outputs for witness paths.
import { readFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

if (process.argv.length !== 4) throw new Error('usage: node merkle_path_root_capture.mjs <compiled-contract-dir> <path-oracle-json>');
const contractIndex = resolve(process.argv[2], 'index.js');
const requireFromContract = createRequire(contractIndex);
await import(pathToFileURL(requireFromContract.resolve('@midnight-ntwrk/compact-runtime')));
const { pureCircuits } = await import(pathToFileURL(contractIndex));
const paths = JSON.parse(readFileSync(process.argv[3], 'utf8'));
function root(key) {
  const data = paths[key];
  const path = {
    leaf: BigInt(data.leaf),
    path: data.path.map(entry => ({
      sibling: { field: BigInt(entry.sibling) },
      goes_left: entry.goesLeft,
    })),
  };
  return pureCircuits.root_of(path).field.toString();
}
process.stdout.write(JSON.stringify({
  rootFor7At0: root('pathFor7At0'),
  rootFor9At3: root('pathFor9At3'),
}, null, 2) + '\n');
