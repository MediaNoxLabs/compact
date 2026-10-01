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

// Compile jubjub_arithmetic.compact with --skip-zk, link its generated
// contract to this branch's runtime, then pass contract/index.js.
import { pathToFileURL } from 'node:url';
import * as runtime from '../../../runtime/dist/index.js';

const [contractPath] = process.argv.slice(2);
if (!contractPath) throw new Error('expected contract/index.js');
const { pureCircuits } = await import(pathToFileURL(contractPath).href);
function hex(value) {
  return Array.from({ length: 32 }, (_, index) =>
    Number((value >> BigInt(index * 8)) & 255n).toString(16).padStart(2, '0')).join('');
}
function coords(point) { return { x: hex(point.x), y: hex(point.y) }; }
function checkPoint(name, compiled, native) {
  const actual = coords(compiled);
  if (JSON.stringify(actual) !== JSON.stringify(coords(native))) {
    throw new Error(`${name}: generated TypeScript differs from native`);
  }
  return actual;
}
const left = runtime.hashToCurve(runtime.CompactTypeField, 3n);
const right = runtime.hashToCurve(runtime.CompactTypeField, 5n);
const add = checkPoint('add', pureCircuits.add_points(3n, 5n), runtime.ecAdd(left, right));
const negate = checkPoint('negate', pureCircuits.negate_point(3n), runtime.ecNeg(left));
const multiply = checkPoint('multiply', pureCircuits.multiply_point(3n, 7n), runtime.ecMul(left, 7n));
const generator = checkPoint('generator', pureCircuits.generator_point(7n), runtime.ecMulGenerator(7n));
const reduced = pureCircuits.reduce_scalar(runtime.MAX_FIELD);
if (reduced !== runtime.reduceModJubjubOrder(runtime.MAX_FIELD)) throw new Error('scalar reduction differs');
const generatorReduced = checkPoint('generatorReduced', pureCircuits.generator_reduced(runtime.MAX_FIELD), runtime.ecMulGenerator(reduced));
let highScalarRejected = false;
try { pureCircuits.generator_point(runtime.MAX_FIELD); } catch { highScalarRejected = true; }
if (!highScalarRejected) throw new Error('noncanonical Jubjub scalar was accepted');
let highMultiplyRejected = false;
try { pureCircuits.multiply_point(3n, runtime.MAX_FIELD); } catch { highMultiplyRejected = true; }
if (!highMultiplyRejected) throw new Error('noncanonical point multiplier was accepted');
process.stdout.write(JSON.stringify({
  add, negate, multiply, generator, generatorReduced,
  reduced: hex(reduced),
  highScalar: hex(runtime.MAX_FIELD),
  highScalarRejected,
  highMultiplyRejected,
}, null, 2) + '\n');
