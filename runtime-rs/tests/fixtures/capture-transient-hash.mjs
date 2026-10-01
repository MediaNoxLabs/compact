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

// Compile transient_hash.compact with --skip-zk, link its generated contract
// to this branch's runtime, then pass contract/index.js as the argument.
import { pathToFileURL } from 'node:url';
import * as runtime from '../../../runtime/dist/index.js';

const [contractPath] = process.argv.slice(2);
if (!contractPath) throw new Error('expected contract/index.js');
const { pureCircuits } = await import(pathToFileURL(contractPath).href);
const pair = [3n, 5n];
const bytes = new Uint8Array([1, 2, 0, 0]);
const field = 42n;
const opening = 7n;
const pairType = new runtime.CompactTypeVector(2, runtime.CompactTypeField);

function hex(value) {
  return Array.from({ length: 32 }, (_, index) =>
    Number((value >> BigInt(index * 8)) & 255n).toString(16).padStart(2, '0')).join('');
}

const cases = {
  field: [pureCircuits.hash_field(field), runtime.transientHash(runtime.CompactTypeField, field)],
  commitField: [pureCircuits.commit_field(field, opening), runtime.transientCommit(runtime.CompactTypeField, field, opening)],
  pair: [pureCircuits.hash_pair(pair), runtime.transientHash(pairType, pair)],
  commitPair: [pureCircuits.commit_pair(pair, opening), runtime.transientCommit(pairType, pair, opening)],
  bytes: [pureCircuits.hash_bytes(bytes), runtime.transientHash(new runtime.CompactTypeBytes(4), bytes)],
};
for (const [name, [compiled, native]] of Object.entries(cases)) {
  if (compiled !== native) throw new Error(`${name}: generated TypeScript differs from runtime native`);
}
process.stdout.write(JSON.stringify(Object.fromEntries(
  Object.entries(cases).map(([name, [value]]) => [name, hex(value)]),
), null, 2) + '\n');
