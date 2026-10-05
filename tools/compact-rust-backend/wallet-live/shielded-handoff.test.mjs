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
import assert from 'node:assert/strict';
import test from 'node:test';
import { mkdtemp, readFile, rm, stat } from 'node:fs/promises';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import { confirmedShieldedObservation, assertShieldedOffersUnchanged, writePrivateHandoff } from './shielded-handoff.mjs';
const hash = n => n.repeat(64);
const action = () => ({ __typename: 'ContractCall', address: hash('a'), entryPoint: 'accept', state: '0102', zswapState: '0304', transaction: { hash: hash('b'), block: { height: 3, hash: hash('c') } } });
const expected = { type: 'ContractCall', address: hash('a'), transactionHash: hash('b'), entryPoint: 'accept' };
const rpc = async method => ({ chain_getFinalizedHead: hash('d'), chain_getHeader: { number: '0x4' }, chain_getBlockHash: hash('c') })[method];
const decoder = { deserialize: b => { if (b.length !== 2) throw new Error('malformed serialization'); } };
const ledger = { ContractState: decoder, ZswapChainState: decoder };

test('same-action pair retains explicit observation limits and exact identity', async () => {
  const result = await confirmedShieldedObservation(action(), expected, rpc, ledger);
  assert.equal(result.provenance.zswapScope, 'contract-specific-observation-not-full-ledger');
  assert.equal(result.provenance.transactionHash, hash('b'));
  assert.equal(result.stateBytes.toString('hex'), '0102');
  assert.equal(result.zswapBytes.toString('hex'), '0304');
  for (const change of [a => {a.entryPoint='release';}, a => {a.transaction.hash=hash('f');}, a => {a.zswapState='';}, a => {a.state='00';}, a => {a.transaction.block.hash=hash('f');}]) {
    const bad=action();change(bad);
    await assert.rejects(confirmedShieldedObservation(bad, expected, rpc, ledger));
  }
});
const offer = text => ({ serialize: () => Buffer.from(text) });
test('fee balancing cannot replace proofs, add funding/change or move shielded segments', () => {
  const original = { guaranteedOffer: offer('exact complete proven offer'), fallibleOffer: new Map() };
  assertShieldedOffersUnchanged(original, { ...original, unrelatedDust: true });
  for (const changed of [
    { guaranteedOffer: offer('changed proof or input/output') },
    { guaranteedOffer: undefined, fallibleOffer: new Map([[1, original.guaranteedOffer]]) },
    { ...original, fallibleOffer: new Map([[1, offer('extra')]]) },
  ]) assert.throws(() => assertShieldedOffersUnchanged(original, changed), /changed the retained/);
});
test('private handoff files are exclusive and do not expose bytes in receipt', async () => {
  const directory=await mkdtemp(join(tmpdir(),'compact-wallet-handoff-'));
  try {
    const file=join(directory,'input.bin');const bytes=Buffer.from('test witness');
    const receipt=await writePrivateHandoff(file,bytes);
    assert.deepEqual(Object.keys(receipt),['bytes','sha256']);
    assert.equal((await stat(file)).mode & 0o777,0o600);
    assert.deepEqual(await readFile(file),bytes);
    await assert.rejects(writePrivateHandoff(file,Buffer.from('replacement')),/EEXIST/);
  } finally { await rm(directory,{recursive:true,force:true}); }
});

test('wallet snapshot acquisition hash is checked before private-state decoding', async () => {
  const { decodeWalletSnapshot, sha256 } = await import('./shielded-handoff.mjs');
  const raw=Buffer.from('exact acquired snapshot');let decoded=0;
  const ledger={ZswapLocalState:{deserialize:bytes=>{decoded++;assert.deepEqual(bytes,raw);return {firstFree:3n};}}};
  const result=decodeWalletSnapshot(raw,sha256(raw),ledger);
  assert.equal(result.wallet.firstFree,3n);assert.equal(decoded,1);
  assert.throws(()=>decodeWalletSnapshot(Buffer.from('substituted snapshot'),sha256(raw),ledger),/acquisition SHA256 mismatch/);
  assert.throws(()=>decodeWalletSnapshot(raw,'bad hash',ledger),/acquisition SHA256 mismatch/);
  assert.equal(decoded,1);
});
