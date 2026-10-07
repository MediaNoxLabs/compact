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

// Compile witness_effect_order.compact with --target ts --skip-zk. Resolve the
// generated contract's runtime normally; COMPACTC and COMPACTC_SCHEME identify
// the compiler binaries used for capture provenance. Pass contract/index.js.
import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import { createRequire } from "node:module";
import { fileURLToPath, pathToFileURL } from "node:url";

const [contractPath] = process.argv.slice(2);
if (!contractPath) throw new Error("expected absolute contract/index.js path");
const require = createRequire(contractPath);
const runtimePath = require.resolve("@midnight-ntwrk/compact-runtime");
const runtime = await import(pathToFileURL(runtimePath));
const { Contract } = await import(pathToFileURL(contractPath));
const sourcePath = fileURLToPath(
  new URL(
    "../../../examples/rust_backend/witness_effect_order.compact",
    import.meta.url,
  ),
);
const sha256 = (path) =>
  createHash("sha256").update(readFileSync(path)).digest("hex");
const normalize = (_, value) =>
  value instanceof Uint8Array
    ? Array.from(value)
    : typeof value === "bigint"
      ? value.toString()
      : value instanceof Map
        ? Object.fromEntries(value)
        : value;
const originalQuery = runtime.QueryContext.prototype.query;
let events = [];
let queries = [];
runtime.QueryContext.prototype.query = function (...args) {
  events.push("query");
  const query = { ops: args[0] };
  queries.push(query);
  try {
    const result = originalQuery.call(this, ...args);
    query.gas = result.gasCost;
    return result;
  } catch (error) {
    query.error = String(error);
    throw error;
  }
};

function run(name, choice, zeroGas = false) {
  const calls = { gate: 0, key: 0 };
  function answer(witness, { privateState }) {
    calls[witness] += 1;
    events.push(witness);
    assert.equal(privateState, 7);
    if (choice === "error") throw new Error("intentional witness refusal");
    return [privateState + 1, witness === "gate" ? choice : BigInt(choice)];
  }
  const contract = new Contract({
    gate: (context) => answer("gate", context),
    key: (context) => answer("key", context),
  });
  const coinPublicKey = { bytes: new Uint8Array(32) };
  const initial = contract.initialState({
    initialPrivateState: 7,
    initialZswapLocalState: runtime.emptyZswapLocalState(coinPublicKey),
  });
  const before = Buffer.from(initial.currentContractState.serialize()).toString(
    "hex",
  );
  const context = runtime.createCircuitContext(
    runtime.dummyContractAddress(),
    coinPublicKey,
    initial.currentContractState.data,
    initial.currentPrivateState,
  );
  if (zeroGas) context.gasLimit = runtime.emptyRunningCost();
  events = [];
  queries = [];
  const row = { name, choice, zeroGas, before };
  try {
    const out = contract.circuits[name](
      context,
      ...(name === "equal_result" ? [42n] : []),
    );
    const after = runtime.ContractState.deserialize(
      initial.currentContractState.serialize(),
    );
    after.data = new runtime.ChargedState(
      out.context.currentQueryContext.state.state,
    );
    Object.assign(row, {
      result: out.result,
      after: Buffer.from(after.serialize()).toString("hex"),
      effects: out.context.currentQueryContext.effects,
      privateState: out.context.currentPrivateState,
      privateOutputs: out.proofData.privateTranscriptOutputs,
      output: out.proofData.output,
      gas: out.gasCost,
      publicTranscript: out.proofData.publicTranscript,
    });
  } catch (error) {
    row.error = String(error);
  }
  // Copy the event arrays; initialization of the next case also queries the ledger.
  return { ...row, calls, events: [...events], queries: [...queries] };
}

try {
  const rows = [];
  for (const name of [
    "equal_result",
    "set_member",
    "map_member",
    "map_lookup",
  ]) {
    for (const choice of name === "equal_result"
      ? [false, true, "error"]
      : [7, 9, "error"]) {
      rows.push(run(name, choice));
    }
  }
  for (const name of ["set_member", "map_member", "map_lookup"]) {
    for (const choice of [7, "error"]) rows.push(run(name, choice, true));
  }
  assert.equal(rows.length, 18);
  for (const row of rows) {
    const witness = row.name === "equal_result" ? "gate" : "key";
    assert.deepEqual(
      row.calls,
      witness === "gate" ? { gate: 1, key: 0 } : { gate: 0, key: 1 },
    );
    const queried = witness === "key" && row.choice !== "error";
    assert.deepEqual(row.events, queried ? [witness, "query"] : [witness]);
    assert.equal(row.queries.length, queried ? 1 : 0);
    if (row.choice === "error")
      assert.equal(row.error, "Error: intentional witness refusal");
    else if (row.zeroGas)
      assert.equal(row.error, "CompactError: Error: ran out of gas budget");
    else if (row.name === "map_lookup" && row.choice === 9)
      assert.equal(
        row.error,
        "CompactError: Error: expected a cell, received null",
      );
    else {
      assert.equal(row.error, undefined);
      assert.equal(
        row.result,
        row.name === "equal_result" || row.name === "map_lookup"
          ? 42n
          : row.choice === 7,
      );
      assert.equal(row.privateState, 8);
      assert.equal(row.privateOutputs.length, 1);
      assert.equal(row.before, row.after);
    }
  }
  const capture = {
    provenance: {
      source: "examples/rust_backend/witness_effect_order.compact",
      sourceSha256: sha256(sourcePath),
      generatedSha256: sha256(contractPath),
      runtimeSha256: sha256(runtimePath),
      compilerSha256: sha256(process.env.COMPACTC),
      schemeSha256: sha256(process.env.COMPACTC_SCHEME),
      nodeVersion: process.version,
    },
    scope:
      "Native execution oracle; query programs are TS observations, not Rust recording or proof evidence.",
    rows,
  };
  process.stdout.write(JSON.stringify(capture, normalize, 2) + "\n");
} finally {
  runtime.QueryContext.prototype.query = originalQuery;
}
