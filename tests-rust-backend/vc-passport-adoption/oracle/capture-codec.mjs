// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//  http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

import assert from 'node:assert/strict';
import { readFileSync, writeFileSync, mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { stripTypeScriptTypes } from 'node:module';
import { pathToFileURL } from 'node:url';

// Execute the exact pinned upstream descriptors, changing only module URLs after
// type stripping. The original .ts bytes are separately pinned by Git blob SHA.
const generated = pathToFileURL(process.argv[2]);
const runtimeUrl = new URL('./node_modules/@midnight-ntwrk/compact-runtime/dist/index.js', generated).href;
const [profile, output] = process.argv.slice(3);
const source = new URL('./upstream-codec-source/', import.meta.url);
const stage = mkdtempSync(join(tmpdir(), 'compact-vc-codec-'));
let codecs;
try {
  const transport = stripTypeScriptTypes(readFileSync(new URL('compact-value-codec.ts', source), 'utf8'), { mode: 'transform' });
  const descriptor = stripTypeScriptTypes(readFileSync(new URL('codecs.ts', source), 'utf8'), { mode: 'transform' })
    .replaceAll("'@midnight-ntwrk/compact-runtime'", `'${runtimeUrl}'`)
    .replace("'./internal/compact-value-codec.js'", "'./compact-value-codec.mjs'");
  writeFileSync(join(stage, 'compact-value-codec.mjs'), transport);
  writeFileSync(join(stage, 'codecs.mjs'), descriptor);
  codecs = await import(pathToFileURL(join(stage, 'codecs.mjs')).href);
} finally {
  // Imported module bodies remain in memory after this temporary staging.
  rmSync(stage, { recursive: true, force: true });
}
const { ecMulGenerator } = await import(runtimeUrl);
const bytes = seed => Uint8Array.from({ length: 32 }, (_, i) => (seed + i) & 255);
const method = seed => ({ controllerAddress: { bytes: bytes(seed) }, methodId: bytes(seed + 1) });
const credential = {
  version: 1n,
  schema: { packageId: bytes(1), schemaId: bytes(2), majorVersion: 1n, minorVersion: 3n },
  issuerVerificationMethodRef: method(3),
  holderBinding: { holderVerificationMethodRef: method(5) },
  statusBinding: {}, issuedAt: (1n << 40n) + 9n,
  hasExpiration: true, expiresAt: (1n << 40n) + 365n,
  claims: {},
  claimCommitments: {
    firstNameCommitment: bytes(11), lastNameCommitment: bytes(12),
    dateOfBirthCommitment: bytes(13), documentNumberCommitment: bytes(14),
    issuingStateCommitment: bytes(15),
  },
  claimRoot: bytes(16),
};
const proof = {
  signerVerificationMethodRef: method(3), createdAt: (1n << 40n) + 10n,
  challengeHash: bytes(21), publicKey: ecMulGenerator(3n),
  signature: { r: ecMulGenerator(7n), s: 13n },
};
const minimalCredential = {
  ...credential, issuedAt: 0n, hasExpiration: false, expiresAt: 0n,
  claimCommitments: { ...credential.claimCommitments, documentNumberCommitment: new Uint8Array(32) },
  claimRoot: new Uint8Array(32),
};
const boundaryProof = {
  ...proof, createdAt: 0n,
  signature: { r: ecMulGenerator(1n), s: 0n },
};
const hex = value => Buffer.from(value).toString('hex');
const capture = (descriptor, encode, decode, value) => {
  const chunks = descriptor.toValue(value);
  assert.deepEqual(descriptor.fromValue(chunks.map(chunk => new Uint8Array(chunk))), value);
  const encoded = encode(value);
  assert.equal(encoded.encoding, 'compact-value-v1.base64url');
  assert.deepEqual(decode(encoded), value);
  return {
    alignment: descriptor.alignment(),
    chunks: chunks.map(hex),
    framedHex: hex(Buffer.from(encoded.payload, 'base64url')),
  };
};
const vectors = {
  credential: capture(codecs.digitalPassportCredentialDescriptor, codecs.encodeDigitalPassportCredential,
    codecs.decodeDigitalPassportCredential, credential),
  proof: capture(codecs.digitalPassportProofDescriptor, codecs.encodeDigitalPassportProof,
    codecs.decodeDigitalPassportProof, proof),
  credentialMinimal: capture(codecs.digitalPassportCredentialDescriptor, codecs.encodeDigitalPassportCredential,
    codecs.decodeDigitalPassportCredential, minimalCredential),
  proofBoundary: capture(codecs.digitalPassportProofDescriptor, codecs.encodeDigitalPassportProof,
    codecs.decodeDigitalPassportProof, boundaryProof),
};
writeFileSync(output, `${JSON.stringify({ profile, vectors }, null, 2)}\n`);
