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
// Direct unchanged Compact export calls for the opposite sampled pure branch.
import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { createRequire } from 'node:module';
import { pathToFileURL } from 'node:url';

const [contractPath, sourcePath, compilerPath, mode] = process.argv.slice(2);
if (!contractPath || !sourcePath || !compilerPath || !['chunked', 'struct'].includes(mode)) {
  throw new Error('usage: node capture-adr221-pure-branches.mjs <contract/index.js> <source.compact> <compactc-scheme> chunked|struct');
}
const contractIndex = resolve(contractPath);
const runtimeIndex = createRequire(contractIndex).resolve('@midnight-ntwrk/compact-runtime');
const runtimePackage = JSON.parse(readFileSync(join(dirname(runtimeIndex), '..', 'package.json')));
const { pureCircuits } = await import(pathToFileURL(contractIndex).href);
const hash = path => createHash('sha256').update(readFileSync(path)).digest('hex');
const value = mode === 'chunked' ? pureCircuits.ping(false) : pureCircuits.runWrapBeta(false);
process.stdout.write(JSON.stringify({
  format: 'adr221-direct-pure-opposite-branch-v1',
  mode,
  export: mode === 'chunked' ? 'ping' : 'runWrapBeta',
  argument: false,
  result: value,
  provenance: {
    sourceSha256: hash(sourcePath), generatedJsSha256: hash(contractIndex),
    runtimeVersion: runtimePackage.version, runtimeJsSha256: hash(runtimeIndex),
    compilerSha256: hash(compilerPath),
  },
}, null, 2) + '\n');
