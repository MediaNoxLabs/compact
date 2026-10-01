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

// Compile jubjub_hash.compact with --skip-zk, link its generated contract
// to this branch's runtime, then pass contract/index.js as the argument.
import { pathToFileURL } from 'node:url';
import * as runtime from '../../../runtime/dist/index.js';

const [contractPath] = process.argv.slice(2);
if (!contractPath) throw new Error('expected contract/index.js');
const { pureCircuits } = await import(pathToFileURL(contractPath).href);
const field = 42n;
const pair = [3n, 5n];
const pairType = new runtime.CompactTypeVector(2, runtime.CompactTypeField);
function hex(value) {
  return Array.from({ length: 32 }, (_, index) =>
    Number((value >> BigInt(index * 8)) & 255n).toString(16).padStart(2, '0')).join('');
}
function coords(point) { return { x: hex(point.x), y: hex(point.y) }; }
function check(name, compiled, native) {
  const actual = coords(compiled);
  const expected = coords(native);
  if (JSON.stringify(actual) !== JSON.stringify(expected)) throw new Error(`${name}: generated TypeScript differs from native`);
  return actual;
}
const point = check('point', pureCircuits.point(field), runtime.hashToCurve(runtime.CompactTypeField, field));
const pairPoint = check('pairPoint', pureCircuits.pair_point(pair), runtime.hashToCurve(pairType, pair));
const x = hex(pureCircuits.point_x(field));
const y = hex(pureCircuits.point_y(field));
if (x !== point.x || y !== point.y) throw new Error('coordinate natives differ from point');
process.stdout.write(JSON.stringify({ point, pairPoint, x, y }, null, 2) + '\n');
