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

const generated = new URL(pathToFileURL(process.argv[2]).href);
const bundle = await import(generated.href);
const pure = bundle.pureCircuits;
const { ecMulGenerator } = await import(new URL('./node_modules/@midnight-ntwrk/compact-runtime/dist/index.js', generated).href);
const [profile, output] = process.argv.slice(3);
const b32 = seed => Uint8Array.from({ length: 32 }, (_, index) => (seed + index) & 255);
const zero = new Uint8Array(32);
const padded = text => { const bytes = new Uint8Array(32); bytes.set(Buffer.from(text)); return bytes; };
const schema = { packageId: padded('midnight:vc:digital-passport'), schemaId: padded('digital-passport:v1'), majorVersion: 1n, minorVersion: 0n };
const method = { controllerAddress: { bytes: b32(1) }, methodId: b32(2) };
const descriptor = {
  version: 1n, authorizationId: b32(40), decisionSequence: 1n,
  state: bundle.AuthorizationState.active, role: bundle.SignerRole.issuer,
  signerVerificationMethodRef: method, signerPublicKey: ecMulGenerator(3n),
  didStateVersion: 1n, verificationRelationship: bundle.VerificationRelationship.assertionMethod,
  scopeCommitment: pure.issuerScopeCommitment(schema), policyCommitment: b32(41),
};
const proof = {
  signerVerificationMethodRef: method, createdAt: 101n, challengeHash: b32(55),
  publicKey: ecMulGenerator(3n), signature: { r: ecMulGenerator(7n), s: 0n },
};
const authority = { domainCommitment: b32(42), verificationMethodRef: method, publicKey: ecMulGenerator(5n) };
const subgroupOrder = 6554484396890773809930967563523245729705921265872317281365359162392183254199n;
const sign = (root, challengeFn, secret, nonce, createdAt) => {
  const unsigned = { signerVerificationMethodRef: method, createdAt, challengeHash: b32(55),
    publicKey: ecMulGenerator(secret), signature: { r: ecMulGenerator(nonce), s: 0n } };
  const challenge = challengeFn(root, unsigned);
  return { ...unsigned, signature: { ...unsigned.signature, s: (nonce + challenge * secret) % subgroupOrder } };
};
const authProof = sign(pure.signerAuthorizationDecisionRoot(descriptor, authority.domainCommitment), pure.signerAuthorizationProofChallenge, 5n, 13n, 1n);
const verifierDescriptor = { ...descriptor, role: bundle.SignerRole.verifier,
  verificationRelationship: bundle.VerificationRelationship.authentication, scopeCommitment: b32(60) };
const verifierProof = sign(b32(60), pure.verifierRequestProofChallenge, 3n, 17n, 101n);
const encode = value => value instanceof Uint8Array ? Buffer.from(value).toString('hex') : value === undefined ? [] : value;
const run = (name, evaluate) => {
  try { return { name, outcome: 'ok', value: encode(evaluate()) }; }
  catch (error) { return { name, outcome: 'error', message: String(error.message) }; }
};
const rows = [
  run('auth_descriptor_valid', () => pure.assertValidAuthorizedSignerDescriptor(descriptor)),
  run('auth_descriptor_root', () => pure.authorizedSignerDescriptorRoot(descriptor)),
  run('auth_descriptor_bad_version', () => pure.assertValidAuthorizedSignerDescriptor({ ...descriptor, version: 2n })),
  run('auth_descriptor_identity_key', () => pure.assertValidAuthorizedSignerDescriptor({ ...descriptor, signerPublicKey: { x: 0n, y: 1n } })),
  run('auth_descriptor_wrong_relation', () => pure.assertValidAuthorizedSignerDescriptor({ ...descriptor, verificationRelationship: bundle.VerificationRelationship.authentication })),
  run('auth_scope_commitment', () => pure.issuerScopeCommitment(schema)),
  run('auth_proof_matches', () => pure.assertProofSignerMatchesAuthorization(proof, descriptor)),
  run('auth_proof_wrong_key', () => pure.assertProofSignerMatchesAuthorization({ ...proof, publicKey: ecMulGenerator(4n) }, descriptor)),
  run('auth_proof_suspended', () => pure.assertProofSignerMatchesAuthorization(proof, { ...descriptor, state: bundle.AuthorizationState.suspended })),
  run('auth_issuer_valid', () => pure.assertAuthorizedIssuerDescriptor(schema, proof, descriptor)),
  run('auth_issuer_wrong_scope', () => pure.assertAuthorizedIssuerDescriptor({ ...schema, schemaId: b32(33) }, proof, descriptor)),
  run('auth_issuer_wrong_role', () => pure.assertAuthorizedIssuerDescriptor(schema, proof, { ...descriptor, role: bundle.SignerRole.verifier, verificationRelationship: bundle.VerificationRelationship.authentication })),
  run('auth_authority_valid', () => pure.assertValidSignerAuthorizationAuthority(authority)),
  run('auth_authority_zero_domain', () => pure.assertValidSignerAuthorizationAuthority({ ...authority, domainCommitment: zero })),
  run('auth_decision_root', () => pure.signerAuthorizationDecisionRoot(descriptor, b32(42))),
  run('auth_decision_zero_domain', () => pure.signerAuthorizationDecisionRoot(descriptor, zero)),
  run('auth_verifier_valid', () => pure.assertAuthorizedVerifierProof(b32(60), verifierProof, verifierDescriptor)),
  run('auth_verifier_wrong_scope', () => pure.assertAuthorizedVerifierProof(b32(61), verifierProof, verifierDescriptor)),
  run('auth_verifier_wrong_role', () => pure.assertAuthorizedVerifierProof(b32(60), verifierProof, descriptor)),
  run('auth_proof_valid', () => pure.assertValidSignerAuthorizationProof(descriptor, authProof, authority)),
  run('auth_proof_wrong_sequence', () => pure.assertValidSignerAuthorizationProof(descriptor, { ...authProof, createdAt: 2n }, authority)),
  run('auth_proof_wrong_authority', () => pure.assertValidSignerAuthorizationProof(descriptor, authProof, { ...authority, verificationMethodRef: { ...method, methodId: b32(62) } })),
  run('auth_update_valid', () => pure.assertValidSignerAuthorizationUpdate(descriptor, { ...descriptor, decisionSequence: 2n })),
  run('auth_update_same_sequence', () => pure.assertValidSignerAuthorizationUpdate(descriptor, descriptor)),
  run('auth_update_revoked_reactivated', () => pure.assertValidSignerAuthorizationUpdate({ ...descriptor, state: bundle.AuthorizationState.revoked }, { ...descriptor, decisionSequence: 2n })),
];
const vectors = { authorizationScalar: authProof.signature.s.toString(16), verifierScalar: verifierProof.signature.s.toString(16) };
writeFileSync(output, `${JSON.stringify({ profile, vectors, rows }, null, 2)}\n`);
