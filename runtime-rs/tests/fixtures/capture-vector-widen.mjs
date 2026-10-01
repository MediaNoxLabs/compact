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

// Compile vector_widen.compact with --skip-zk, link generated contract to
// this branch's runtime, then pass contract/index.js as argument.
import { pathToFileURL } from 'node:url';
const [contractPath] = process.argv.slice(2);
if (!contractPath) throw new Error('expected contract/index.js');
const { pureCircuits } = await import(pathToFileURL(contractPath).href);
const values = [7n, 255n];
process.stdout.write(JSON.stringify({
  widened: pureCircuits.widen(values).map((value) => value.toString()),
  elements: pureCircuits.widen_elements(4294967295n).map((value) => value.toString()),
  hashHex: Buffer.from(pureCircuits.hash_widened(values)).toString('hex'),
  nestedHashHex: Buffer.from(pureCircuits.hash_nested(values)).toString('hex'),
}, null, 2) + '\n');
