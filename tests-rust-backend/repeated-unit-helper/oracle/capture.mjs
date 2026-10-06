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

// Usage: node capture.mjs /absolute/generated/contract/index.js
import { pathToFileURL } from "node:url";
import { readFileSync, realpathSync } from "node:fs";
import { resolve, dirname } from "node:path";
import { createHash } from "node:crypto";
import * as r from "../../../runtime/dist/index.js";
const modulePath = resolve(process.argv[2]);
if (
  realpathSync(
    resolve(
      dirname(modulePath),
      "node_modules/@midnight-ntwrk/compact-runtime",
    ),
  ) !== realpathSync("runtime")
)
  throw Error("capture must use this checkout's built TypeScript runtime");
const m = await import(pathToFileURL(modulePath).href);
const json = (v) =>
  JSON.parse(
    JSON.stringify(v, (_, x) =>
      typeof x === "bigint"
        ? String(x)
        : x instanceof Uint8Array
          ? Array.from(x)
          : x,
    ),
  );
const hex = (s) => Buffer.from(s.serialize()).toString("hex");
const key = { bytes: new Uint8Array(32) };
let queries = [];
const original = r.QueryContext.prototype.query;
r.QueryContext.prototype.query = function (...args) {
  const output = original.call(this, ...args);
  queries.push(json({ gasCost: output.gasCost }));
  return output;
};
function witness(name, { privateState, ledger }, argument) {
  const count = ledger.count,
    last = ledger.last,
    receipt = ledger.receipt;
  const ordinal = BigInt(privateState.length);
  const result =
    name === "argument"
      ? argument + 100n * count + ordinal
      : argument + 1000n * count + ordinal;
  return [
    [...privateState, { name, argument, count, last, receipt, result }],
    result,
  ];
}
const contract = new m.Contract({
  argument: (ctx, value) => witness("argument", ctx, value),
  observe: (ctx, value) => witness("observe", ctx, value),
});
const cases = [];
for (const [id, first, second] of [
  ["distinct", 3n, 7n],
  ["reversed", 7n, 3n],
  ["same-label-zero", 0n, 0n],
]) {
  const initial = contract.initialState({
    initialPrivateState: [],
    initialZswapLocalState: r.emptyZswapLocalState(key),
  });
  const state = initial.currentContractState;
  const before = hex(state);
  queries = [];
  const output = contract.circuits.twice(
    r.createCircuitContext(r.dummyContractAddress(), key, state.data, []),
    first,
    second,
  );
  const savedQueries = json(queries);
  state.data = new r.ChargedState(
    output.context.currentQueryContext.state.state,
  );
  cases.push(
    json({
      id,
      first,
      second,
      before,
      after: hex(state),
      result: output.result,
      privateAfter: output.context.currentPrivateState,
      queries: savedQueries,
      reportedGas: output.gasCost,
      publicTranscript: output.proofData.publicTranscript,
      privateTranscript: output.proofData.privateTranscriptOutputs.map((v) => ({
        valueAtoms: v.value.map((a) => Array.from(a)),
        alignment: v.alignment,
      })),
    }),
  );
}
const provenance = [
  ["source", "examples/rust_backend/repeated_unit_helper.compact"],
  ["capture", "tests-rust-backend/repeated-unit-helper/oracle/capture.mjs"],
  ["generated_typescript", modulePath],
  ["runtime_builtins", "runtime/dist/built-ins.js"],
  ["runtime_types", "runtime/dist/compact-types.js"],
].map(([role, path]) => ({
  role,
  path: path === modulePath ? "generated/contract/index.js" : path,
  sha256: createHash("sha256").update(readFileSync(path)).digest("hex"),
}));
process.stdout.write(
  JSON.stringify(
    {
      scope:
        "Branch TypeScript compiler/runtime independent execution capture; no proof or network claim",
      compiler: "0.31.133",
      runtime: "0.16.101",
      provenance,
      cases,
    },
    null,
    2,
  ) + "\n",
);
