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
// Run against a generated shielded_receive_oracle TS bundle whose local
// @midnight-ntwrk/compact-runtime resolves to the isolated runtime under test.
// The original ADR195 refusal capture must remain unchanged.
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { pathToFileURL } from 'node:url';
import { readFileSync } from 'node:fs';
import { createHash } from 'node:crypto';
const [contractPath] = process.argv.slice(2);
if (!contractPath) throw new Error('expected generated receive contract path');
const require = createRequire(contractPath);
const runtimePath = require.resolve('@midnight-ntwrk/compact-runtime');
const r = await import(pathToFileURL(runtimePath));
const { Contract } = await import(pathToFileURL(contractPath));
const max128 = (1n << 128n) - 1n;
const values = [0n, 42n, (1n << 64n) - 1n, 1n << 64n, max128];
const nonce = new Uint8Array(32).fill(3),
  color = new Uint8Array(32).fill(4);
const key = { bytes: new Uint8Array(32).fill(7) },
  address = { bytes: new Uint8Array(32).fill(8) };
const recipient = (is_left) => ({
  is_left,
  left: is_left ? key : { bytes: new Uint8Array(32) },
  right: is_left ? { bytes: new Uint8Array(32) } : address,
});
const encode = (_, x) =>
  x instanceof Uint8Array ? Array.from(x) : typeof x === 'bigint' ? x.toString() : x instanceof Map ? Object.fromEntries(x) : x;
const rows = [];
for (const name of ['accept', 'accept_renamed']) {
  for (const value of [...values, -1n, max128 + 1n]) {
    const contract = new Contract({});
    const init = contract.initialState({ initialPrivateState: [], initialZswapLocalState: r.emptyZswapLocalState(key) });
    const context = r.createCircuitContext(r.decodeContractAddress(address.bytes), key, init.currentContractState.data, []);
    context.currentZswapLocalState.currentIndex = 13n;
    const coin = { nonce, color, value };
    if (value < 0n || value > max128) {
      assert.throws(() => contract.circuits[name](context, coin));
      assert.equal(context.currentZswapLocalState.currentIndex, 13n);
      assert.equal(context.currentZswapLocalState.outputs.length, 0);
      rows.push({ name, value, status: 'rejected', outputs: 0, index: 13n });
      continue;
    }
    const out = contract.circuits[name](context, coin);
    assert.deepEqual(out.result, []);
    assert.equal(out.context.currentZswapLocalState.currentIndex, 14n);
    assert.equal(out.context.currentZswapLocalState.outputs.length, 1);
    assert.deepEqual(out.context.currentZswapLocalState.outputs[0], { coinInfo: coin, recipient: recipient(false) });
    const contractHash = Buffer.from(contract._coinCommitment_0(coin, recipient(false))).toString('hex');
    assert.equal(out.context.currentQueryContext.comIndices.get(contractHash), 13n);
    const commitments = [];
    for (const isLeft of [false, true]) {
      const pure = Buffer.from(contract._coinCommitment_0(coin, recipient(isLeft))).toString('hex');
      const direct = r.createCircuitContext(r.decodeContractAddress(address.bytes), key, init.currentContractState.data, []);
      r.createZswapOutput(direct, coin, recipient(isLeft));
      assert.equal(direct.currentQueryContext.comIndices.get(pure), 0n);
      commitments.push({ recipient: isLeft ? 'user' : 'contract', commitment: pure });
    }
    rows.push({
      name,
      value,
      status: 'accepted',
      index: out.context.currentZswapLocalState.currentIndex,
      outputs: out.context.currentZswapLocalState.outputs.map((output) => ({
        value: output.coinInfo.value,
        isLeft: output.recipient.is_left,
      })),
      commitments,
      publicTranscriptOperationCount: out.proofData.publicTranscript.length,
      privateOutputs: out.proofData.privateTranscriptOutputs,
      effects: out.context.currentQueryContext.effects,
      gas: out.gasCost,
    });
  }
}
const hash = (path) => createHash('sha256').update(readFileSync(path)).digest('hex');
process.stdout.write(
  JSON.stringify(
    {
      scope: 'Corrected TS runtime generated receive/output evidence; no funded ledger or proof acceptance claim',
      contractPath,
      contractSha256: hash(contractPath),
      runtimePath,
      runtimeIndexSha256: hash(runtimePath),
      rows,
    },
    encode,
    2,
  ) + '\n',
);
