// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//   http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

import { pathToFileURL } from 'node:url';
import { writeFileSync } from 'node:fs';

const { pureCircuits: pure } = await import(pathToFileURL(process.argv[2]).href);
const [profile, output] = process.argv.slice(3);
const b32 = seed => Uint8Array.from({ length: 32 }, (_, index) => (seed + index) & 255);
const zero = new Uint8Array(32);
const padded = text => { const bytes = new Uint8Array(32); bytes.set(Buffer.from(text)); return bytes; };
const method = { controllerAddress: { bytes: b32(1) }, methodId: b32(2) };
const otherController = { ...method, controllerAddress: { bytes: b32(3) } };
const otherMethod = { ...method, methodId: b32(4) };
const emptyMethod = { ...method, methodId: zero };
const binding = { holderVerificationMethodRef: method };
const proof = {
  signerVerificationMethodRef: method, createdAt: 100n, challengeHash: b32(55),
  publicKey: { x: 0n, y: 1n }, signature: { r: { x: 0n, y: 1n }, s: 0n },
};
const registry = { registryId: b32(20), authorityVerificationMethodRef: method };
const status = { registryRef: registry, statusHandleCommitment: b32(21) };
const schema = { packageId: padded('midnight:vc:digital-passport'), schemaId: padded('digital-passport:v1'), majorVersion: 1n, minorVersion: 0n };
const encode = value => value instanceof Uint8Array ? Buffer.from(value).toString('hex') : value === undefined ? [] : value;
const run = (name, evaluate) => {
  try { return { name, outcome: 'ok', value: encode(evaluate()) }; }
  catch (error) { return { name, outcome: 'error', message: String(error.message) }; }
};
const rows = [
  run('holder_valid', () => pure.assertValidExplicitHolderBinding(binding)),
  run('holder_missing_method', () => pure.assertValidExplicitHolderBinding({ holderVerificationMethodRef: emptyMethod })),
  run('holder_match', () => pure.assertMatchingExplicitHolderBindings(binding, binding)),
  run('holder_other_controller', () => pure.assertMatchingExplicitHolderBindings(binding, { holderVerificationMethodRef: otherController })),
  run('holder_other_method', () => pure.assertMatchingExplicitHolderBindings(binding, { holderVerificationMethodRef: otherMethod })),
  run('holder_proof_match', () => pure.assertProofMatchesExplicitHolderBinding(binding, proof)),
  run('holder_proof_other_controller', () => pure.assertProofMatchesExplicitHolderBinding(binding, { ...proof, signerVerificationMethodRef: otherController })),
  run('holder_proof_other_method', () => pure.assertProofMatchesExplicitHolderBinding(binding, { ...proof, signerVerificationMethodRef: otherMethod })),
  run('registry_valid', () => pure.assertValidStatusRegistryRef(registry)),
  run('registry_missing_id', () => pure.assertValidStatusRegistryRef({ ...registry, registryId: zero })),
  run('registry_missing_authority', () => pure.assertValidStatusRegistryRef({ ...registry, authorityVerificationMethodRef: emptyMethod })),
  run('status_valid', () => pure.assertValidRegistryBoundStatusBinding(status)),
  run('status_missing_handle', () => pure.assertValidRegistryBoundStatusBinding({ ...status, statusHandleCommitment: zero })),
  run('status_missing_registry', () => pure.assertValidRegistryBoundStatusBinding({ ...status, registryRef: { ...registry, registryId: zero } })),
  run('status_root_valid', () => pure.registryBoundStatusBindingRoot(status)),
  run('status_root_invalid', () => pure.registryBoundStatusBindingRoot({ ...status, statusHandleCommitment: zero })),
  run('issuer_scope_valid', () => pure.issuerScopeCommitment(schema)),
  run('issuer_scope_invalid', () => pure.issuerScopeCommitment({ ...schema, schemaId: zero })),
  run('verify_identity_key', () => pure.verifySignature(proof.publicKey, proof.signature, 5n)),
  run('issuance_proof_identity_key', () => pure.assertValidIssuanceContextProof(b32(33), proof)),
  run('presentation_proof_identity_key', () => pure.assertValidPresentationContextProof(b32(33), proof)),
  run('signer_proof_identity_key', () => pure.assertValidSignerAuthorizationContextProof(b32(33), proof)),
  run('verifier_proof_identity_key', () => pure.assertValidVerifierRequestContextProof(b32(33), proof)),
];
writeFileSync(output, `${JSON.stringify({ profile, rows }, null, 2)}\n`);
