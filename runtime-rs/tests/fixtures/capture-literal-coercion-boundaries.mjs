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

// Compile the unchanged literal_coercion_oracle.compact with --skip-zk, link
// contract/node_modules/@midnight-ntwrk/compact-runtime to the isolated
// corrected runtime, then pass contract/index.js, runtime root, and source.
import { createHash } from 'node:crypto';
import { readFile, realpath } from 'node:fs/promises';
import { dirname, resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

const [contractPath, runtimeRoot, sourcePath] = process.argv.slice(2);
if (!contractPath || !runtimeRoot || !sourcePath) {
  throw new Error('expected contract/index.js, corrected runtime root, and Compact source');
}
const contractRuntime = resolve(dirname(contractPath), 'node_modules/@midnight-ntwrk/compact-runtime');
if (await realpath(contractRuntime) !== await realpath(runtimeRoot)) {
  throw new Error('generated contract is not linked to the selected corrected runtime');
}
const runtime = await import(pathToFileURL(resolve(runtimeRoot, 'dist/index.js')).href);
const { pureCircuits } = await import(pathToFileURL(resolve(contractPath)).href);
const runtimePackage = JSON.parse(await readFile(resolve(runtimeRoot, 'package.json'), 'utf8'));
const sha256 = async (path) => createHash('sha256').update(await readFile(path)).digest('hex');
const hex = (bytes) => Buffer.from(bytes).toString('hex');
const fieldHex = (value) => {
  const bytes = new Uint8Array(32);
  for (let index = 0; index < 32; index++) {
    bytes[index] = Number((value >> BigInt(8 * index)) & 255n);
  }
  return hex(bytes);
};
const point = ({ x, y }) => ({ x: fieldHex(x), y: fieldHex(y) });
const fieldVector = new runtime.CompactTypeVector(2, runtime.CompactTypeField);
const nestedFieldVector = new runtime.CompactTypeVector(1, fieldVector);
const byteVector = new runtime.CompactTypeVector(
  2,
  new runtime.CompactTypeUnsignedInteger(255n, 1),
);
const boundary = 1n << 248n;
const boundaryActual = pureCircuits.callArgMaxUnsignedPlusOne();
if (boundaryActual !== boundary) throw new Error('2^248 call-argument boundary differs');
const hashes = [0n, 255n].map((input) => {
  const flat = hex(pureCircuits.hashUintVarRefElem(input));
  const flatReference = hex(runtime.persistentHash(fieldVector, [input, 0n]));
  const nested = hex(pureCircuits.hashNestedUintVarRefElem(input));
  const nestedReference = hex(runtime.persistentHash(nestedFieldVector, [[input, 0n]]));
  const byteAligned = hex(runtime.persistentHash(byteVector, [input, 0n]));
  if (flat !== flatReference || nested !== nestedReference) {
    throw new Error(`generated hash differs from Field-aligned runtime reference at ${input}`);
  }
  if (flat === byteAligned || nested === byteAligned) {
    throw new Error(`Field-aligned hash matches Byte-aligned hash at ${input}`);
  }
  return { input: input.toString(), flat, flatReference, nested, nestedReference, byteAligned };
});
const subgroup = point(pureCircuits.subgroupCheck());
const generator = point(runtime.ecMulGenerator(1n));
if (JSON.stringify(subgroup) !== JSON.stringify(generator)) {
  throw new Error('subgroup scalar composition differs from the generator');
}

process.stdout.write(JSON.stringify({
  provenance: {
    sourceSha256: await sha256(sourcePath),
    generatedIndexSha256: await sha256(contractPath),
    runtimeVersion: runtimePackage.version,
    runtimeIndexSha256: await sha256(resolve(runtimeRoot, 'dist/index.js')),
    runtimeBuiltinsSha256: await sha256(resolve(runtimeRoot, 'dist/built-ins.js')),
    runtimeTypesSha256: await sha256(resolve(runtimeRoot, 'dist/compact-types.js')),
  },
  boundary: { decimal: boundaryActual.toString(), fieldHex: fieldHex(boundaryActual) },
  hashes,
  subgroup,
  generator,
}, null, 2) + '\n');
